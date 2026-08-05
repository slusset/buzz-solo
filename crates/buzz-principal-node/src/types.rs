//! Domain values and application-owned synchronization evidence.

use buzz_core::replication::{
    ReplicationBatch, ReplicationCursor, ReplicationIngestOutcome, ReplicationReceipt,
    ReplicationRecord, ReplicationSourceId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Creates a validated value.
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                if value.is_empty() {
                    return Err(IdentifierError::Empty { kind: $kind });
                }
                Ok(Self(value))
            }

            /// Returns the underlying value without changing its meaning.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentifierError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

/// Error returned when a compiler-distinct identifier is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentifierError {
    /// The identifier was empty.
    #[error("{kind} must not be empty")]
    Empty {
        /// Name of the invalid identifier type.
        kind: &'static str,
    },
}

string_newtype!(
    /// Stable operational identity of one Principal Node.
    PrincipalNodeId,
    "PrincipalNodeId"
);
string_newtype!(
    /// Immutable identity of one replication source and selection predicate.
    SourceStreamId,
    "SourceStreamId"
);
string_newtype!(
    /// Immutable identity of one synchronization attempt.
    SyncSessionId,
    "SyncSessionId"
);
string_newtype!(
    /// Authenticated peer identity observed by a transport adapter.
    TransportPrincipal,
    "TransportPrincipal"
);
string_newtype!(
    /// Bounded reference to authority, declaration, receipt, or failure evidence.
    EvidenceRef,
    "EvidenceRef"
);
string_newtype!(
    /// Opaque source-issued cursor token.
    OpaqueSourceCursorToken,
    "OpaqueSourceCursorToken"
);
string_newtype!(
    /// Canonical digest binding an immutable continuity candidate.
    CandidateDigest,
    "CandidateDigest"
);

impl SourceStreamId {
    /// Projects this application identity into the portable relay source type.
    pub fn to_replication_source_id(&self) -> ReplicationSourceId {
        ReplicationSourceId::new(self.as_str())
    }

    /// Returns whether a portable source identity names this exact stream.
    pub fn matches_replication_source(&self, source: &ReplicationSourceId) -> bool {
        self.as_str() == source.as_str()
    }
}

impl OpaqueSourceCursorToken {
    /// Projects this opaque value into the portable relay cursor type.
    pub fn to_replication_cursor(&self) -> ReplicationCursor {
        ReplicationCursor::new(self.as_str())
    }

    /// Creates the application token from an unchanged portable cursor.
    pub fn from_replication_cursor(cursor: &ReplicationCursor) -> Result<Self, IdentifierError> {
        Self::new(cursor.as_str())
    }
}

/// Direction in which the selected source and sink adapters are composed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    /// Read from a peer source and ingest into the local sink.
    Pull,
    /// Read from the local source and ingest into a peer sink.
    Push,
}

/// Timing signal that requested a synchronization evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTrigger {
    /// Runtime startup requested evaluation.
    Startup,
    /// A durable journal commit requested evaluation.
    JournalCommit,
    /// An authenticated peer wake requested evaluation.
    PeerWake,
    /// Principal-Node recovery policy requested evaluation.
    RecoveryTick,
    /// An operator requested evaluation.
    OperatorRequest,
    /// A new attempt references an immutable failed or blocked attempt.
    Retry,
}

/// State of one synchronization attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    /// A valid timing request created the attempt.
    Requested,
    /// Current authority, plan, cursor, and one source page are being evaluated.
    Evaluating,
    /// Selected event envelopes are being delivered.
    Transferring,
    /// Destination evidence is being classified.
    AwaitingDurableReceipts,
    /// An exact cursor and completed summary candidate awaits atomic continuity.
    CommittingCursor,
    /// The immutable completed summary is durable.
    Completed,
    /// Current authority or readiness did not permit transfer.
    Blocked,
    /// A durable failed summary records an operational or safety failure.
    Failed,
    /// A durable cancelled summary records safe-boundary quiescence.
    Cancelled,
}

impl SyncState {
    /// Returns whether this state is a terminal committed fact.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Blocked | Self::Failed | Self::Cancelled
        )
    }
}

/// Exact source-bound cursor owned by Principal Node continuity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceBoundCursor {
    source_stream_id: SourceStreamId,
    opaque_token: OpaqueSourceCursorToken,
}

impl SourceBoundCursor {
    /// Creates a source-bound opaque cursor.
    pub fn new(source_stream_id: SourceStreamId, opaque_token: OpaqueSourceCursorToken) -> Self {
        Self {
            source_stream_id,
            opaque_token,
        }
    }

    /// Returns the source stream that owns the token.
    pub fn source_stream_id(&self) -> &SourceStreamId {
        &self.source_stream_id
    }

    /// Returns the exact opaque token.
    pub fn opaque_token(&self) -> &OpaqueSourceCursorToken {
        &self.opaque_token
    }

    /// Returns the unchanged portable cursor value.
    pub fn to_replication_cursor(&self) -> ReplicationCursor {
        self.opaque_token.to_replication_cursor()
    }
}

/// Infallible clock evidence supplied by runtime composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockSample {
    /// Wall-clock timestamp represented without host-specific clock APIs.
    pub wall_time: String,
    /// Monotonic tick comparable only with the same clock adapter.
    pub monotonic_tick: u64,
}

impl ClockSample {
    /// Creates bounded attempt clock evidence.
    pub fn new(wall_time: impl Into<String>, monotonic_tick: u64) -> Self {
        Self {
            wall_time: wall_time.into(),
            monotonic_tick,
        }
    }
}

/// Opaque current event selection projected from signed declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventSelection(Value);

impl EventSelection {
    /// Creates a projected selection value.
    pub fn new(value: Value) -> Self {
        Self(value)
    }

    /// Returns the projected selection without granting it authority.
    pub fn as_value(&self) -> &Value {
        &self.0
    }
}

/// Validated synchronization request containing no authority-bearing fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncRequest {
    /// Principal Node that owns the attempt.
    pub principal_node_id: PrincipalNodeId,
    /// Adapter composition direction.
    pub direction: SyncDirection,
    /// Source stream selector, not a credential.
    pub source_stream_id: SourceStreamId,
    /// Timing trigger.
    pub trigger: SyncTrigger,
    /// Immutable prior attempt referenced only by a retry trigger.
    pub previous_session_id: Option<SyncSessionId>,
}

impl SyncRequest {
    /// Creates and validates a synchronization request.
    pub fn new(
        principal_node_id: PrincipalNodeId,
        direction: SyncDirection,
        source_stream_id: SourceStreamId,
        trigger: SyncTrigger,
        previous_session_id: Option<SyncSessionId>,
    ) -> Result<Self, RequestValidationError> {
        let request = Self {
            principal_node_id,
            direction,
            source_stream_id,
            trigger,
            previous_session_id,
        };
        request.validate()?;
        Ok(request)
    }

    /// Revalidates retry lineage constraints.
    pub fn validate(&self) -> Result<(), RequestValidationError> {
        match (self.trigger, self.previous_session_id.as_ref()) {
            (SyncTrigger::Retry, None) => Err(RequestValidationError::RetryMissingPrevious),
            (SyncTrigger::Retry, Some(_)) | (_, None) => Ok(()),
            (_, Some(_)) => Err(RequestValidationError::UnexpectedPrevious),
        }
    }
}

/// Validation failure for an authority-free synchronization request.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RequestValidationError {
    /// A retry did not reference the prior terminal attempt.
    #[error("retry requires previous_session_id")]
    RetryMissingPrevious,
    /// A non-retry trigger attempted to provide retry lineage.
    #[error("previous_session_id is permitted only for retry")]
    UnexpectedPrevious,
}

/// Fresh authenticated peer evidence for this attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedTransportEvidence {
    /// Principal derived from authentication rather than trigger data.
    pub transport_principal: TransportPrincipal,
    /// Stream to which the transport binding applies.
    pub source_stream_id: SourceStreamId,
    /// Direction to which the transport binding applies.
    pub direction: SyncDirection,
    /// Time at which authentication was observed.
    pub authenticated_at: ClockSample,
    /// Bounded adapter evidence reference.
    pub evidence_ref: EvidenceRef,
}

/// Result of fresh peer authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransportAuthentication {
    /// The peer was authenticated for this stream and direction.
    Authenticated {
        /// Fresh authentication evidence.
        evidence: AuthenticatedTransportEvidence,
    },
    /// The peer is not currently authenticated or admitted.
    NotReady {
        /// Evidence explaining the blocked readiness result.
        evidence_refs: Vec<EvidenceRef>,
    },
}

/// Current immutable synchronization plan derived for one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentSyncPlan {
    /// Principal Node evaluated by the projection.
    pub principal_node_id: PrincipalNodeId,
    /// Immutable stream and predicate identity.
    pub source_stream_id: SourceStreamId,
    /// Adapter composition direction.
    pub direction: SyncDirection,
    /// Current domain authorization evidence.
    pub domain_authorization_ref: EvidenceRef,
    /// Current Principal Node authorization evidence.
    pub node_authorization_ref: EvidenceRef,
    /// Current agreement snapshot reference.
    pub agreement_snapshot_ref: EvidenceRef,
    /// Current export declaration head.
    pub export_head_ref: EvidenceRef,
    /// Current admit declaration head.
    pub admit_head_ref: EvidenceRef,
    /// Current declaration-projected selection.
    pub selection: EventSelection,
    /// Authenticated transport principal.
    pub transport_principal: TransportPrincipal,
    /// Fresh transport evidence reference.
    pub transport_evidence_ref: EvidenceRef,
    /// Evidence time for this projection.
    pub evaluated_at: ClockSample,
    /// Maximum number of source records in the one bounded page.
    pub batch_limit: usize,
}

/// Current readiness classification that prevents transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedClassification {
    /// Principal Node authorization is not current.
    NodeUnauthorized,
    /// The stream agreement is not matched and current.
    AgreementNotReady,
    /// No authenticated admitted peer is ready.
    TransportNotReady,
    /// A required mechanical capability is absent.
    RequiredCapabilityMissing,
}

/// Fully bound current synchronization plan that refuses transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockedSyncPlan {
    /// Principal Node evaluated by the projection.
    pub principal_node_id: PrincipalNodeId,
    /// Immutable stream and predicate identity.
    pub source_stream_id: SourceStreamId,
    /// Adapter composition direction.
    pub direction: SyncDirection,
    /// Exact time evidence supplied to the current projection.
    pub evaluated_at: ClockSample,
    /// Stable blocked classification.
    pub classification: BlockedClassification,
    /// Bounded current evidence supporting refusal.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl BlockedSyncPlan {
    /// Returns whether the blocked result belongs to this exact evaluation request.
    pub fn matches_request(&self, request: &SyncRequest, evaluated_at: &ClockSample) -> bool {
        self.principal_node_id == request.principal_node_id
            && self.source_stream_id == request.source_stream_id
            && self.direction == request.direction
            && &self.evaluated_at == evaluated_at
    }
}

/// Current projection result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PlanEvaluation {
    /// Current authority and readiness permit one bounded page.
    Ready {
        /// Current immutable plan.
        plan: Box<CurrentSyncPlan>,
    },
    /// Current evidence blocks transfer.
    Blocked {
        /// Fully bound current blocked plan.
        plan: Box<BlockedSyncPlan>,
    },
}

/// Operational failure classification before transfer begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationFailureClassification {
    /// Durable cursor load failed.
    CursorLoadFailed,
    /// Authenticated peer transport was unavailable.
    TransportUnavailable,
    /// Current authority/declaration projection was unavailable.
    ProjectionUnavailable,
    /// The bounded source read failed.
    SourceUnavailable,
    /// The source page violated its bounded-page contract.
    MalformedSourceBatch,
}

/// Granular bounded evidence for an evaluation failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationFailure {
    /// Stable operational classification.
    pub classification: EvaluationFailureClassification,
    /// Bounded evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl EvaluationFailure {
    /// Creates granular evaluation failure evidence.
    pub fn new(
        classification: EvaluationFailureClassification,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Self {
        Self {
            classification,
            evidence_refs,
        }
    }
}

/// Source-owned equality classification for one bounded page cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCursorProgress {
    /// The next cursor equals the prior token or source-declared initial no-position.
    Unchanged,
    /// The exact source-issued next cursor advances scan progress.
    Advanced,
}

/// One bounded portable replication page plus source-owned cursor equality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePage {
    /// Unchanged portable relay batch.
    pub batch: ReplicationBatch,
    /// Source-owned classification relative to the requested cursor.
    pub cursor_progress: SourceCursorProgress,
}

impl SourcePage {
    /// Creates an application page wrapper without changing the core batch.
    pub fn new(batch: ReplicationBatch, cursor_progress: SourceCursorProgress) -> Self {
        Self {
            batch,
            cursor_progress,
        }
    }

    /// Validates source binding, page bound, and cursor equality semantics.
    pub fn validate(
        &self,
        stream: &SourceStreamId,
        cursor_before: Option<&SourceBoundCursor>,
        batch_limit: usize,
    ) -> Result<(), SourcePageError> {
        if batch_limit == 0 || self.batch.records.len() > batch_limit {
            return Err(SourcePageError::BatchLimitExceeded);
        }
        if self.batch.next_cursor.as_str().is_empty() {
            return Err(SourcePageError::EmptyCursorToken);
        }
        if self
            .batch
            .records
            .iter()
            .any(|record| record.cursor.as_str().is_empty())
        {
            return Err(SourcePageError::EmptyRecordCursorToken);
        }
        if self
            .batch
            .records
            .iter()
            .any(|record| !stream.matches_replication_source(&record.source))
        {
            return Err(SourcePageError::SourceMismatch);
        }
        if let Some(cursor) = cursor_before {
            if cursor.source_stream_id() != stream {
                return Err(SourcePageError::CursorSourceMismatch);
            }
            let equal = cursor.opaque_token().as_str() == self.batch.next_cursor.as_str();
            match (self.cursor_progress, equal) {
                (SourceCursorProgress::Unchanged, false)
                | (SourceCursorProgress::Advanced, true) => {
                    return Err(SourcePageError::ContradictoryProgress)
                }
                _ => {}
            }
        }
        if !self.batch.records.is_empty() && self.cursor_progress == SourceCursorProgress::Unchanged
        {
            return Err(SourcePageError::ContradictoryProgress);
        }
        if self.batch.records.is_empty()
            && !self.batch.caught_up
            && self.cursor_progress == SourceCursorProgress::Unchanged
        {
            return Err(SourcePageError::StalledEmptyPage);
        }
        Ok(())
    }

    /// Returns the exact source-bound candidate cursor.
    pub fn candidate_cursor(
        &self,
        stream: SourceStreamId,
    ) -> Result<SourceBoundCursor, IdentifierError> {
        let token = OpaqueSourceCursorToken::from_replication_cursor(&self.batch.next_cursor)?;
        Ok(SourceBoundCursor::new(stream, token))
    }
}

/// Malformed source-page evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SourcePageError {
    /// The page exceeded the current plan bound or the bound was zero.
    #[error("source page exceeds the current batch limit")]
    BatchLimitExceeded,
    /// The required source-issued next cursor was empty.
    #[error("source page next cursor must not be empty")]
    EmptyCursorToken,
    /// A returned record contained an empty opaque cursor token.
    #[error("source page record cursor must not be empty")]
    EmptyRecordCursorToken,
    /// A record asserted a different source stream.
    #[error("source page record belongs to a different stream")]
    SourceMismatch,
    /// The loaded cursor belongs to another stream.
    #[error("cursor belongs to a different source stream")]
    CursorSourceMismatch,
    /// The source progress classification contradicts exact token equality.
    #[error("source cursor progress contradicts exact token equality")]
    ContradictoryProgress,
    /// An empty non-caught-up page made no source-classified progress.
    #[error("empty non-caught-up source page made no cursor progress")]
    StalledEmptyPage,
}

/// Validated 32-byte event identifier encoded as 64 hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EventId(String);

impl EventId {
    /// Creates a validated event identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, EventIdError> {
        let value = value.into();
        if value.len() != 64 {
            return Err(EventIdError::InvalidLength {
                actual: value.len(),
            });
        }
        hex::decode(&value).map_err(EventIdError::InvalidHex)?;
        Ok(Self(value))
    }

    /// Returns the unchanged hexadecimal identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EventId {
    type Error = EventIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<EventId> for String {
    fn from(value: EventId) -> Self {
        value.0
    }
}

/// Invalid event identifier supplied by a receipt adapter.
#[derive(Debug, Error)]
pub enum EventIdError {
    /// The hexadecimal representation did not contain exactly 64 characters.
    #[error("event ID must contain 64 hexadecimal characters, got {actual}")]
    InvalidLength {
        /// Actual character length.
        actual: usize,
    },
    /// The identifier contained a non-hexadecimal character.
    #[error("event ID is not hexadecimal: {0}")]
    InvalidHex(hex::FromHexError),
}

/// Application-owned durability and digest evidence for one sink observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptEvidence {
    /// Source copied from the exact delivered record.
    pub source: SourceStreamId,
    /// Cursor copied from the exact delivered record.
    pub cursor: OpaqueSourceCursorToken,
    /// Validated event ID copied from the exact delivered record.
    pub event_id: EventId,
    /// Stable application outcome derived from portable destination evidence.
    pub outcome: ReceiptOutcomeClass,
    /// Whether destination durability is positively established.
    pub durable: bool,
    /// Bounded immutable receipt evidence reference.
    pub receipt_digest: EvidenceRef,
}

impl ReceiptEvidence {
    /// Wraps a portable receipt with application-owned durability evidence.
    pub fn from_receipt(
        receipt: ReplicationReceipt,
        durable: bool,
        receipt_digest: EvidenceRef,
    ) -> Result<Self, ReceiptEvidenceError> {
        let outcome = match receipt.outcome {
            ReplicationIngestOutcome::Stored => ReceiptOutcomeClass::Stored,
            ReplicationIngestOutcome::Duplicate => ReceiptOutcomeClass::Duplicate,
            ReplicationIngestOutcome::Superseded => ReceiptOutcomeClass::Superseded,
            ReplicationIngestOutcome::Rejected { .. } => ReceiptOutcomeClass::Rejected,
        };
        Ok(Self {
            source: SourceStreamId::new(receipt.source.as_str())?,
            cursor: OpaqueSourceCursorToken::from_replication_cursor(&receipt.cursor)?,
            event_id: EventId::new(receipt.event_id)?,
            outcome,
            durable,
            receipt_digest,
        })
    }

    /// Creates ambiguous evidence bound to the exact delivered record.
    pub fn ambiguous(
        record: &ReplicationRecord,
        receipt_digest: EvidenceRef,
    ) -> Result<Self, ReceiptEvidenceError> {
        Ok(Self {
            source: SourceStreamId::new(record.source.as_str())?,
            cursor: OpaqueSourceCursorToken::from_replication_cursor(&record.cursor)?,
            event_id: EventId::new(record.event.id.to_hex())?,
            outcome: ReceiptOutcomeClass::Ambiguous,
            durable: false,
            receipt_digest,
        })
    }

    /// Returns whether this evidence permits checkpoint commitment.
    pub fn checkpoint_safe(&self) -> bool {
        self.durable
            && matches!(
                self.outcome,
                ReceiptOutcomeClass::Stored
                    | ReceiptOutcomeClass::Duplicate
                    | ReceiptOutcomeClass::Superseded
            )
    }

    /// Verifies that evidence is bound to the exact delivered record.
    pub fn validate_against(&self, record: &ReplicationRecord) -> bool {
        self.source.matches_replication_source(&record.source)
            && self.cursor.as_str() == record.cursor.as_str()
            && self.event_id.as_str() == record.event.id.to_hex()
    }

    /// Returns the stable outcome classification for summary counting.
    pub fn outcome_class(&self) -> ReceiptOutcomeClass {
        self.outcome
    }
}

/// Invalid application receipt evidence derived from a portable receipt.
#[derive(Debug, Error)]
pub enum ReceiptEvidenceError {
    /// Source or cursor identity was empty.
    #[error(transparent)]
    Identifier(#[from] IdentifierError),
    /// Event identifier did not satisfy the declared 64-hex contract.
    #[error(transparent)]
    EventId(#[from] EventIdError),
}

/// Stable application receipt class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptOutcomeClass {
    /// Event entered durable destination history.
    Stored,
    /// Event ID was already durably present.
    Duplicate,
    /// Event lost replacement ordering but is durably accounted for.
    Superseded,
    /// Destination policy or verification rejected the event.
    Rejected,
    /// No durable destination outcome is known.
    Ambiguous,
}

/// Counted destination evidence for one attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptSummary {
    /// Stored outcomes.
    pub stored: usize,
    /// Duplicate outcomes.
    pub duplicate: usize,
    /// Superseded outcomes.
    pub superseded: usize,
    /// Rejected outcomes.
    pub rejected: usize,
    /// Ambiguous outcomes.
    pub ambiguous: usize,
}

impl ReceiptSummary {
    /// Counts receipt evidence without changing its checkpoint classification.
    pub fn from_evidence(receipts: &[ReceiptEvidence]) -> Self {
        let mut summary = Self::default();
        for receipt in receipts {
            match receipt.outcome_class() {
                ReceiptOutcomeClass::Stored => summary.stored += 1,
                ReceiptOutcomeClass::Duplicate => summary.duplicate += 1,
                ReceiptOutcomeClass::Superseded => summary.superseded += 1,
                ReceiptOutcomeClass::Rejected => summary.rejected += 1,
                ReceiptOutcomeClass::Ambiguous => summary.ambiguous += 1,
            }
        }
        summary
    }
}

/// Terminal outcome classification recorded in continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOutcomeClassification {
    /// Source is caught up at the current committed cursor.
    CaughtUp,
    /// An empty page advanced filtered scan progress.
    ScanProgress,
    /// One or more exact selected records were durably replicated.
    Replicated,
    /// Principal Node authorization was not current.
    NodeUnauthorized,
    /// Current agreement was not ready.
    AgreementNotReady,
    /// Authenticated admitted transport was not ready.
    TransportNotReady,
    /// A required mechanical capability was missing.
    RequiredCapabilityMissing,
    /// Durable cursor load failed during evaluation.
    CursorLoadFailed,
    /// Peer transport was operationally unavailable during evaluation.
    TransportUnavailable,
    /// Current projection was unavailable.
    ProjectionUnavailable,
    /// Bounded source read was unavailable.
    SourceUnavailable,
    /// Source page validation failed.
    MalformedSourceBatch,
    /// A destination receipt was rejected.
    ReceiptRejected,
    /// A destination outcome was ambiguous or not durable.
    ReceiptAmbiguous,
    /// Transfer failed after it began.
    TransportFailed,
    /// Atomic cursor commitment was definitively rejected.
    CursorCommitFailed,
    /// Quiescence cancelled the attempt at a safe boundary.
    Cancelled,
}

impl From<BlockedClassification> for SyncOutcomeClassification {
    fn from(value: BlockedClassification) -> Self {
        match value {
            BlockedClassification::NodeUnauthorized => Self::NodeUnauthorized,
            BlockedClassification::AgreementNotReady => Self::AgreementNotReady,
            BlockedClassification::TransportNotReady => Self::TransportNotReady,
            BlockedClassification::RequiredCapabilityMissing => Self::RequiredCapabilityMissing,
        }
    }
}

impl From<EvaluationFailureClassification> for SyncOutcomeClassification {
    fn from(value: EvaluationFailureClassification) -> Self {
        match value {
            EvaluationFailureClassification::CursorLoadFailed => Self::CursorLoadFailed,
            EvaluationFailureClassification::TransportUnavailable => Self::TransportUnavailable,
            EvaluationFailureClassification::ProjectionUnavailable => Self::ProjectionUnavailable,
            EvaluationFailureClassification::SourceUnavailable => Self::SourceUnavailable,
            EvaluationFailureClassification::MalformedSourceBatch => Self::MalformedSourceBatch,
        }
    }
}

/// Immutable terminal attempt evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSyncSessionSummary {
    /// Principal Node that owns the attempt.
    pub principal_node_id: PrincipalNodeId,
    /// Immutable attempt identity.
    pub session_id: SyncSessionId,
    /// Adapter composition direction.
    pub direction: SyncDirection,
    /// Immutable source stream identity.
    pub source_stream_id: SourceStreamId,
    /// Timing trigger.
    pub trigger: SyncTrigger,
    /// Durable terminal lifecycle state.
    pub terminal_state: SyncState,
    /// Attempt start evidence.
    pub started_at: ClockSample,
    /// Attempt finish evidence.
    pub finished_at: ClockSample,
    /// Stable outcome classification.
    pub outcome: SyncOutcomeClassification,
    /// Number of selected records examined.
    pub records_examined: usize,
    /// Whether this summary committed a cursor atomically.
    pub cursor_committed: bool,
    /// Bounded evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
    /// Prior immutable attempt for a retry trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_session_id: Option<SyncSessionId>,
    /// Current agreement snapshot used by a ready attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement_snapshot_ref: Option<EvidenceRef>,
    /// Durable cursor before the attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_before: Option<SourceBoundCursor>,
    /// Durable cursor atomically committed with this summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_after: Option<SourceBoundCursor>,
    /// Destination outcome counts when records were delivered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_summary: Option<ReceiptSummary>,
}

impl TerminalSyncSessionSummary {
    /// Validates terminal-state, retry-lineage, and cursor consistency.
    pub fn validate(&self) -> Result<(), SummaryValidationError> {
        if !self.terminal_state.is_terminal() {
            return Err(SummaryValidationError::NonTerminalState);
        }
        let outcome_matches_state = match self.terminal_state {
            SyncState::Completed => matches!(
                self.outcome,
                SyncOutcomeClassification::CaughtUp
                    | SyncOutcomeClassification::ScanProgress
                    | SyncOutcomeClassification::Replicated
            ),
            SyncState::Blocked => matches!(
                self.outcome,
                SyncOutcomeClassification::NodeUnauthorized
                    | SyncOutcomeClassification::AgreementNotReady
                    | SyncOutcomeClassification::TransportNotReady
                    | SyncOutcomeClassification::RequiredCapabilityMissing
            ),
            SyncState::Failed => matches!(
                self.outcome,
                SyncOutcomeClassification::CursorLoadFailed
                    | SyncOutcomeClassification::TransportUnavailable
                    | SyncOutcomeClassification::ProjectionUnavailable
                    | SyncOutcomeClassification::SourceUnavailable
                    | SyncOutcomeClassification::MalformedSourceBatch
                    | SyncOutcomeClassification::ReceiptRejected
                    | SyncOutcomeClassification::ReceiptAmbiguous
                    | SyncOutcomeClassification::TransportFailed
                    | SyncOutcomeClassification::CursorCommitFailed
            ),
            SyncState::Cancelled => self.outcome == SyncOutcomeClassification::Cancelled,
            _ => false,
        };
        if !outcome_matches_state {
            return Err(SummaryValidationError::OutcomeStateMismatch);
        }
        if self.cursor_committed && self.terminal_state != SyncState::Completed {
            return Err(SummaryValidationError::CursorCommittedOutsideCompletion);
        }
        if matches!(
            self.outcome,
            SyncOutcomeClassification::ScanProgress | SyncOutcomeClassification::Replicated
        ) && !self.cursor_committed
        {
            return Err(SummaryValidationError::MissingRequiredCursorCommit);
        }
        if self.trigger == SyncTrigger::Retry && self.previous_session_id.is_none() {
            return Err(SummaryValidationError::MissingRetryLineage);
        }
        if self.trigger != SyncTrigger::Retry && self.previous_session_id.is_some() {
            return Err(SummaryValidationError::UnexpectedRetryLineage);
        }
        match (self.cursor_committed, self.cursor_after.as_ref()) {
            (true, None) => return Err(SummaryValidationError::MissingCommittedCursor),
            (false, Some(_)) => return Err(SummaryValidationError::UnexpectedCommittedCursor),
            _ => {}
        }
        for cursor in [self.cursor_before.as_ref(), self.cursor_after.as_ref()]
            .into_iter()
            .flatten()
        {
            if cursor.source_stream_id() != &self.source_stream_id {
                return Err(SummaryValidationError::CursorSourceMismatch);
            }
        }
        if self.finished_at.monotonic_tick < self.started_at.monotonic_tick {
            return Err(SummaryValidationError::ClockReversed);
        }
        Ok(())
    }
}

/// Invalid immutable terminal summary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SummaryValidationError {
    /// Summary state is not terminal.
    #[error("terminal summary contains a non-terminal state")]
    NonTerminalState,
    /// Retry summary lacks its immutable prior attempt.
    #[error("retry summary is missing previous_session_id")]
    MissingRetryLineage,
    /// Non-retry summary contains retry lineage.
    #[error("non-retry summary contains previous_session_id")]
    UnexpectedRetryLineage,
    /// Cursor commitment lacks cursor-after evidence.
    #[error("cursor_committed requires cursor_after")]
    MissingCommittedCursor,
    /// Summary claims no cursor commit but contains cursor-after evidence.
    #[error("cursor_after is forbidden when cursor_committed is false")]
    UnexpectedCommittedCursor,
    /// Cursor-before or cursor-after belongs to another source stream.
    #[error("summary cursor belongs to another source stream")]
    CursorSourceMismatch,
    /// Finish clock evidence precedes start evidence.
    #[error("finished_at precedes started_at")]
    ClockReversed,
    /// Terminal state and outcome classification contradict one another.
    #[error("terminal state and outcome classification are inconsistent")]
    OutcomeStateMismatch,
    /// A blocked, failed, or cancelled summary claimed cursor commitment.
    #[error("cursor commitment is permitted only for completed summaries")]
    CursorCommittedOutsideCompletion,
    /// Scan-progress or replicated completion omitted its cursor commitment.
    #[error("completion outcome requires a committed cursor")]
    MissingRequiredCursorCommit,
}

/// Immutable non-cursor terminal continuity candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalCommitCandidate {
    /// Exact terminal summary to persist.
    summary: TerminalSyncSessionSummary,
}

impl TerminalCommitCandidate {
    /// Creates a validated non-cursor terminal candidate.
    pub fn new(summary: TerminalSyncSessionSummary) -> Result<Self, CandidateValidationError> {
        summary.validate()?;
        if summary.cursor_committed || summary.cursor_after.is_some() {
            return Err(CandidateValidationError::TerminalCandidateMutatesCursor);
        }
        Ok(Self { summary })
    }

    /// Returns the exact immutable terminal summary.
    pub fn summary(&self) -> &TerminalSyncSessionSummary {
        &self.summary
    }

    /// Consumes the candidate and returns its exact summary.
    pub fn into_summary(self) -> TerminalSyncSessionSummary {
        self.summary
    }

    /// Revalidates candidate-specific invariants at a port boundary.
    pub fn validate(&self) -> Result<(), CandidateValidationError> {
        Self::new(self.summary.clone()).map(|_| ())
    }
}

/// Immutable atomic cursor-and-completion continuity candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedCommitCandidate {
    /// Expected durable cursor before compare-and-commit.
    expected: Option<SourceBoundCursor>,
    /// Exact source-issued cursor candidate.
    candidate: SourceBoundCursor,
    /// Exact application-owned receipt evidence; empty only for scan progress.
    receipts: Vec<ReceiptEvidence>,
    /// Exact completed summary committed atomically with the cursor.
    summary: TerminalSyncSessionSummary,
}

impl CompletedCommitCandidate {
    /// Creates a validated atomic cursor-and-completed-summary candidate.
    pub fn new(
        expected: Option<SourceBoundCursor>,
        candidate: SourceBoundCursor,
        receipts: Vec<ReceiptEvidence>,
        summary: TerminalSyncSessionSummary,
    ) -> Result<Self, CandidateValidationError> {
        let value = Self {
            expected,
            candidate,
            receipts,
            summary,
        };
        value.validate()?;
        Ok(value)
    }

    /// Revalidates atomic namespace, cursor, summary, and receipt invariants.
    pub fn validate(&self) -> Result<(), CandidateValidationError> {
        self.summary.validate()?;
        if self.summary.terminal_state != SyncState::Completed {
            return Err(CandidateValidationError::AtomicSummaryNotCompleted);
        }
        if !self.summary.cursor_committed
            || self.summary.cursor_after.as_ref() != Some(&self.candidate)
        {
            return Err(CandidateValidationError::AtomicCursorSummaryMismatch);
        }
        if self.summary.cursor_before != self.expected
            || self.candidate.source_stream_id() != &self.summary.source_stream_id
            || self
                .expected
                .as_ref()
                .is_some_and(|cursor| cursor.source_stream_id() != &self.summary.source_stream_id)
        {
            return Err(CandidateValidationError::AtomicNamespaceMismatch);
        }
        if self
            .receipts
            .iter()
            .any(|receipt| !receipt.checkpoint_safe())
        {
            return Err(CandidateValidationError::ReceiptNotCheckpointSafe);
        }
        if self
            .receipts
            .iter()
            .any(|receipt| receipt.source != self.summary.source_stream_id)
        {
            return Err(CandidateValidationError::ReceiptNamespaceMismatch);
        }
        if self.summary.records_examined != self.receipts.len() {
            return Err(CandidateValidationError::ReceiptCoverageMismatch);
        }
        let expected_receipt_summary = if self.receipts.is_empty() {
            None
        } else {
            Some(ReceiptSummary::from_evidence(&self.receipts))
        };
        if self.summary.receipt_summary != expected_receipt_summary {
            return Err(CandidateValidationError::ReceiptSummaryMismatch);
        }
        Ok(())
    }

    /// Returns the Principal Node continuity namespace.
    pub fn principal_node_id(&self) -> &PrincipalNodeId {
        &self.summary.principal_node_id
    }

    /// Returns the source-stream continuity namespace.
    pub fn source_stream_id(&self) -> &SourceStreamId {
        &self.summary.source_stream_id
    }

    /// Returns the direction-specific continuity namespace.
    pub fn direction(&self) -> SyncDirection {
        self.summary.direction
    }

    /// Returns the expected durable cursor.
    pub fn expected(&self) -> Option<&SourceBoundCursor> {
        self.expected.as_ref()
    }

    /// Returns the exact candidate cursor.
    pub fn candidate(&self) -> &SourceBoundCursor {
        &self.candidate
    }

    /// Returns the exact application-owned receipts.
    pub fn receipts(&self) -> &[ReceiptEvidence] {
        &self.receipts
    }

    /// Returns the exact immutable completed summary.
    pub fn summary(&self) -> &TerminalSyncSessionSummary {
        &self.summary
    }

    /// Consumes the candidate into its atomic parts.
    pub fn into_parts(
        self,
    ) -> (
        Option<SourceBoundCursor>,
        SourceBoundCursor,
        Vec<ReceiptEvidence>,
        TerminalSyncSessionSummary,
    ) {
        (self.expected, self.candidate, self.receipts, self.summary)
    }
}

/// Invalid immutable continuity candidate.
#[derive(Debug, Error)]
pub enum CandidateValidationError {
    /// The contained terminal summary was invalid.
    #[error(transparent)]
    Summary(#[from] SummaryValidationError),
    /// A terminal-only candidate attempted to mutate a cursor.
    #[error("terminal-only candidate must not commit a cursor")]
    TerminalCandidateMutatesCursor,
    /// An atomic candidate did not contain a completed summary.
    #[error("atomic candidate summary must be completed")]
    AtomicSummaryNotCompleted,
    /// Candidate cursor and completed summary cursor evidence differed.
    #[error("atomic candidate cursor differs from completed summary")]
    AtomicCursorSummaryMismatch,
    /// Principal, stream, direction, expected cursor, or candidate source differed.
    #[error("atomic candidate continuity namespace is inconsistent")]
    AtomicNamespaceMismatch,
    /// At least one selected record lacked receipt coverage.
    #[error("atomic candidate receipt coverage differs from records examined")]
    ReceiptCoverageMismatch,
    /// At least one receipt was not durable and checkpoint-safe.
    #[error("atomic candidate contains a receipt that is not checkpoint-safe")]
    ReceiptNotCheckpointSafe,
    /// At least one receipt belongs to another source-stream namespace.
    #[error("atomic candidate contains a receipt from another source stream")]
    ReceiptNamespaceMismatch,
    /// Counted receipt evidence contradicted the exact candidate receipts.
    #[error("atomic candidate receipt summary differs from exact receipts")]
    ReceiptSummaryMismatch,
}

/// Exact immutable continuity work retained after unavailable or ambiguous persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "candidate_kind",
    content = "exact_candidate",
    rename_all = "snake_case"
)]
pub enum ContinuityCandidate {
    /// Terminal summary without cursor mutation.
    TerminalSummary(TerminalCommitCandidate),
    /// Atomic cursor and completed summary.
    CompletedAtomic(CompletedCommitCandidate),
}

impl ContinuityCandidate {
    /// Returns the exact candidate session identity.
    pub fn session_id(&self) -> &SyncSessionId {
        match self {
            Self::TerminalSummary(candidate) => &candidate.summary().session_id,
            Self::CompletedAtomic(candidate) => &candidate.summary().session_id,
        }
    }

    /// Returns the Principal Node namespace carried by the candidate summary.
    pub fn principal_node_id(&self) -> &PrincipalNodeId {
        match self {
            Self::TerminalSummary(candidate) => &candidate.summary().principal_node_id,
            Self::CompletedAtomic(candidate) => candidate.principal_node_id(),
        }
    }

    /// Revalidates candidate-specific invariants after deserialization or retention.
    pub fn validate(&self) -> Result<(), CandidateValidationError> {
        match self {
            Self::TerminalSummary(candidate) => candidate.validate(),
            Self::CompletedAtomic(candidate) => candidate.validate(),
        }
    }

    /// Computes the deterministic SHA-256 digest of the canonical candidate wire object.
    pub fn canonical_digest(&self) -> Result<CandidateDigest, CandidateDigestError> {
        let encoded = serde_json::to_vec(self).map_err(CandidateDigestError::Serialize)?;
        let digest = Sha256::digest(encoded);
        CandidateDigest::new(format!("sha256:{}", hex::encode(digest)))
            .map_err(CandidateDigestError::Identifier)
    }
}

/// Typed same-session continuity result awaiting idempotent retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingContinuityCommit {
    /// Principal Node that owns the candidate.
    principal_node_id: PrincipalNodeId,
    /// Same immutable session identity used by every retry.
    session_id: SyncSessionId,
    /// Lifecycle state retained until persistence is acknowledged.
    prior_lifecycle_state: SyncState,
    /// Exact immutable candidate.
    exact_candidate: ContinuityCandidate,
    /// Canonical digest of the candidate.
    candidate_digest: CandidateDigest,
}

impl PendingContinuityCommit {
    /// Creates pending continuity while verifying candidate/session identity.
    pub fn new(
        principal_node_id: PrincipalNodeId,
        prior_lifecycle_state: SyncState,
        exact_candidate: ContinuityCandidate,
    ) -> Result<Self, CandidateDigestError> {
        exact_candidate
            .validate()
            .map_err(CandidateDigestError::Candidate)?;
        if prior_lifecycle_state.is_terminal() {
            return Err(CandidateDigestError::PriorStateIsTerminal);
        }
        if exact_candidate.principal_node_id() != &principal_node_id {
            return Err(CandidateDigestError::PrincipalMismatch);
        }
        let session_id = exact_candidate.session_id().clone();
        let candidate_digest = exact_candidate.canonical_digest()?;
        Ok(Self {
            principal_node_id,
            session_id,
            prior_lifecycle_state,
            exact_candidate,
            candidate_digest,
        })
    }

    /// Recomputes and verifies the immutable candidate digest.
    pub fn verify_digest(&self) -> Result<bool, CandidateDigestError> {
        self.validate()?;
        Ok(self.candidate_digest == self.exact_candidate.canonical_digest()?)
    }

    /// Revalidates retained namespace, state, candidate, and session invariants.
    pub fn validate(&self) -> Result<(), CandidateDigestError> {
        self.exact_candidate
            .validate()
            .map_err(CandidateDigestError::Candidate)?;
        if self.prior_lifecycle_state.is_terminal() {
            return Err(CandidateDigestError::PriorStateIsTerminal);
        }
        if self.exact_candidate.principal_node_id() != &self.principal_node_id {
            return Err(CandidateDigestError::PrincipalMismatch);
        }
        if self.exact_candidate.session_id() != &self.session_id {
            return Err(CandidateDigestError::SessionMismatch);
        }
        Ok(())
    }

    /// Returns the Principal Node continuity namespace.
    pub fn principal_node_id(&self) -> &PrincipalNodeId {
        &self.principal_node_id
    }

    /// Returns the immutable same-session identity.
    pub fn session_id(&self) -> &SyncSessionId {
        &self.session_id
    }

    /// Returns the nonterminal state retained until durable acknowledgement.
    pub fn prior_lifecycle_state(&self) -> SyncState {
        self.prior_lifecycle_state
    }

    /// Returns the exact immutable continuity candidate.
    pub fn exact_candidate(&self) -> &ContinuityCandidate {
        &self.exact_candidate
    }

    /// Returns the retained canonical candidate digest.
    pub fn candidate_digest(&self) -> &CandidateDigest {
        &self.candidate_digest
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "candidate_kind", rename_all = "snake_case", deny_unknown_fields)]
enum PendingContinuityCommitWire {
    TerminalSummary {
        principal_node_id: PrincipalNodeId,
        session_id: SyncSessionId,
        prior_lifecycle_state: SyncState,
        candidate_digest: CandidateDigest,
        exact_candidate: TerminalCommitCandidate,
    },
    CompletedAtomic {
        principal_node_id: PrincipalNodeId,
        session_id: SyncSessionId,
        prior_lifecycle_state: SyncState,
        candidate_digest: CandidateDigest,
        exact_candidate: CompletedCommitCandidate,
    },
}

impl Serialize for PendingContinuityCommit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = match &self.exact_candidate {
            ContinuityCandidate::TerminalSummary(candidate) => {
                PendingContinuityCommitWire::TerminalSummary {
                    principal_node_id: self.principal_node_id.clone(),
                    session_id: self.session_id.clone(),
                    prior_lifecycle_state: self.prior_lifecycle_state,
                    candidate_digest: self.candidate_digest.clone(),
                    exact_candidate: candidate.clone(),
                }
            }
            ContinuityCandidate::CompletedAtomic(candidate) => {
                PendingContinuityCommitWire::CompletedAtomic {
                    principal_node_id: self.principal_node_id.clone(),
                    session_id: self.session_id.clone(),
                    prior_lifecycle_state: self.prior_lifecycle_state,
                    candidate_digest: self.candidate_digest.clone(),
                    exact_candidate: candidate.clone(),
                }
            }
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PendingContinuityCommit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PendingContinuityCommitWire::deserialize(deserializer)?;
        let value = match wire {
            PendingContinuityCommitWire::TerminalSummary {
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                candidate_digest,
                exact_candidate,
            } => Self {
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                exact_candidate: ContinuityCandidate::TerminalSummary(exact_candidate),
                candidate_digest,
            },
            PendingContinuityCommitWire::CompletedAtomic {
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                candidate_digest,
                exact_candidate,
            } => Self {
                principal_node_id,
                session_id,
                prior_lifecycle_state,
                exact_candidate: ContinuityCandidate::CompletedAtomic(exact_candidate),
                candidate_digest,
            },
        };
        Ok(value)
    }
}

/// Failure to create or verify a canonical continuity candidate digest.
#[derive(Debug, Error)]
pub enum CandidateDigestError {
    /// Candidate serialization failed.
    #[error("continuity candidate serialization failed: {0}")]
    Serialize(serde_json::Error),
    /// Generated digest identifier was invalid.
    #[error(transparent)]
    Identifier(IdentifierError),
    /// Candidate-specific continuity invariants failed.
    #[error(transparent)]
    Candidate(CandidateValidationError),
    /// Pending continuity retained a terminal prior state.
    #[error("pending continuity prior state must be nonterminal")]
    PriorStateIsTerminal,
    /// Candidate and pending Principal Node namespaces differed.
    #[error("pending continuity principal does not match candidate")]
    PrincipalMismatch,
    /// Candidate and pending session identities differed.
    #[error("pending continuity session does not match candidate")]
    SessionMismatch,
}

/// Result of idempotently persisting a non-cursor terminal candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalCommitDisposition {
    /// Candidate was stored now.
    Stored,
    /// The exact candidate was already durable.
    AlreadyStoredSame,
}

/// Result of idempotently persisting an atomic completion candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletedCommitDisposition {
    /// Cursor and summary were committed now.
    Committed,
    /// The exact cursor, receipts, and summary were already committed.
    AlreadyCommittedSame,
}

/// Exact atomic continuity acknowledgement returned by the continuity port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedCommitResult {
    /// Whether the exact candidate committed now or was already durable.
    pub disposition: CompletedCommitDisposition,
    /// Exact durable cursor acknowledged by continuity.
    pub committed_cursor: SourceBoundCursor,
    /// Exact immutable completed summary acknowledged by continuity.
    pub committed_summary: TerminalSyncSessionSummary,
}

/// Successful or pending result of an application procedure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncProcedureResult {
    /// Terminal state is durable and may be exposed.
    Terminal {
        /// Immutable durable terminal summary.
        summary: Box<TerminalSyncSessionSummary>,
    },
    /// Exact continuity work must be retried for the same session.
    PendingContinuity {
        /// Immutable same-session candidate.
        pending: Box<PendingContinuityCommit>,
    },
}

/// Transfer failure after the session entered `transferring`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferFailure {
    /// Bounded failure evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Error returned by Principal Node continuity operations.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContinuityError {
    /// Persistence was unavailable and no outcome is known.
    #[error("continuity is unavailable")]
    Unavailable {
        /// Bounded adapter evidence.
        evidence_refs: Vec<EvidenceRef>,
    },
    /// Persistence may have happened but acknowledgement was ambiguous.
    #[error("continuity result is ambiguous")]
    Ambiguous {
        /// Bounded adapter evidence.
        evidence_refs: Vec<EvidenceRef>,
    },
    /// Expected cursor did not match durable continuity.
    #[error("cursor compare-and-commit conflict")]
    CursorCompareConflict {
        /// Bounded conflict evidence.
        evidence_refs: Vec<EvidenceRef>,
    },
    /// Candidate cursor belongs to another source.
    #[error("candidate cursor belongs to another source")]
    CursorSourceMismatch,
    /// Candidate contains a receipt that is not checkpoint-safe.
    #[error("candidate receipt is not checkpoint-safe")]
    ReceiptNotCheckpointSafe,
    /// Different content is already associated with the same session ID.
    #[error("conflicting continuity content for session ID")]
    ConflictingContent,
}

impl ContinuityError {
    /// Returns whether this error must produce typed pending continuity.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Unavailable { .. } | Self::Ambiguous { .. })
    }
}
