//! Journey: `specs/journeys/evolve-a-principal-node.md`, step 6.
//! Story: `specs/stories/principal-node/place-synchronization-inside-principal-node.md`.
//! Feature: `specs/features/principal-node/sync-session.feature`.
//! Contract: `specs/contracts/principal-node/sync-session-ports.yaml`.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use buzz_core::replication::{ReplicationIngestOutcome, ReplicationReceipt, ReplicationRecord};
use buzz_principal_node::ports::{
    PeerAuthenticationRequest, ProjectionRequest, SinkIngestRequest, SourcePageRequest,
};
use buzz_principal_node::{
    AttemptClock, AuthenticatedPeerTransport, AuthenticatedTransportEvidence,
    BlockedClassification, BlockedSyncPlan, ClockSample, CompletedCommitCandidate,
    CompletedCommitDisposition, CompletedCommitResult, ContinuityError, CurrentSyncPlan,
    CurrentSyncProjection, EvaluationFailure, EvaluationFailureClassification, EvidenceRef,
    PlanEvaluation, PrincipalNodeSyncContinuity, PrincipalNodeSyncService, ReceiptEvidence,
    ReplicationSink, ReplicationSource, SourceBoundCursor, SourcePage, SyncDirection,
    SyncOutcomeClassification, SyncProcedureResult, SyncRequest, SyncServiceError, SyncSessionId,
    SyncSessionIdentityIssuer, SyncState, SyncTrigger, TerminalCommitCandidate,
    TerminalCommitDisposition, TerminalSyncSessionSummary, TransferFailure,
    TransportAuthentication,
};
use serde::Deserialize;
use serde_json::Value;

const SUCCESS: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-success-v0.1.json");
const SCAN: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-scan-progress-v0.1.json");
const EMPTY: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-empty-batch-v0.1.json");
const TRIGGERS: &str =
    include_str!("../../../specs/fixtures/principal-node/sync-session-triggers-v0.1.json");
const EVALUATION_FAILURES: &str = include_str!(
    "../../../specs/fixtures/principal-node/sync-session-evaluation-failures-v0.1.json"
);

type Service = PrincipalNodeSyncService<
    MockIdentityIssuer,
    MockClock,
    MockTransport,
    MockProjection,
    MockSource,
    MockSink,
    MockContinuity,
>;

#[tokio::test]
async fn success_fixture_drives_exact_envelope_and_direction_keyed_atomic_commit() {
    let harness = Harness::success();
    let expected_record = harness.record.clone();
    let expected_summary: TerminalSyncSessionSummary =
        serde_json::from_value(harness.fixture["expected_summary"].clone())
            .expect("expected summary deserializes");

    let result = harness
        .service
        .request_sync(harness.request.clone())
        .await
        .expect("sync succeeds");
    let summary = terminal(result);
    assert_eq!(summary, expected_summary);
    assert_eq!(harness.source_state.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        harness
            .sink_state
            .records
            .lock()
            .expect("records")
            .as_slice(),
        &[expected_record]
    );
    let expected_transport: AuthenticatedTransportEvidence =
        serde_json::from_value(harness.fixture["transport_evidence"].clone())
            .expect("transport evidence");
    assert_eq!(
        harness
            .source_state
            .transport_evidence
            .lock()
            .expect("source transport evidence")
            .as_slice(),
        std::slice::from_ref(&expected_transport)
    );
    assert_eq!(
        harness
            .sink_state
            .transport_evidence
            .lock()
            .expect("sink transport evidence")
            .as_slice(),
        &[expected_transport]
    );

    let loads = harness
        .continuity_state
        .cursor_loads
        .lock()
        .expect("cursor loads");
    assert_eq!(loads.len(), 1);
    assert_eq!(loads[0].2, SyncDirection::Pull);
    drop(loads);

    let commits = harness
        .continuity_state
        .completed_candidates
        .lock()
        .expect("completed candidates");
    assert_eq!(commits.len(), 1);
    let commit = &commits[0];
    commit
        .validate()
        .expect("captured atomic candidate validates");
    assert_eq!(commit.direction(), SyncDirection::Pull);
    assert_eq!(commit.summary(), &summary);
    assert_eq!(commit.receipts().len(), 1);
}

#[tokio::test]
async fn push_uses_the_same_exact_envelope_procedure_with_push_keyed_ports() {
    let harness = Harness::success_direction(SyncDirection::Push);
    let expected_record = harness.record.clone();
    let summary = terminal(
        harness
            .service
            .request_sync(harness.request.clone())
            .await
            .expect("push succeeds"),
    );

    assert_eq!(summary.direction, SyncDirection::Push);
    assert_eq!(
        harness
            .source_state
            .directions
            .lock()
            .expect("source directions")
            .as_slice(),
        &[SyncDirection::Push]
    );
    assert_eq!(
        harness
            .sink_state
            .directions
            .lock()
            .expect("sink directions")
            .as_slice(),
        &[SyncDirection::Push]
    );
    assert_eq!(
        harness
            .sink_state
            .records
            .lock()
            .expect("records")
            .as_slice(),
        &[expected_record]
    );
    let source_transport = harness
        .source_state
        .transport_evidence
        .lock()
        .expect("source transport evidence");
    let sink_transport = harness
        .sink_state
        .transport_evidence
        .lock()
        .expect("sink transport evidence");
    assert_eq!(source_transport[0].direction, SyncDirection::Push);
    assert_eq!(sink_transport[0], source_transport[0]);
    let loads = harness
        .continuity_state
        .cursor_loads
        .lock()
        .expect("cursor loads");
    assert_eq!(loads[0].2, SyncDirection::Push);
    drop(loads);
    let commits = harness
        .continuity_state
        .completed_candidates
        .lock()
        .expect("completed candidates");
    assert_eq!(commits[0].direction(), SyncDirection::Push);
}

#[tokio::test]
async fn empty_scan_progress_commits_exact_cursor_without_sink_receipts() {
    let harness = Harness::success();
    let scan: Value = serde_json::from_str(SCAN).expect("scan fixture");
    let before: SourceBoundCursor =
        serde_json::from_value(scan["cursor_before"].clone()).expect("cursor");
    let page: SourcePage =
        serde_json::from_value(scan["cases"][0]["source_page"].clone()).expect("scan page");
    *harness
        .continuity_state
        .cursor_result
        .lock()
        .expect("cursor result") = Ok(Some(before));
    *harness.source_state.result.lock().expect("source result") = Ok(page);

    let summary = terminal(
        harness
            .service
            .request_sync(harness.request.clone())
            .await
            .expect("scan progress succeeds"),
    );
    assert_eq!(summary.outcome, SyncOutcomeClassification::ScanProgress);
    assert_eq!(summary.records_examined, 0);
    assert!(summary.cursor_committed);
    assert!(harness
        .sink_state
        .records
        .lock()
        .expect("records")
        .is_empty());
    let commits = harness
        .continuity_state
        .completed_candidates
        .lock()
        .expect("completed candidates");
    assert!(commits[0].receipts().is_empty());
    assert_eq!(
        commits[0].candidate().opaque_token().as_str(),
        "opaque:scan-002"
    );
}

#[tokio::test]
async fn malformed_stalled_page_persists_granular_failure_without_cursor_commit() {
    let harness = Harness::success();
    let scan: Value = serde_json::from_str(SCAN).expect("scan fixture");
    let before: SourceBoundCursor =
        serde_json::from_value(scan["cursor_before"].clone()).expect("cursor");
    let page: SourcePage = serde_json::from_value(scan["malformed_case"]["source_page"].clone())
        .expect("malformed page shape");
    *harness
        .continuity_state
        .cursor_result
        .lock()
        .expect("cursor result") = Ok(Some(before));
    *harness.source_state.result.lock().expect("source result") = Ok(page);

    let summary = terminal(
        harness
            .service
            .request_sync(harness.request.clone())
            .await
            .expect("failure summary persists"),
    );
    assert_eq!(summary.terminal_state, SyncState::Failed);
    assert_eq!(
        summary.outcome,
        SyncOutcomeClassification::MalformedSourceBatch
    );
    assert!(!summary.cursor_committed);
    assert!(harness
        .continuity_state
        .completed_candidates
        .lock()
        .expect("completed candidates")
        .is_empty());
    assert_eq!(
        harness
            .continuity_state
            .terminal_candidates
            .lock()
            .expect("terminal candidates")
            .len(),
        1
    );
}

#[tokio::test]
async fn requested_and_evaluating_cancellation_are_both_durable_safe_boundaries() {
    let requested = Harness::success();
    let session = requested
        .service
        .prepare_sync(requested.request.clone())
        .await
        .expect("requested session");
    let summary = terminal(
        requested
            .service
            .cancel_at_safe_boundary(session)
            .await
            .expect("requested cancellation"),
    );
    assert_eq!(summary.terminal_state, SyncState::Cancelled);

    let evaluating = Harness::success();
    let mut session = evaluating
        .service
        .prepare_sync(evaluating.request.clone())
        .await
        .expect("requested session");
    evaluating
        .service
        .begin_evaluation(&mut session)
        .expect("enter evaluating");
    assert_eq!(session.state(), SyncState::Evaluating);
    let summary = terminal(
        evaluating
            .service
            .cancel_at_safe_boundary(session)
            .await
            .expect("evaluating cancellation"),
    );
    assert_eq!(summary.outcome, SyncOutcomeClassification::Cancelled);
    assert_eq!(evaluating.source_state.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pending_atomic_retry_reuses_exact_candidate_and_session_id() {
    let harness = Harness::success();
    harness
        .continuity_state
        .completed_behaviors
        .lock()
        .expect("completed results")
        .extend([
            CompletedBehavior::Error(ContinuityError::Ambiguous {
                evidence_refs: vec![EvidenceRef::new("continuity:ambiguous").expect("evidence")],
            }),
            CompletedBehavior::Match(CompletedCommitDisposition::AlreadyCommittedSame),
        ]);

    let first = harness
        .service
        .request_sync(harness.request.clone())
        .await
        .expect("pending result");
    let pending = match first {
        SyncProcedureResult::PendingContinuity { pending } => *pending,
        SyncProcedureResult::Terminal { .. } => panic!("expected pending continuity"),
    };
    let exact = pending.exact_candidate().clone();
    let session_id = pending.session_id().clone();
    assert_eq!(pending.prior_lifecycle_state(), SyncState::CommittingCursor);

    let summary = terminal(
        harness
            .service
            .retry_pending_continuity(pending)
            .await
            .expect("exact retry succeeds"),
    );
    assert_eq!(summary.session_id, session_id);
    assert_eq!(harness.identity_state.calls.load(Ordering::SeqCst), 1);
    let commits = harness
        .continuity_state
        .completed_candidates
        .lock()
        .expect("completed candidates");
    assert_eq!(commits.len(), 2);
    assert_eq!(
        buzz_principal_node::ContinuityCandidate::CompletedAtomic(commits[1].clone()),
        exact
    );
}

#[tokio::test]
async fn pending_terminal_retry_reuses_exact_blocked_summary() {
    let harness = Harness::success();
    *harness.projection_state.result.lock().expect("projection") = Ok(PlanEvaluation::Blocked {
        plan: Box::new(BlockedSyncPlan {
            principal_node_id: harness.request.principal_node_id.clone(),
            source_stream_id: harness.request.source_stream_id.clone(),
            direction: harness.request.direction,
            evaluated_at: serde_json::from_value(harness.fixture["clock"]["evaluated_at"].clone())
                .expect("evaluated clock"),
            classification: BlockedClassification::TransportNotReady,
            evidence_refs: vec![EvidenceRef::new("transport:not-ready").expect("evidence")],
        }),
    });
    harness
        .continuity_state
        .terminal_results
        .lock()
        .expect("terminal results")
        .extend([
            Err(ContinuityError::Unavailable {
                evidence_refs: vec![EvidenceRef::new("continuity:down").expect("evidence")],
            }),
            Ok(TerminalCommitDisposition::AlreadyStoredSame),
        ]);

    let pending = match harness
        .service
        .request_sync(harness.request.clone())
        .await
        .expect("pending result")
    {
        SyncProcedureResult::PendingContinuity { pending } => *pending,
        SyncProcedureResult::Terminal { .. } => panic!("expected pending continuity"),
    };
    let exact = pending.exact_candidate().clone();
    let summary = terminal(
        harness
            .service
            .retry_pending_continuity(pending)
            .await
            .expect("exact terminal retry succeeds"),
    );
    assert_eq!(summary.terminal_state, SyncState::Blocked);
    assert_eq!(harness.identity_state.calls.load(Ordering::SeqCst), 1);
    let candidates = harness
        .continuity_state
        .terminal_candidates
        .lock()
        .expect("terminal candidates");
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        buzz_principal_node::ContinuityCandidate::TerminalSummary(candidates[1].clone()),
        exact
    );
}

#[tokio::test]
async fn rejected_ambiguous_and_transport_failure_never_reach_cursor_commit() {
    for (mode, expected) in [
        (
            SinkMode::Rejected,
            SyncOutcomeClassification::ReceiptRejected,
        ),
        (
            SinkMode::Ambiguous,
            SyncOutcomeClassification::ReceiptAmbiguous,
        ),
        (SinkMode::Failed, SyncOutcomeClassification::TransportFailed),
    ] {
        let harness = Harness::success();
        *harness.sink_state.mode.lock().expect("sink mode") = mode;
        let summary = terminal(
            harness
                .service
                .request_sync(harness.request.clone())
                .await
                .expect("failed summary persists"),
        );
        assert_eq!(summary.terminal_state, SyncState::Failed);
        assert_eq!(summary.outcome, expected);
        assert!(!summary.cursor_committed);
        assert!(harness
            .continuity_state
            .completed_candidates
            .lock()
            .expect("completed candidates")
            .is_empty());
    }
}

#[tokio::test]
async fn deserialized_pending_reentry_revalidates_state_namespace_and_digest() {
    let harness = Harness::success();
    harness
        .continuity_state
        .completed_behaviors
        .lock()
        .expect("completed results")
        .push_back(CompletedBehavior::Error(ContinuityError::Ambiguous {
            evidence_refs: vec![EvidenceRef::new("continuity:ambiguous").expect("evidence")],
        }));
    let pending = match harness
        .service
        .request_sync(harness.request.clone())
        .await
        .expect("pending result")
    {
        SyncProcedureResult::PendingContinuity { pending } => *pending,
        SyncProcedureResult::Terminal { .. } => panic!("expected pending continuity"),
    };
    let encoded = serde_json::to_value(&pending).expect("pending serializes");

    let mut terminal_prior = encoded.clone();
    terminal_prior["prior_lifecycle_state"] = Value::String("completed".to_string());
    let terminal_prior = serde_json::from_value(terminal_prior).expect("shape deserializes");
    assert!(harness
        .service
        .retry_pending_continuity(terminal_prior)
        .await
        .is_err());

    let mut wrong_direction = encoded.clone();
    wrong_direction["exact_candidate"]["summary"]["direction"] = Value::String("push".to_string());
    let wrong_direction = serde_json::from_value(wrong_direction).expect("shape deserializes");
    assert!(harness
        .service
        .retry_pending_continuity(wrong_direction)
        .await
        .is_err());

    let mut wrong_digest = encoded;
    wrong_digest["candidate_digest"] = Value::String(
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
    );
    let wrong_digest = serde_json::from_value(wrong_digest).expect("shape deserializes");
    assert!(harness
        .service
        .retry_pending_continuity(wrong_digest)
        .await
        .is_err());
    assert_eq!(harness.identity_state.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn every_declared_trigger_routes_through_request_sync_with_real_retry_lineage() {
    let fixture: TriggerServiceFixture = serde_json::from_str(TRIGGERS).expect("trigger fixture");
    assert_eq!(fixture.cases.len(), 6);

    for vector in fixture.cases {
        let mut harness = Harness::success();
        harness.request.trigger = vector.trigger;
        harness.request.previous_session_id = vector.previous_session_id.clone();
        if let Some(previous_session_id) = vector.previous_session_id.clone() {
            *harness
                .continuity_state
                .prior_summary
                .lock()
                .expect("prior summary") = Some(retryable_prior(&harness, previous_session_id));
        }

        let summary = terminal(
            harness
                .service
                .request_sync(harness.request.clone())
                .await
                .expect("trigger uses shared procedure"),
        );
        assert_eq!(summary.trigger, vector.trigger);
        assert_eq!(summary.previous_session_id, vector.previous_session_id);
        assert_eq!(summary.terminal_state, SyncState::Completed);
        assert_eq!(harness.source_state.calls.load(Ordering::SeqCst), 1);
        assert_eq!(harness.identity_state.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn invariant_invalid_retry_lineage_is_rejected_before_identity_or_evaluation() {
    let mut harness = Harness::success();
    let previous_session_id =
        SyncSessionId::new("session:structurally-typed-invalid-prior").expect("session ID");
    harness.request.trigger = SyncTrigger::Retry;
    harness.request.previous_session_id = Some(previous_session_id.clone());

    let mut previous = retryable_prior(&harness, previous_session_id);
    previous.outcome = SyncOutcomeClassification::CaughtUp;
    assert!(previous.validate().is_err());
    *harness
        .continuity_state
        .prior_summary
        .lock()
        .expect("prior summary") = Some(previous);

    let error = harness
        .service
        .request_sync(harness.request.clone())
        .await
        .expect_err("invariant-invalid prior summary must fail closed");

    assert!(matches!(error, SyncServiceError::PriorSessionNotRetryable));
    assert_eq!(harness.identity_state.calls.load(Ordering::SeqCst), 0);
    assert!(harness
        .continuity_state
        .cursor_loads
        .lock()
        .expect("cursor loads")
        .is_empty());
    assert!(harness
        .transport_state
        .requests
        .lock()
        .expect("transport requests")
        .is_empty());
    assert!(harness
        .projection_state
        .requests
        .lock()
        .expect("projection requests")
        .is_empty());
    assert_eq!(harness.source_state.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn all_five_evaluation_failure_vectors_become_durable_typed_failed_summaries() {
    let fixture: EvaluationServiceFixture =
        serde_json::from_str(EVALUATION_FAILURES).expect("evaluation fixture");
    assert_eq!(fixture.cases.len(), 5);

    for vector in fixture.cases {
        let harness = Harness::success();
        let failure =
            EvaluationFailure::new(vector.classification, vec![vector.evidence_ref.clone()]);
        match vector.classification {
            EvaluationFailureClassification::CursorLoadFailed => {
                *harness
                    .continuity_state
                    .cursor_result
                    .lock()
                    .expect("cursor result") = Err(failure);
            }
            EvaluationFailureClassification::TransportUnavailable => {
                *harness
                    .transport_state
                    .result
                    .lock()
                    .expect("transport result") = Err(failure);
            }
            EvaluationFailureClassification::ProjectionUnavailable => {
                *harness.projection_state.result.lock().expect("projection") = Err(failure);
            }
            EvaluationFailureClassification::SourceUnavailable
            | EvaluationFailureClassification::MalformedSourceBatch => {
                *harness.source_state.result.lock().expect("source result") = Err(failure);
            }
        }

        let summary = terminal(
            harness
                .service
                .request_sync(harness.request.clone())
                .await
                .expect("failed summary becomes durable"),
        );
        assert_eq!(summary.terminal_state, SyncState::Failed);
        assert_eq!(
            summary.outcome,
            SyncOutcomeClassification::from(vector.classification)
        );
        assert!(summary.evidence_refs.contains(&vector.evidence_ref));
        assert!(!summary.cursor_committed);
        assert!(harness
            .continuity_state
            .completed_candidates
            .lock()
            .expect("completed candidates")
            .is_empty());
        assert_eq!(
            harness
                .continuity_state
                .terminal_candidates
                .lock()
                .expect("terminal candidates")
                .len(),
            1
        );
    }
}

#[tokio::test]
async fn all_four_fully_bound_blocked_plans_persist_the_declared_classification() {
    for classification in [
        BlockedClassification::NodeUnauthorized,
        BlockedClassification::AgreementNotReady,
        BlockedClassification::TransportNotReady,
        BlockedClassification::RequiredCapabilityMissing,
    ] {
        let harness = Harness::success();
        let evidence =
            EvidenceRef::new(format!("blocked:{classification:?}")).expect("blocked evidence");
        let blocked = BlockedSyncPlan {
            principal_node_id: harness.request.principal_node_id.clone(),
            source_stream_id: harness.request.source_stream_id.clone(),
            direction: harness.request.direction,
            evaluated_at: evaluated_clock(&harness),
            classification,
            evidence_refs: vec![evidence.clone()],
        };
        let blocked_wire = serde_json::to_value(&blocked).expect("blocked plan serializes");
        assert_eq!(blocked_wire.as_object().expect("blocked object").len(), 6);
        *harness.projection_state.result.lock().expect("projection") =
            Ok(PlanEvaluation::Blocked {
                plan: Box::new(blocked),
            });

        let summary = terminal(
            harness
                .service
                .request_sync(harness.request.clone())
                .await
                .expect("blocked summary becomes durable"),
        );
        assert_eq!(summary.terminal_state, SyncState::Blocked);
        assert_eq!(
            summary.outcome,
            SyncOutcomeClassification::from(classification)
        );
        assert_eq!(summary.evidence_refs, vec![evidence]);
        assert_eq!(harness.source_state.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn caught_up_unchanged_empty_fixture_persists_no_work_without_atomic_commit() {
    let fixture: EmptyServiceFixture = serde_json::from_str(EMPTY).expect("empty fixture");
    let mut harness = Harness::success();
    harness.request = fixture.request;
    *harness
        .continuity_state
        .cursor_result
        .lock()
        .expect("cursor result") = Ok(Some(fixture.cursor_before));
    *harness.source_state.result.lock().expect("source result") = Ok(fixture.source_page);

    let summary = terminal(
        harness
            .service
            .request_sync(harness.request.clone())
            .await
            .expect("caught-up no-work succeeds"),
    );
    assert_eq!(summary.terminal_state, SyncState::Completed);
    assert_eq!(summary.outcome, SyncOutcomeClassification::CaughtUp);
    assert_eq!(summary.records_examined, 0);
    assert!(!summary.cursor_committed);
    assert!(harness
        .continuity_state
        .completed_candidates
        .lock()
        .expect("completed candidates")
        .is_empty());
    assert!(harness
        .sink_state
        .records
        .lock()
        .expect("sink records")
        .is_empty());
}

#[tokio::test]
async fn definitive_cursor_compare_failure_persists_failed_summary_without_advance() {
    let harness = Harness::success();
    harness
        .continuity_state
        .completed_behaviors
        .lock()
        .expect("completed behaviors")
        .push_back(CompletedBehavior::Error(
            ContinuityError::CursorCompareConflict {
                evidence_refs: vec![EvidenceRef::new("cursor:conflict").expect("evidence")],
            },
        ));

    let summary = terminal(
        harness
            .service
            .request_sync(harness.request.clone())
            .await
            .expect("cursor failure summary persists"),
    );
    assert_eq!(summary.terminal_state, SyncState::Failed);
    assert_eq!(
        summary.outcome,
        SyncOutcomeClassification::CursorCommitFailed
    );
    assert!(!summary.cursor_committed);
    assert_eq!(
        harness
            .continuity_state
            .terminal_candidates
            .lock()
            .expect("terminal candidates")
            .len(),
        1
    );
}

#[tokio::test]
async fn mismatched_atomic_acknowledgements_fail_closed_for_initial_and_pending_commits() {
    let direct = Harness::success();
    direct
        .continuity_state
        .completed_behaviors
        .lock()
        .expect("completed behaviors")
        .push_back(CompletedBehavior::Mismatch);
    let error = direct
        .service
        .request_sync(direct.request.clone())
        .await
        .expect_err("mismatched acknowledgement must fail");
    assert!(matches!(
        error,
        SyncServiceError::ContinuityAcknowledgementMismatch
    ));

    let replay = Harness::success();
    replay
        .continuity_state
        .completed_behaviors
        .lock()
        .expect("completed behaviors")
        .extend([
            CompletedBehavior::Error(ContinuityError::Ambiguous {
                evidence_refs: vec![EvidenceRef::new("continuity:ambiguous").expect("evidence")],
            }),
            CompletedBehavior::Mismatch,
        ]);
    let pending = match replay
        .service
        .request_sync(replay.request.clone())
        .await
        .expect("pending result")
    {
        SyncProcedureResult::PendingContinuity { pending } => *pending,
        SyncProcedureResult::Terminal { .. } => panic!("expected pending result"),
    };
    let error = replay
        .service
        .retry_pending_continuity(pending)
        .await
        .expect_err("pending mismatched acknowledgement must fail");
    assert!(matches!(
        error,
        SyncServiceError::ContinuityAcknowledgementMismatch
    ));
    assert_eq!(replay.identity_state.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stale_transport_and_plan_time_evidence_are_rejected_before_transfer() {
    let stale_transport = Harness::success();
    let mut authentication = stale_transport
        .transport_state
        .result
        .lock()
        .expect("transport result")
        .clone()
        .expect("authentication");
    if let TransportAuthentication::Authenticated { evidence } = &mut authentication {
        evidence.authenticated_at = ClockSample::new("2026-08-04T13:59:00Z", 11999);
    }
    *stale_transport
        .transport_state
        .result
        .lock()
        .expect("transport result") = Ok(authentication);
    let summary = terminal(
        stale_transport
            .service
            .request_sync(stale_transport.request.clone())
            .await
            .expect("stale transport becomes durable failure"),
    );
    assert_eq!(
        summary.outcome,
        SyncOutcomeClassification::TransportUnavailable
    );
    assert_eq!(stale_transport.source_state.calls.load(Ordering::SeqCst), 0);

    let stale_plan = Harness::success();
    let mut evaluation = stale_plan
        .projection_state
        .result
        .lock()
        .expect("projection")
        .clone()
        .expect("plan evaluation");
    if let PlanEvaluation::Ready { plan } = &mut evaluation {
        plan.evaluated_at = ClockSample::new("2026-08-04T13:59:00Z", 11999);
    }
    *stale_plan
        .projection_state
        .result
        .lock()
        .expect("projection") = Ok(evaluation);
    let summary = terminal(
        stale_plan
            .service
            .request_sync(stale_plan.request.clone())
            .await
            .expect("stale plan becomes durable failure"),
    );
    assert_eq!(
        summary.outcome,
        SyncOutcomeClassification::ProjectionUnavailable
    );
    assert_eq!(stale_plan.source_state.calls.load(Ordering::SeqCst), 0);

    let mismatched_blocked = Harness::success();
    *mismatched_blocked
        .projection_state
        .result
        .lock()
        .expect("projection") = Ok(PlanEvaluation::Blocked {
        plan: Box::new(BlockedSyncPlan {
            principal_node_id: mismatched_blocked.request.principal_node_id.clone(),
            source_stream_id: mismatched_blocked.request.source_stream_id.clone(),
            direction: SyncDirection::Push,
            evaluated_at: evaluated_clock(&mismatched_blocked),
            classification: BlockedClassification::AgreementNotReady,
            evidence_refs: vec![EvidenceRef::new("blocked:mismatch").expect("evidence")],
        }),
    });
    let summary = terminal(
        mismatched_blocked
            .service
            .request_sync(mismatched_blocked.request.clone())
            .await
            .expect("mismatched blocked plan becomes durable failure"),
    );
    assert_eq!(summary.terminal_state, SyncState::Failed);
    assert_eq!(
        summary.outcome,
        SyncOutcomeClassification::ProjectionUnavailable
    );
}

#[derive(Deserialize)]
struct TriggerServiceFixture {
    cases: Vec<TriggerServiceCase>,
}

#[derive(Deserialize)]
struct TriggerServiceCase {
    trigger: SyncTrigger,
    previous_session_id: Option<SyncSessionId>,
}

#[derive(Deserialize)]
struct EvaluationServiceFixture {
    cases: Vec<EvaluationServiceCase>,
}

#[derive(Deserialize)]
struct EvaluationServiceCase {
    classification: EvaluationFailureClassification,
    evidence_ref: EvidenceRef,
}

#[derive(Deserialize)]
struct EmptyServiceFixture {
    request: SyncRequest,
    cursor_before: SourceBoundCursor,
    source_page: SourcePage,
}

fn evaluated_clock(harness: &Harness) -> ClockSample {
    serde_json::from_value(harness.fixture["clock"]["evaluated_at"].clone())
        .expect("evaluated clock")
}

fn retryable_prior(
    harness: &Harness,
    previous_session_id: SyncSessionId,
) -> TerminalSyncSessionSummary {
    let mut summary: TerminalSyncSessionSummary =
        serde_json::from_value(harness.fixture["expected_summary"].clone())
            .expect("expected summary");
    summary.session_id = previous_session_id;
    summary.trigger = SyncTrigger::Startup;
    summary.terminal_state = SyncState::Failed;
    summary.outcome = SyncOutcomeClassification::SourceUnavailable;
    summary.records_examined = 0;
    summary.cursor_committed = false;
    summary.cursor_after = None;
    summary.receipt_summary = None;
    summary.previous_session_id = None;
    summary
        .validate()
        .expect("retryable prior summary validates");
    summary
}

struct Harness {
    fixture: Value,
    request: SyncRequest,
    record: ReplicationRecord,
    service: Service,
    identity_state: Arc<IdentityState>,
    transport_state: Arc<TransportState>,
    source_state: Arc<SourceState>,
    sink_state: Arc<SinkState>,
    projection_state: Arc<ProjectionState>,
    continuity_state: Arc<ContinuityState>,
}

impl Harness {
    fn success() -> Self {
        Self::success_direction(SyncDirection::Pull)
    }

    fn success_direction(direction: SyncDirection) -> Self {
        let fixture: Value = serde_json::from_str(SUCCESS).expect("success fixture");
        let mut request: SyncRequest =
            serde_json::from_value(fixture["request"].clone()).expect("request");
        let mut transport_evidence: AuthenticatedTransportEvidence =
            serde_json::from_value(fixture["transport_evidence"].clone()).expect("transport");
        let mut plan: CurrentSyncPlan =
            serde_json::from_value(fixture["current_plan"].clone()).expect("plan");
        request.direction = direction;
        transport_evidence.direction = direction;
        plan.direction = direction;
        let page: SourcePage =
            serde_json::from_value(fixture["source_page"].clone()).expect("page");
        let record = page.batch.records[0].clone();
        let cursor: SourceBoundCursor =
            serde_json::from_value(fixture["cursor_before"].clone()).expect("cursor");
        let clocks = ["started_at", "evaluated_at", "finished_at"]
            .into_iter()
            .map(|key| serde_json::from_value(fixture["clock"][key].clone()).expect("clock"))
            .collect();

        let identity_state = Arc::new(IdentityState::default());
        let transport_state = Arc::new(TransportState {
            requests: Mutex::new(Vec::new()),
            result: Mutex::new(Ok(TransportAuthentication::Authenticated {
                evidence: transport_evidence,
            })),
        });
        let source_state = Arc::new(SourceState {
            calls: AtomicUsize::new(0),
            directions: Mutex::new(Vec::new()),
            transport_evidence: Mutex::new(Vec::new()),
            result: Mutex::new(Ok(page)),
        });
        let sink_state = Arc::new(SinkState {
            records: Mutex::new(Vec::new()),
            directions: Mutex::new(Vec::new()),
            transport_evidence: Mutex::new(Vec::new()),
            mode: Mutex::new(SinkMode::Stored),
        });
        let projection_state = Arc::new(ProjectionState {
            requests: Mutex::new(Vec::new()),
            result: Mutex::new(Ok(PlanEvaluation::Ready {
                plan: Box::new(plan),
            })),
        });
        let continuity_state = Arc::new(ContinuityState {
            cursor_result: Mutex::new(Ok(Some(cursor))),
            prior_summary: Mutex::new(None),
            cursor_loads: Mutex::new(Vec::new()),
            terminal_results: Mutex::new(VecDeque::new()),
            completed_behaviors: Mutex::new(VecDeque::new()),
            terminal_candidates: Mutex::new(Vec::new()),
            completed_candidates: Mutex::new(Vec::new()),
        });
        let service = PrincipalNodeSyncService::new(
            MockIdentityIssuer {
                state: Arc::clone(&identity_state),
                issued: serde_json::from_value(fixture["issued_session_id"].clone())
                    .expect("session ID"),
            },
            MockClock {
                samples: Arc::new(Mutex::new(clocks)),
                fallback: ClockSample::new("2026-08-04T14:00:04Z", 12004),
            },
            MockTransport {
                state: Arc::clone(&transport_state),
            },
            MockProjection {
                state: Arc::clone(&projection_state),
            },
            MockSource {
                state: Arc::clone(&source_state),
            },
            MockSink {
                state: Arc::clone(&sink_state),
            },
            MockContinuity {
                state: Arc::clone(&continuity_state),
            },
        );
        Self {
            fixture,
            request,
            record,
            service,
            identity_state,
            transport_state,
            source_state,
            sink_state,
            projection_state,
            continuity_state,
        }
    }
}

#[derive(Default)]
struct IdentityState {
    calls: AtomicUsize,
}

struct MockIdentityIssuer {
    state: Arc<IdentityState>,
    issued: SyncSessionId,
}

impl SyncSessionIdentityIssuer for MockIdentityIssuer {
    fn issue_session_id(&self, _: &buzz_principal_node::PrincipalNodeId) -> SyncSessionId {
        self.state.calls.fetch_add(1, Ordering::SeqCst);
        self.issued.clone()
    }
}

struct MockClock {
    samples: Arc<Mutex<VecDeque<ClockSample>>>,
    fallback: ClockSample,
}

impl AttemptClock for MockClock {
    fn now(&self) -> ClockSample {
        self.samples
            .lock()
            .expect("clock samples")
            .pop_front()
            .unwrap_or_else(|| self.fallback.clone())
    }
}

struct TransportState {
    requests: Mutex<Vec<PeerAuthenticationRequest>>,
    result: Mutex<Result<TransportAuthentication, EvaluationFailure>>,
}

struct MockTransport {
    state: Arc<TransportState>,
}

impl AuthenticatedPeerTransport for MockTransport {
    async fn authenticate_peer(
        &self,
        request: PeerAuthenticationRequest,
    ) -> Result<TransportAuthentication, EvaluationFailure> {
        self.state
            .requests
            .lock()
            .expect("transport requests")
            .push(request);
        self.state.result.lock().expect("transport result").clone()
    }
}

struct ProjectionState {
    requests: Mutex<Vec<ProjectionRequest>>,
    result: Mutex<Result<PlanEvaluation, EvaluationFailure>>,
}

struct MockProjection {
    state: Arc<ProjectionState>,
}

impl CurrentSyncProjection for MockProjection {
    async fn evaluate_current_plan(
        &self,
        request: ProjectionRequest,
    ) -> Result<PlanEvaluation, EvaluationFailure> {
        self.state
            .requests
            .lock()
            .expect("projection requests")
            .push(request);
        self.state.result.lock().expect("projection").clone()
    }
}

struct SourceState {
    calls: AtomicUsize,
    directions: Mutex<Vec<SyncDirection>>,
    transport_evidence: Mutex<Vec<AuthenticatedTransportEvidence>>,
    result: Mutex<Result<SourcePage, EvaluationFailure>>,
}

struct MockSource {
    state: Arc<SourceState>,
}

impl ReplicationSource for MockSource {
    async fn read_bounded_page(
        &self,
        request: SourcePageRequest,
    ) -> Result<SourcePage, EvaluationFailure> {
        self.state.calls.fetch_add(1, Ordering::SeqCst);
        self.state
            .directions
            .lock()
            .expect("source directions")
            .push(request.transport_evidence.direction);
        self.state
            .transport_evidence
            .lock()
            .expect("source transport evidence")
            .push(request.transport_evidence);
        self.state.result.lock().expect("source result").clone()
    }
}

#[derive(Clone, Copy)]
enum SinkMode {
    Stored,
    Rejected,
    Ambiguous,
    Failed,
}

struct SinkState {
    records: Mutex<Vec<ReplicationRecord>>,
    directions: Mutex<Vec<SyncDirection>>,
    transport_evidence: Mutex<Vec<AuthenticatedTransportEvidence>>,
    mode: Mutex<SinkMode>,
}

struct MockSink {
    state: Arc<SinkState>,
}

impl ReplicationSink for MockSink {
    async fn ingest_exact(
        &self,
        request: SinkIngestRequest,
    ) -> Result<ReceiptEvidence, TransferFailure> {
        self.state
            .directions
            .lock()
            .expect("sink directions")
            .push(request.current_plan.direction);
        self.state
            .transport_evidence
            .lock()
            .expect("sink transport evidence")
            .push(request.transport_evidence.clone());
        self.state
            .records
            .lock()
            .expect("records")
            .push(request.record.clone());
        let digest = EvidenceRef::new("receipt:digest").expect("evidence");
        match *self.state.mode.lock().expect("sink mode") {
            SinkMode::Stored => Ok(ReceiptEvidence::from_receipt(
                ReplicationReceipt {
                    source: request.record.source,
                    cursor: request.record.cursor,
                    event_id: request.record.event.id.to_hex(),
                    outcome: ReplicationIngestOutcome::Stored,
                },
                true,
                EvidenceRef::new(
                    "sha256:3333333333333333333333333333333333333333333333333333333333333333",
                )
                .expect("digest"),
            )
            .expect("valid event ID")),
            SinkMode::Rejected => Ok(ReceiptEvidence::from_receipt(
                ReplicationReceipt {
                    source: request.record.source,
                    cursor: request.record.cursor,
                    event_id: request.record.event.id.to_hex(),
                    outcome: ReplicationIngestOutcome::Rejected {
                        reason: "fixture rejection".to_string(),
                    },
                },
                true,
                digest,
            )
            .expect("valid event ID")),
            SinkMode::Ambiguous => {
                Ok(ReceiptEvidence::ambiguous(&request.record, digest).expect("valid event ID"))
            }
            SinkMode::Failed => Err(TransferFailure {
                evidence_refs: vec![digest],
            }),
        }
    }
}

struct ContinuityState {
    cursor_result: Mutex<Result<Option<SourceBoundCursor>, EvaluationFailure>>,
    prior_summary: Mutex<Option<TerminalSyncSessionSummary>>,
    cursor_loads: Mutex<
        Vec<(
            buzz_principal_node::PrincipalNodeId,
            buzz_principal_node::SourceStreamId,
            SyncDirection,
        )>,
    >,
    terminal_results: Mutex<VecDeque<Result<TerminalCommitDisposition, ContinuityError>>>,
    completed_behaviors: Mutex<VecDeque<CompletedBehavior>>,
    terminal_candidates: Mutex<Vec<TerminalCommitCandidate>>,
    completed_candidates: Mutex<Vec<CompletedCommitCandidate>>,
}

enum CompletedBehavior {
    Match(CompletedCommitDisposition),
    Mismatch,
    Error(ContinuityError),
}

struct MockContinuity {
    state: Arc<ContinuityState>,
}

impl PrincipalNodeSyncContinuity for MockContinuity {
    async fn load_cursor(
        &self,
        principal_node_id: buzz_principal_node::PrincipalNodeId,
        source_stream_id: buzz_principal_node::SourceStreamId,
        direction: SyncDirection,
    ) -> Result<Option<SourceBoundCursor>, EvaluationFailure> {
        self.state.cursor_loads.lock().expect("cursor loads").push((
            principal_node_id,
            source_stream_id,
            direction,
        ));
        self.state
            .cursor_result
            .lock()
            .expect("cursor result")
            .clone()
    }

    async fn load_terminal_summary(
        &self,
        _: buzz_principal_node::PrincipalNodeId,
        _: SyncSessionId,
    ) -> Result<Option<TerminalSyncSessionSummary>, ContinuityError> {
        Ok(self
            .state
            .prior_summary
            .lock()
            .expect("prior summary")
            .clone())
    }

    async fn persist_terminal_summary(
        &self,
        candidate: TerminalCommitCandidate,
    ) -> Result<TerminalCommitDisposition, ContinuityError> {
        candidate.validate().expect("terminal candidate validates");
        self.state
            .terminal_candidates
            .lock()
            .expect("terminal candidates")
            .push(candidate);
        self.state
            .terminal_results
            .lock()
            .expect("terminal results")
            .pop_front()
            .unwrap_or(Ok(TerminalCommitDisposition::Stored))
    }

    async fn commit_completed(
        &self,
        candidate: CompletedCommitCandidate,
    ) -> Result<CompletedCommitResult, ContinuityError> {
        candidate.validate().expect("completed candidate validates");
        let result = CompletedCommitResult {
            disposition: CompletedCommitDisposition::Committed,
            committed_cursor: candidate.candidate().clone(),
            committed_summary: candidate.summary().clone(),
        };
        self.state
            .completed_candidates
            .lock()
            .expect("completed candidates")
            .push(candidate);
        match self
            .state
            .completed_behaviors
            .lock()
            .expect("completed behaviors")
            .pop_front()
            .unwrap_or(CompletedBehavior::Match(
                CompletedCommitDisposition::Committed,
            )) {
            CompletedBehavior::Match(disposition) => Ok(CompletedCommitResult {
                disposition,
                ..result
            }),
            CompletedBehavior::Mismatch => Ok(CompletedCommitResult {
                disposition: CompletedCommitDisposition::Committed,
                committed_cursor: SourceBoundCursor::new(
                    result.committed_summary.source_stream_id.clone(),
                    buzz_principal_node::OpaqueSourceCursorToken::new("opaque:mismatched")
                        .expect("cursor token"),
                ),
                committed_summary: result.committed_summary,
            }),
            CompletedBehavior::Error(error) => Err(error),
        }
    }
}

fn terminal(result: SyncProcedureResult) -> TerminalSyncSessionSummary {
    match result {
        SyncProcedureResult::Terminal { summary } => *summary,
        SyncProcedureResult::PendingContinuity { .. } => panic!("expected terminal result"),
    }
}
