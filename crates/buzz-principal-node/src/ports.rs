//! Inward-owned ports required by the synchronization application service.

use std::future::Future;

use buzz_core::replication::ReplicationRecord;

use crate::types::{
    AuthenticatedTransportEvidence, ClockSample, CompletedCommitCandidate, CompletedCommitResult,
    ContinuityError, CurrentSyncPlan, EvaluationFailure, EventSelection, PlanEvaluation,
    PrincipalNodeId, ReceiptEvidence, SourceBoundCursor, SourcePage, SourceStreamId, SyncDirection,
    SyncSessionId, TerminalCommitCandidate, TerminalCommitDisposition, TerminalSyncSessionSummary,
    TransferFailure, TransportAuthentication,
};

/// Request for fresh peer authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAuthenticationRequest {
    /// Principal Node owning the attempt.
    pub principal_node_id: PrincipalNodeId,
    /// Immutable stream and predicate identity.
    pub source_stream_id: SourceStreamId,
    /// Adapter composition direction.
    pub direction: SyncDirection,
    /// Current attempt time evidence.
    pub observed_at: ClockSample,
}

/// Request for current authority, agreement, and declaration projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRequest {
    /// Principal Node owning the attempt.
    pub principal_node_id: PrincipalNodeId,
    /// Immutable stream and predicate identity.
    pub source_stream_id: SourceStreamId,
    /// Adapter composition direction.
    pub direction: SyncDirection,
    /// Fresh transport authentication result, including not-ready evidence.
    pub transport: TransportAuthentication,
    /// Current attempt time evidence.
    pub evaluated_at: ClockSample,
}

/// Request for exactly one bounded source page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePageRequest {
    /// Immutable stream and predicate identity.
    pub source_stream_id: SourceStreamId,
    /// Current durable cursor, when one exists.
    pub cursor_before: Option<SourceBoundCursor>,
    /// Current declaration-projected selection.
    pub selection: EventSelection,
    /// Maximum number of records returned.
    pub batch_limit: usize,
    /// Full fresh authenticated transport evidence for this attempt.
    pub transport_evidence: AuthenticatedTransportEvidence,
}

/// Request to ingest one exact portable event envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkIngestRequest {
    /// Exact unchanged portable record.
    pub record: ReplicationRecord,
    /// Current immutable synchronization plan.
    pub current_plan: CurrentSyncPlan,
    /// Full fresh authenticated transport evidence for this attempt.
    pub transport_evidence: AuthenticatedTransportEvidence,
}

/// Issues application-owned session identities before lifecycle work begins.
pub trait SyncSessionIdentityIssuer {
    /// Issues a fresh identity without a fallible runtime effect.
    fn issue_session_id(&self, principal_node_id: &PrincipalNodeId) -> SyncSessionId;
}

/// Supplies application clock evidence without a fallible runtime effect.
pub trait AttemptClock {
    /// Captures current wall and monotonic attempt evidence.
    fn now(&self) -> ClockSample;
}

/// Authenticates the currently reachable peer for the requested composition.
pub trait AuthenticatedPeerTransport {
    /// Performs fresh authentication or returns an operational evaluation failure.
    fn authenticate_peer(
        &self,
        request: PeerAuthenticationRequest,
    ) -> impl Future<Output = Result<TransportAuthentication, EvaluationFailure>>;
}

/// Re-evaluates current authority, agreement, declarations, and readiness.
pub trait CurrentSyncProjection {
    /// Produces a current ready plan or a current blocked classification.
    fn evaluate_current_plan(
        &self,
        request: ProjectionRequest,
    ) -> impl Future<Output = Result<PlanEvaluation, EvaluationFailure>>;
}

/// Reads exactly one bounded application-wrapped portable replication page.
pub trait ReplicationSource {
    /// Returns one page or granular source-unavailable evidence.
    fn read_bounded_page(
        &self,
        request: SourcePageRequest,
    ) -> impl Future<Output = Result<SourcePage, EvaluationFailure>>;
}

/// Ingests exact unchanged portable records and returns app-owned durability evidence.
pub trait ReplicationSink {
    /// Ingests one record without changing its signed event envelope.
    fn ingest_exact(
        &self,
        request: SinkIngestRequest,
    ) -> impl Future<Output = Result<ReceiptEvidence, TransferFailure>>;
}

/// Owns durable cursor and immutable session-summary continuity.
pub trait PrincipalNodeSyncContinuity {
    /// Loads the durable cursor for one immutable source stream.
    fn load_cursor(
        &self,
        principal_node_id: PrincipalNodeId,
        source_stream_id: SourceStreamId,
        direction: SyncDirection,
    ) -> impl Future<Output = Result<Option<SourceBoundCursor>, EvaluationFailure>>;

    /// Loads an immutable prior terminal session for explicit retry lineage.
    fn load_terminal_summary(
        &self,
        principal_node_id: PrincipalNodeId,
        session_id: SyncSessionId,
    ) -> impl Future<Output = Result<Option<TerminalSyncSessionSummary>, ContinuityError>>;

    /// Idempotently persists one exact terminal summary without cursor mutation.
    fn persist_terminal_summary(
        &self,
        candidate: TerminalCommitCandidate,
    ) -> impl Future<Output = Result<TerminalCommitDisposition, ContinuityError>>;

    /// Atomically and idempotently compare-commits cursor, receipts, and summary.
    fn commit_completed(
        &self,
        candidate: CompletedCommitCandidate,
    ) -> impl Future<Output = Result<CompletedCommitResult, ContinuityError>>;
}
