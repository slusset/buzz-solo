//! Journey: `specs/journeys/evolve-a-principal-node.md`, step 6.
//! Story: `specs/stories/principal-node/place-synchronization-inside-principal-node.md`.
//! Feature: `specs/features/principal-node/sync-session.feature`.
//! Contract: `specs/contracts/principal-node/sync-session-ports.yaml`.

use buzz_core::replication::{ReplicationIngestOutcome, ReplicationReceipt, ReplicationRecord};
use buzz_principal_node::{
    AuthenticatedTransportEvidence, CandidateDigest, ClockSample, CommitProof,
    CompletedCommitCandidate, CompletedCommitDisposition, ContinuityCandidate, CurrentSyncPlan,
    EvaluationFailureClassification, EventId, EvidenceRef, InvalidTransition,
    InvalidTransitionReason, LifecycleError, OpaqueSourceCursorToken, PrincipalNodeId,
    ReceiptEvidence, ReceiptOutcomeClass, SourceBoundCursor, SourceCursorProgress, SourcePage,
    SourcePageError, SourceStreamId, SyncDirection, SyncLifecycle, SyncRequest, SyncSessionId,
    SyncState, SyncTransition, SyncTrigger, TerminalCommitCandidate, TerminalCommitDisposition,
    TerminalSyncSessionSummary,
};
use serde::Deserialize;
use serde_json::Value;

const VALID_TRANSITIONS: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-valid-transitions-v0.1.json");
const INVALID_TRANSITIONS: &str = include_str!(
    "../../../specs/fixtures/principal-node/sync-session-invalid-transitions-v0.1.json"
);
const TRIGGERS: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-triggers-v0.1.json");
const EMPTY: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-empty-batch-v0.1.json");
const SCAN: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-scan-progress-v0.1.json");
const SUCCESS: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-success-v0.1.json");
const RECEIPTS: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-receipts-v0.1.json");
const EVALUATION_FAILURES: &str = include_str!(
    "../../../specs/fixtures/principal-node/sync-session-evaluation-failures-v0.1.json"
);
const PENDING_CONTINUITY: &str = include_str!(
    "../../../specs/fixtures/principal-node/sync-session-pending-continuity-v0.1.json"
);

#[test]
fn all_nine_capability_fixtures_deserialize_into_typed_contract_values() {
    let triggers: TriggerFixture = serde_json::from_str(TRIGGERS).expect("typed trigger fixture");
    assert_eq!(triggers.cases.len(), 6);
    assert_eq!(triggers.base_request.direction, SyncDirection::Pull);
    assert!(triggers
        .cases
        .iter()
        .any(|case| case.trigger == SyncTrigger::Retry && case.previous_session_id.is_some()));

    let success: SuccessFixture = serde_json::from_str(SUCCESS).expect("typed success fixture");
    let success_wire: Value = serde_json::from_str(SUCCESS).expect("success JSON");
    success.request.validate().expect("typed request validates");
    success
        .source_page
        .validate(
            &success.request.source_stream_id,
            Some(&success.cursor_before),
            success.current_plan.batch_limit,
        )
        .expect("typed success page validates");
    assert_eq!(success.receipts.len(), 1);
    assert!(success.receipts[0].checkpoint_safe());
    success
        .expected_summary
        .validate()
        .expect("typed expected summary validates");
    assert_eq!(
        success.transport_evidence.direction,
        success.request.direction
    );
    assert_eq!(
        serde_json::to_value(&success.current_plan).expect("current plan serializes"),
        success_wire["current_plan"]
    );

    let empty: EmptyFixture = serde_json::from_str(EMPTY).expect("typed empty fixture");
    empty
        .source_page
        .validate(
            &empty.request.source_stream_id,
            Some(&empty.cursor_before),
            64,
        )
        .expect("typed empty page validates");

    let scan: ScanFixture = serde_json::from_str(SCAN).expect("typed scan fixture");
    assert_eq!(scan.cases.len(), 2);
    for case in &scan.cases {
        case.source_page
            .validate(&scan.source_stream_id, Some(&scan.cursor_before), 64)
            .expect("typed scan page validates");
    }
    assert!(scan
        .malformed_case
        .source_page
        .validate(&scan.source_stream_id, Some(&scan.cursor_before), 64)
        .is_err());

    let receipts: ReceiptFixture = serde_json::from_str(RECEIPTS).expect("typed receipt fixture");
    assert_eq!(receipts.record_binding.event_id.as_str().len(), 64);
    assert_eq!(receipts.cases.len(), 6);
    assert!(receipts
        .cases
        .iter()
        .any(|case| case.outcome == ReceiptOutcomeClass::Ambiguous));

    let failures: EvaluationFailureFixture =
        serde_json::from_str(EVALUATION_FAILURES).expect("typed evaluation fixture");
    assert_eq!(failures.prior_state, SyncState::Evaluating);
    assert_eq!(failures.transition, SyncTransition::EvaluationFailure);
    assert_eq!(failures.terminal_state, SyncState::Failed);
    assert_eq!(failures.cases.len(), 5);
    assert!(failures.cases.iter().all(|case| {
        !case.evidence_ref.as_str().is_empty()
            && matches!(
                case.classification,
                EvaluationFailureClassification::CursorLoadFailed
                    | EvaluationFailureClassification::TransportUnavailable
                    | EvaluationFailureClassification::ProjectionUnavailable
                    | EvaluationFailureClassification::SourceUnavailable
                    | EvaluationFailureClassification::MalformedSourceBatch
            )
    }));

    let pending: PendingFixture =
        serde_json::from_str(PENDING_CONTINUITY).expect("typed pending fixture");
    assert_eq!(pending.cases.len(), 2);
    for case in pending.cases {
        let (candidate, digest) = case.into_candidate_and_digest();
        candidate.validate().expect("typed candidate validates");
        assert_eq!(
            candidate
                .canonical_digest()
                .expect("candidate digest computes"),
            digest
        );
    }
    let pending_wire: Value = serde_json::from_str(PENDING_CONTINUITY).expect("pending JSON");
    for case in pending_wire["cases"].as_array().expect("pending cases") {
        let mut exact_wire = case.clone();
        exact_wire
            .as_object_mut()
            .expect("pending case object")
            .remove("accepted_dispositions");
        let pending: buzz_principal_node::PendingContinuityCommit =
            serde_json::from_value(exact_wire.clone())
                .expect("pending outer wire shape deserializes");
        pending.validate().expect("pending wire validates");
        assert!(pending.verify_digest().expect("pending digest verifies"));
        assert_eq!(
            serde_json::to_value(pending).expect("pending outer wire serializes"),
            exact_wire
        );
        let mut unknown_outer = exact_wire;
        unknown_outer
            .as_object_mut()
            .expect("pending object")
            .insert("unexpected".to_string(), Value::Bool(true));
        assert!(
            serde_json::from_value::<buzz_principal_node::PendingContinuityCommit>(unknown_outer)
                .is_err()
        );
    }

    let valid: ValidFixture =
        serde_json::from_str(VALID_TRANSITIONS).expect("typed valid transitions");
    assert_eq!(valid.transitions.len(), 13);
    assert!(valid
        .transitions
        .iter()
        .all(|vector| !vector.from.values().is_empty()));

    let invalid: InvalidFixture =
        serde_json::from_str(INVALID_TRANSITIONS).expect("typed invalid transitions");
    assert_eq!(invalid.cases.len(), 12);
    assert!(invalid
        .cases
        .iter()
        .all(|case| case.transition.reason() == case.reason));
}

#[test]
fn fixture_covers_and_executes_all_thirteen_valid_transitions() {
    let fixture: Value = serde_json::from_str(VALID_TRANSITIONS).expect("valid fixture JSON");
    let transitions = fixture["transitions"].as_array().expect("transition array");
    assert_eq!(transitions.len(), 13);

    for vector in transitions {
        let transition: SyncTransition =
            serde_json::from_value(vector["name"].clone()).expect("known transition");
        let from_value = vector["from"]
            .as_array()
            .and_then(|values| values.first())
            .cloned()
            .unwrap_or_else(|| vector["from"].clone());
        let from: SyncState = serde_json::from_value(from_value).expect("known source state");
        let to: SyncState =
            serde_json::from_value(vector["to"].clone()).expect("known target state");
        let proof = match vector["durability"].as_str().expect("durability") {
            "no_terminal_persistence" => CommitProof::InMemory,
            "cursor_and_completed_summary_atomic" => CommitProof::AtomicCompletionDurable,
            _ => CommitProof::TerminalSummaryDurable,
        };
        let mut lifecycle = lifecycle_in(from);
        assert_eq!(
            lifecycle
                .apply(transition, proof)
                .expect("allowed transition"),
            to
        );
    }
}

#[test]
fn fixture_covers_and_rejects_all_twelve_invalid_transitions() {
    let fixture: Value = serde_json::from_str(INVALID_TRANSITIONS).expect("valid fixture JSON");
    let cases = fixture["cases"].as_array().expect("case array");
    assert_eq!(cases.len(), 12);

    for vector in cases {
        let transition: InvalidTransition =
            serde_json::from_value(vector["transition"].clone()).expect("known invalid transition");
        let expected_reason: InvalidTransitionReason =
            serde_json::from_value(vector["reason"].clone()).expect("known invalid reason");
        let lifecycle = SyncLifecycle::requested();
        let state = lifecycle.state();
        assert!(matches!(
            lifecycle.reject(transition),
            LifecycleError::Forbidden { reason, .. } if reason == expected_reason
        ));
        assert_eq!(lifecycle.state(), state);
    }
}

#[test]
fn cancellation_transition_refuses_every_checkpoint_unsafe_state() {
    for unsafe_state in [
        SyncState::Transferring,
        SyncState::AwaitingDurableReceipts,
        SyncState::CommittingCursor,
    ] {
        let mut lifecycle = lifecycle_in(unsafe_state);
        let error = lifecycle
            .apply(
                SyncTransition::CancelAtSafeBoundary,
                CommitProof::TerminalSummaryDurable,
            )
            .expect_err("unsafe boundary must reject cancellation");
        assert!(matches!(error, LifecycleError::InvalidSource { .. }));
        assert_eq!(lifecycle.state(), unsafe_state);
    }
}

#[test]
fn trigger_fixture_deserializes_only_authority_free_requests() {
    let fixture: Value = serde_json::from_str(TRIGGERS).expect("valid fixture JSON");
    let base = &fixture["base_request"];
    let cases = fixture["cases"].as_array().expect("trigger cases");
    assert_eq!(cases.len(), 6);

    for case in cases {
        let request = serde_json::json!({
            "principal_node_id": base["principal_node_id"],
            "direction": base["direction"],
            "source_stream_id": base["source_stream_id"],
            "trigger": case["trigger"],
            "previous_session_id": case["previous_session_id"],
        });
        let request: SyncRequest = serde_json::from_value(request).expect("valid request");
        request.validate().expect("fixture request validates");
    }

    let untrusted = serde_json::json!({
        "principal_node_id": "principal-node:home",
        "direction": "pull",
        "source_stream_id": "shared/clinical-review-v1",
        "trigger": "startup",
        "agreement": "trigger-supplied"
    });
    assert!(serde_json::from_value::<SyncRequest>(untrusted).is_err());
}

#[test]
fn empty_and_scan_fixtures_enforce_exact_cursor_progress() {
    let empty_fixture: Value = serde_json::from_str(EMPTY).expect("valid empty fixture");
    let empty: SourcePage = serde_json::from_value(empty_fixture["source_page"].clone())
        .expect("source page deserializes");
    let cursor: SourceBoundCursor =
        serde_json::from_value(empty_fixture["cursor_before"].clone()).expect("cursor");
    let stream = SourceStreamId::new("shared/clinical-review-v1").expect("stream");
    empty
        .validate(&stream, Some(&cursor), 64)
        .expect("unchanged caught-up page is valid");

    let scan_fixture: Value = serde_json::from_str(SCAN).expect("valid scan fixture");
    let before: SourceBoundCursor =
        serde_json::from_value(scan_fixture["cursor_before"].clone()).expect("cursor");
    for case in scan_fixture["cases"].as_array().expect("scan cases") {
        let page: SourcePage =
            serde_json::from_value(case["source_page"].clone()).expect("scan page");
        page.validate(&stream, Some(&before), 64)
            .expect("advanced empty page is valid");
        assert_eq!(page.cursor_progress, SourceCursorProgress::Advanced);
    }

    let malformed: SourcePage =
        serde_json::from_value(scan_fixture["malformed_case"]["source_page"].clone())
            .expect("malformed page shape still deserializes");
    assert_eq!(
        malformed.validate(&stream, Some(&before), 64),
        Err(SourcePageError::StalledEmptyPage)
    );
}

#[test]
fn source_page_rejects_empty_opaque_tokens_before_continuity() {
    let empty_fixture: Value = serde_json::from_str(EMPTY).expect("valid empty fixture");
    let mut page: SourcePage = serde_json::from_value(empty_fixture["source_page"].clone())
        .expect("source page deserializes");
    page.batch.next_cursor = buzz_core::replication::ReplicationCursor::new("");
    let stream = SourceStreamId::new("shared/clinical-review-v1").expect("stream");
    assert_eq!(
        page.validate(&stream, None, 64),
        Err(SourcePageError::EmptyCursorToken)
    );

    let success: Value = serde_json::from_str(SUCCESS).expect("valid success fixture");
    let mut page: SourcePage =
        serde_json::from_value(success["source_page"].clone()).expect("success page");
    page.batch.records[0].cursor = buzz_core::replication::ReplicationCursor::new("");
    assert_eq!(
        page.validate(&stream, None, 64),
        Err(SourcePageError::EmptyRecordCursorToken)
    );
}

#[test]
fn receipt_fixture_matches_checkpoint_safe_policy_and_exact_binding() {
    let success: Value = serde_json::from_str(SUCCESS).expect("valid success fixture");
    let flat_receipt: ReceiptEvidence =
        serde_json::from_value(success["receipts"][0].clone()).expect("flat receipt evidence");
    assert_eq!(
        serde_json::to_value(flat_receipt).expect("flat receipt serializes"),
        success["receipts"][0]
    );
    let record: ReplicationRecord =
        serde_json::from_value(success["source_page"]["batch"]["records"][0].clone())
            .expect("record");
    let fixture: Value = serde_json::from_str(RECEIPTS).expect("valid receipt fixture");

    for vector in fixture["cases"].as_array().expect("receipt cases") {
        let outcome = match vector["outcome"].as_str().expect("outcome") {
            "stored" => ReplicationIngestOutcome::Stored,
            "duplicate" => ReplicationIngestOutcome::Duplicate,
            "superseded" => ReplicationIngestOutcome::Superseded,
            "rejected" => ReplicationIngestOutcome::Rejected {
                reason: "fixture rejection".to_string(),
            },
            other => {
                assert_eq!(other, "ambiguous");
                let evidence = ReceiptEvidence::ambiguous(
                    &record,
                    EvidenceRef::new("receipt:ambiguous").expect("evidence"),
                )
                .expect("valid event ID");
                assert_eq!(
                    evidence.checkpoint_safe(),
                    vector["checkpoint_safe"].as_bool().expect("safe")
                );
                continue;
            }
        };
        let receipt = ReplicationReceipt {
            source: record.source.clone(),
            cursor: record.cursor.clone(),
            event_id: record.event.id.to_hex(),
            outcome,
        };
        let evidence = ReceiptEvidence::from_receipt(
            receipt,
            vector["durable"].as_bool().expect("durable"),
            EvidenceRef::new("receipt:digest").expect("evidence"),
        )
        .expect("valid event ID");
        assert!(evidence.validate_against(&record));
        assert_eq!(
            evidence.checkpoint_safe(),
            vector["checkpoint_safe"].as_bool().expect("safe")
        );
    }
}

#[test]
fn identifier_types_are_compiler_distinct_and_nonempty() {
    assert!(PrincipalNodeId::new("").is_err());
    assert!(SourceStreamId::new("").is_err());
    assert!(OpaqueSourceCursorToken::new("").is_err());
    assert!(EventId::new("00").is_err());
    assert!(EventId::new("z".repeat(64)).is_err());
    assert!(ReceiptEvidence::from_receipt(
        ReplicationReceipt {
            source: buzz_core::replication::ReplicationSourceId::new("source"),
            cursor: buzz_core::replication::ReplicationCursor::new("cursor"),
            event_id: "not-an-event-id".to_string(),
            outcome: ReplicationIngestOutcome::Stored,
        },
        true,
        EvidenceRef::new("receipt:invalid-id").expect("evidence"),
    )
    .is_err());
    let request = SyncRequest::new(
        PrincipalNodeId::new("principal-node:home").expect("principal"),
        SyncDirection::Push,
        SourceStreamId::new("shared/clinical-review-v1").expect("stream"),
        SyncTrigger::JournalCommit,
        None,
    )
    .expect("request");
    assert_eq!(request.direction, SyncDirection::Push);
    assert_eq!(
        ClockSample::new("2026-08-04T00:00:00Z", 1).monotonic_tick,
        1
    );
}

#[derive(Deserialize)]
struct TriggerFixture {
    base_request: TriggerBaseRequest,
    cases: Vec<TriggerCase>,
}

#[derive(Deserialize)]
struct TriggerBaseRequest {
    direction: SyncDirection,
}

#[derive(Deserialize)]
struct TriggerCase {
    trigger: SyncTrigger,
    previous_session_id: Option<SyncSessionId>,
}

#[derive(Deserialize)]
struct SuccessFixture {
    request: SyncRequest,
    transport_evidence: AuthenticatedTransportEvidence,
    current_plan: CurrentSyncPlan,
    cursor_before: SourceBoundCursor,
    source_page: SourcePage,
    receipts: Vec<ReceiptEvidence>,
    expected_summary: TerminalSyncSessionSummary,
}

#[derive(Deserialize)]
struct EmptyFixture {
    request: SyncRequest,
    cursor_before: SourceBoundCursor,
    source_page: SourcePage,
}

#[derive(Deserialize)]
struct ScanFixture {
    source_stream_id: SourceStreamId,
    cursor_before: SourceBoundCursor,
    cases: Vec<ScanCase>,
    malformed_case: ScanCase,
}

#[derive(Deserialize)]
struct ScanCase {
    source_page: SourcePage,
}

#[derive(Deserialize)]
struct ReceiptFixture {
    record_binding: ReceiptBinding,
    cases: Vec<ReceiptCase>,
}

#[derive(Deserialize)]
struct ReceiptBinding {
    event_id: EventId,
}

#[derive(Deserialize)]
struct ReceiptCase {
    outcome: ReceiptOutcomeClass,
}

#[derive(Deserialize)]
struct EvaluationFailureFixture {
    prior_state: SyncState,
    transition: SyncTransition,
    terminal_state: SyncState,
    cases: Vec<EvaluationFailureCase>,
}

#[derive(Deserialize)]
struct EvaluationFailureCase {
    classification: EvaluationFailureClassification,
    evidence_ref: EvidenceRef,
}

#[derive(Deserialize)]
struct PendingFixture {
    cases: Vec<PendingCase>,
}

#[derive(Deserialize)]
#[serde(tag = "candidate_kind", rename_all = "snake_case")]
enum PendingCase {
    TerminalSummary {
        principal_node_id: PrincipalNodeId,
        session_id: SyncSessionId,
        prior_lifecycle_state: SyncState,
        candidate_digest: CandidateDigest,
        exact_candidate: TerminalCommitCandidate,
        #[serde(rename = "accepted_dispositions")]
        _accepted_dispositions: Vec<TerminalCommitDisposition>,
    },
    CompletedAtomic {
        principal_node_id: PrincipalNodeId,
        session_id: SyncSessionId,
        prior_lifecycle_state: SyncState,
        candidate_digest: CandidateDigest,
        exact_candidate: CompletedCommitCandidate,
        #[serde(rename = "accepted_dispositions")]
        _accepted_dispositions: Vec<CompletedCommitDisposition>,
    },
}

impl PendingCase {
    fn into_candidate_and_digest(self) -> (ContinuityCandidate, CandidateDigest) {
        let (principal_node_id, session_id, prior_lifecycle_state, digest, candidate) = match self {
            Self::TerminalSummary {
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                candidate_digest,
                exact_candidate,
                ..
            } => (
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                candidate_digest,
                ContinuityCandidate::TerminalSummary(exact_candidate),
            ),
            Self::CompletedAtomic {
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                candidate_digest,
                exact_candidate,
                ..
            } => (
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                candidate_digest,
                ContinuityCandidate::CompletedAtomic(exact_candidate),
            ),
        };
        assert_eq!(candidate.principal_node_id(), &principal_node_id);
        assert_eq!(candidate.session_id(), &session_id);
        assert!(!prior_lifecycle_state.is_terminal());
        (candidate, digest)
    }
}

#[derive(Deserialize)]
struct ValidFixture {
    transitions: Vec<ValidTransitionCase>,
}

#[derive(Deserialize)]
struct ValidTransitionCase {
    #[serde(rename = "name")]
    _name: SyncTransition,
    from: OneOrMany<SyncState>,
    #[serde(rename = "to")]
    _to: SyncState,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    fn values(&self) -> Vec<&T> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values.iter().collect(),
        }
    }
}

#[derive(Deserialize)]
struct InvalidFixture {
    cases: Vec<InvalidTransitionCase>,
}

#[derive(Deserialize)]
struct InvalidTransitionCase {
    transition: InvalidTransition,
    reason: InvalidTransitionReason,
}

fn lifecycle_in(state: SyncState) -> SyncLifecycle {
    let mut lifecycle = SyncLifecycle::requested();
    if state == SyncState::Requested {
        return lifecycle;
    }
    lifecycle
        .apply(SyncTransition::BeginEvaluation, CommitProof::InMemory)
        .expect("enter evaluating");
    match state {
        SyncState::Evaluating => lifecycle,
        SyncState::Transferring => {
            lifecycle
                .apply(SyncTransition::BeginTransfer, CommitProof::InMemory)
                .expect("enter transferring");
            lifecycle
        }
        SyncState::AwaitingDurableReceipts => {
            lifecycle
                .apply(SyncTransition::BeginTransfer, CommitProof::InMemory)
                .expect("enter transferring");
            lifecycle
                .apply(SyncTransition::BatchDelivered, CommitProof::InMemory)
                .expect("enter receipt classification");
            lifecycle
        }
        SyncState::CommittingCursor => {
            lifecycle
                .apply(SyncTransition::BeginCursorCommit, CommitProof::InMemory)
                .expect("enter cursor commit");
            lifecycle
        }
        terminal => panic!("fixture source state unexpectedly terminal: {terminal:?}"),
    }
}
