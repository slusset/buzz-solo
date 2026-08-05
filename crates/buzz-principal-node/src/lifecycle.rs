//! Explicit synchronization lifecycle and fail-closed transition validation.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::SyncState;

/// Evidence that must already be durable before a transition exposes a terminal fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitProof {
    /// The transition changes only in-memory procedure state.
    InMemory,
    /// The exact immutable terminal summary is durable.
    TerminalSummaryDurable,
    /// The exact cursor, receipts, and completed summary are durable atomically.
    AtomicCompletionDurable,
}

/// One of the thirteen allowed lifecycle transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTransition {
    /// `requested -> evaluating`.
    BeginEvaluation,
    /// `evaluating -> completed` after a durable no-work summary.
    NoWorkNeeded,
    /// `evaluating -> transferring`.
    BeginTransfer,
    /// `evaluating -> committing_cursor` for a validated empty advancing page.
    BeginCursorCommit,
    /// `evaluating -> blocked` after a durable blocked summary.
    RefuseTransfer,
    /// `evaluating -> failed` after a durable evaluation-failure summary.
    EvaluationFailure,
    /// `transferring -> awaiting_durable_receipts`.
    BatchDelivered,
    /// `transferring -> failed` after a durable transfer-failure summary.
    TransportFailure,
    /// `awaiting_durable_receipts -> committing_cursor`.
    ReceiptsCheckpointSafe,
    /// `awaiting_durable_receipts -> failed` after a durable receipt-failure summary.
    ReceiptsNotCheckpointSafe,
    /// `committing_cursor -> completed` after atomic continuity commitment.
    CursorDurable,
    /// `committing_cursor -> failed` after a durable cursor-write-failure summary.
    CursorWriteFailed,
    /// `requested|evaluating -> cancelled` after a durable cancellation summary.
    CancelAtSafeBoundary,
}

/// A named forbidden transition from the lifecycle contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidTransition {
    /// A wake signal attempted to bypass evaluation.
    WakeDirectlyToTransfer,
    /// A host attempted to commit continuity independently.
    HostCommitCursor,
    /// Rejected destination evidence attempted to advance a cursor.
    RejectedReceiptToCursorCommit,
    /// A cursor was reused for another source stream.
    CrossSourceCursorReuse,
    /// Transfer attempted to rewrite an event envelope.
    TransferRewrittenEvent,
    /// A host scheduler tried to reconstruct retry lineage.
    RetryFromHostSchedulerHistory,
    /// Retry attempted to mutate a terminal attempt.
    MutateTerminalAttemptForRetry,
    /// A terminal state was exposed before its summary was durable.
    ExposeTerminalWithoutDurableSummary,
    /// Cursor commitment omitted the completed summary.
    CommitCursorWithoutCompletedSummary,
    /// Cancellation was attempted while transferring.
    CancelFromTransferring,
    /// Cancellation was attempted while classifying receipts.
    CancelFromAwaitingDurableReceipts,
    /// Cancellation was attempted while committing continuity.
    CancelFromCommittingCursor,
}

impl InvalidTransition {
    /// Returns the stable contract reason for refusing this transition.
    pub fn reason(self) -> InvalidTransitionReason {
        match self {
            Self::WakeDirectlyToTransfer => InvalidTransitionReason::EvaluationRequired,
            Self::HostCommitCursor => InvalidTransitionReason::PrincipalNodeOwnsCursorCommit,
            Self::RejectedReceiptToCursorCommit => {
                InvalidTransitionReason::ReceiptNotCheckpointSafe
            }
            Self::CrossSourceCursorReuse => InvalidTransitionReason::CursorSourceMismatch,
            Self::TransferRewrittenEvent => InvalidTransitionReason::ExactEnvelopeRequired,
            Self::RetryFromHostSchedulerHistory => {
                InvalidTransitionReason::PrincipalNodeEvidenceRequired
            }
            Self::MutateTerminalAttemptForRetry => {
                InvalidTransitionReason::TerminalSessionImmutable
            }
            Self::ExposeTerminalWithoutDurableSummary => {
                InvalidTransitionReason::DurableTerminalSummaryRequired
            }
            Self::CommitCursorWithoutCompletedSummary => {
                InvalidTransitionReason::AtomicContinuityCommitRequired
            }
            Self::CancelFromTransferring
            | Self::CancelFromAwaitingDurableReceipts
            | Self::CancelFromCommittingCursor => {
                InvalidTransitionReason::UnsafeCancellationBoundary
            }
        }
    }
}

/// Stable reason exposed when a named invalid transition is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidTransitionReason {
    /// Current evaluation cannot be bypassed.
    EvaluationRequired,
    /// Principal Node application policy owns cursor commitment.
    PrincipalNodeOwnsCursorCommit,
    /// Destination evidence is not checkpoint-safe.
    ReceiptNotCheckpointSafe,
    /// Cursor and source namespaces differ.
    CursorSourceMismatch,
    /// A signed event envelope would be changed.
    ExactEnvelopeRequired,
    /// Retry lineage requires Principal Node continuity evidence.
    PrincipalNodeEvidenceRequired,
    /// Terminal session history is immutable.
    TerminalSessionImmutable,
    /// Terminal exposure requires a durable immutable summary.
    DurableTerminalSummaryRequired,
    /// Cursor and completed summary require one atomic continuity operation.
    AtomicContinuityCommitRequired,
    /// Cancellation was requested after checkpoint-unsafe work began.
    UnsafeCancellationBoundary,
}

/// Stateful validator for one synchronization attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncLifecycle {
    state: SyncState,
}

impl SyncLifecycle {
    /// Creates a lifecycle in the only initial state.
    pub fn requested() -> Self {
        Self {
            state: SyncState::Requested,
        }
    }

    /// Reconstitutes a non-terminal state retained by pending continuity.
    pub fn from_pending(state: SyncState) -> Result<Self, LifecycleError> {
        if state.is_terminal() {
            return Err(LifecycleError::PendingStateIsTerminal { state });
        }
        Ok(Self { state })
    }

    /// Returns the current procedure state.
    pub fn state(&self) -> SyncState {
        self.state
    }

    /// Applies an allowed transition only when its source and durability proof match.
    pub fn apply(
        &mut self,
        transition: SyncTransition,
        proof: CommitProof,
    ) -> Result<SyncState, LifecycleError> {
        let (sources, target, required_proof): (&[SyncState], SyncState, CommitProof) =
            match transition {
                SyncTransition::BeginEvaluation => (
                    &[SyncState::Requested],
                    SyncState::Evaluating,
                    CommitProof::InMemory,
                ),
                SyncTransition::NoWorkNeeded => (
                    &[SyncState::Evaluating],
                    SyncState::Completed,
                    CommitProof::TerminalSummaryDurable,
                ),
                SyncTransition::BeginTransfer => (
                    &[SyncState::Evaluating],
                    SyncState::Transferring,
                    CommitProof::InMemory,
                ),
                SyncTransition::BeginCursorCommit => (
                    &[SyncState::Evaluating],
                    SyncState::CommittingCursor,
                    CommitProof::InMemory,
                ),
                SyncTransition::RefuseTransfer => (
                    &[SyncState::Evaluating],
                    SyncState::Blocked,
                    CommitProof::TerminalSummaryDurable,
                ),
                SyncTransition::EvaluationFailure => (
                    &[SyncState::Evaluating],
                    SyncState::Failed,
                    CommitProof::TerminalSummaryDurable,
                ),
                SyncTransition::BatchDelivered => (
                    &[SyncState::Transferring],
                    SyncState::AwaitingDurableReceipts,
                    CommitProof::InMemory,
                ),
                SyncTransition::TransportFailure => (
                    &[SyncState::Transferring],
                    SyncState::Failed,
                    CommitProof::TerminalSummaryDurable,
                ),
                SyncTransition::ReceiptsCheckpointSafe => (
                    &[SyncState::AwaitingDurableReceipts],
                    SyncState::CommittingCursor,
                    CommitProof::InMemory,
                ),
                SyncTransition::ReceiptsNotCheckpointSafe => (
                    &[SyncState::AwaitingDurableReceipts],
                    SyncState::Failed,
                    CommitProof::TerminalSummaryDurable,
                ),
                SyncTransition::CursorDurable => (
                    &[SyncState::CommittingCursor],
                    SyncState::Completed,
                    CommitProof::AtomicCompletionDurable,
                ),
                SyncTransition::CursorWriteFailed => (
                    &[SyncState::CommittingCursor],
                    SyncState::Failed,
                    CommitProof::TerminalSummaryDurable,
                ),
                SyncTransition::CancelAtSafeBoundary => (
                    &[SyncState::Requested, SyncState::Evaluating],
                    SyncState::Cancelled,
                    CommitProof::TerminalSummaryDurable,
                ),
            };

        if !sources.contains(&self.state) {
            return Err(LifecycleError::InvalidSource {
                transition,
                state: self.state,
            });
        }
        if proof != required_proof {
            return Err(LifecycleError::InsufficientCommitProof {
                transition,
                required: required_proof,
                provided: proof,
            });
        }
        self.state = target;
        Ok(target)
    }

    /// Rejects a named forbidden transition without mutating the lifecycle.
    pub fn reject(&self, transition: InvalidTransition) -> LifecycleError {
        LifecycleError::Forbidden {
            transition,
            reason: transition.reason(),
            state: self.state,
        }
    }
}

impl Default for SyncLifecycle {
    fn default() -> Self {
        Self::requested()
    }
}

/// Fail-closed lifecycle validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LifecycleError {
    /// An allowed transition was applied from a disallowed source state.
    #[error("transition {transition:?} is invalid from {state:?}")]
    InvalidSource {
        /// Attempted transition.
        transition: SyncTransition,
        /// Actual source state.
        state: SyncState,
    },
    /// A transition was attempted without its required durability acknowledgement.
    #[error("transition {transition:?} requires {required:?}, got {provided:?}")]
    InsufficientCommitProof {
        /// Attempted transition.
        transition: SyncTransition,
        /// Required proof.
        required: CommitProof,
        /// Supplied proof.
        provided: CommitProof,
    },
    /// Pending continuity cannot retain a terminal state.
    #[error("pending continuity cannot retain terminal state {state:?}")]
    PendingStateIsTerminal {
        /// Invalid retained state.
        state: SyncState,
    },
    /// A named invalid transition was rejected.
    #[error("forbidden transition {transition:?} rejected from {state:?}")]
    Forbidden {
        /// Forbidden contract transition.
        transition: InvalidTransition,
        /// Stable contract refusal reason.
        reason: InvalidTransitionReason,
        /// State preserved by the rejection.
        state: SyncState,
    },
}
