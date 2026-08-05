//! Single Principal-Node synchronization application procedure.

use thiserror::Error;

use crate::lifecycle::{CommitProof, LifecycleError, SyncLifecycle, SyncTransition};
use crate::ports::{
    AttemptClock, AuthenticatedPeerTransport, CurrentSyncProjection, PeerAuthenticationRequest,
    PrincipalNodeSyncContinuity, ProjectionRequest, ReplicationSink, ReplicationSource,
    SinkIngestRequest, SourcePageRequest, SyncSessionIdentityIssuer,
};
use crate::types::{
    AuthenticatedTransportEvidence, BlockedSyncPlan, CandidateDigestError, ClockSample,
    CompletedCommitCandidate, CompletedCommitResult, ContinuityCandidate, ContinuityError,
    CurrentSyncPlan, EvaluationFailure, EvaluationFailureClassification, PendingContinuityCommit,
    PlanEvaluation, ReceiptOutcomeClass, ReceiptSummary, RequestValidationError, SourceBoundCursor,
    SourceCursorProgress, SourcePage, SourcePageError, SyncOutcomeClassification,
    SyncProcedureResult, SyncRequest, SyncSessionId, SyncState, TerminalCommitCandidate,
    TerminalSyncSessionSummary, TransportAuthentication,
};

/// A prepared immutable attempt at a cancellable safe boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSyncSession {
    request: SyncRequest,
    session_id: SyncSessionId,
    started_at: ClockSample,
    lifecycle: SyncLifecycle,
}

impl ActiveSyncSession {
    /// Returns the immutable session identity.
    pub fn session_id(&self) -> &SyncSessionId {
        &self.session_id
    }

    /// Returns the current in-memory lifecycle state.
    pub fn state(&self) -> SyncState {
        self.lifecycle.state()
    }

    /// Returns the validated authority-free request.
    pub fn request(&self) -> &SyncRequest {
        &self.request
    }
}

/// Technology-neutral synchronization application service.
pub struct PrincipalNodeSyncService<I, C, T, P, S, D, K> {
    identity_issuer: I,
    clock: C,
    transport: T,
    projection: P,
    source: S,
    sink: D,
    continuity: K,
}

impl<I, C, T, P, S, D, K> PrincipalNodeSyncService<I, C, T, P, S, D, K>
where
    I: SyncSessionIdentityIssuer,
    C: AttemptClock,
    T: AuthenticatedPeerTransport,
    P: CurrentSyncProjection,
    S: ReplicationSource,
    D: ReplicationSink,
    K: PrincipalNodeSyncContinuity,
{
    /// Composes all mandatory application prerequisites and inward ports.
    pub fn new(
        identity_issuer: I,
        clock: C,
        transport: T,
        projection: P,
        source: S,
        sink: D,
        continuity: K,
    ) -> Self {
        Self {
            identity_issuer,
            clock,
            transport,
            projection,
            source,
            sink,
            continuity,
        }
    }

    /// Validates a request and creates a requested session without evaluating authority.
    pub async fn prepare_sync(
        &self,
        request: SyncRequest,
    ) -> Result<ActiveSyncSession, SyncServiceError> {
        request.validate()?;
        self.validate_retry_lineage(&request).await?;

        let session_id = self
            .identity_issuer
            .issue_session_id(&request.principal_node_id);
        if request.previous_session_id.as_ref() == Some(&session_id) {
            return Err(SyncServiceError::IdentityIssuerReusedPriorSession);
        }
        Ok(ActiveSyncSession {
            request,
            session_id,
            started_at: self.clock.now(),
            lifecycle: SyncLifecycle::requested(),
        })
    }

    /// Runs all triggers and both directions through the same bounded procedure.
    pub async fn request_sync(
        &self,
        request: SyncRequest,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        let session = self.prepare_sync(request).await?;
        self.evaluate_prepared(session).await
    }

    /// Enters evaluation through the shared lifecycle, preserving a cancellation boundary.
    pub fn begin_evaluation(
        &self,
        session: &mut ActiveSyncSession,
    ) -> Result<(), SyncServiceError> {
        session
            .lifecycle
            .apply(SyncTransition::BeginEvaluation, CommitProof::InMemory)?;
        Ok(())
    }

    /// Evaluates one prepared session and reads at most one bounded source page.
    pub async fn evaluate_prepared(
        &self,
        mut session: ActiveSyncSession,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        if session.state() == SyncState::Requested {
            self.begin_evaluation(&mut session)?;
        } else if session.state() != SyncState::Evaluating {
            return Err(SyncServiceError::PreparedSessionNotEvaluating {
                state: session.state(),
            });
        }

        let cursor_before = match self
            .continuity
            .load_cursor(
                session.request.principal_node_id.clone(),
                session.request.source_stream_id.clone(),
                session.request.direction,
            )
            .await
        {
            Ok(cursor) => cursor,
            Err(failure) => {
                let failure = EvaluationFailure::new(
                    EvaluationFailureClassification::CursorLoadFailed,
                    failure.evidence_refs,
                );
                return self
                    .commit_evaluation_failure(&mut session, None, None, failure)
                    .await;
            }
        };
        if cursor_before
            .as_ref()
            .is_some_and(|cursor| cursor.source_stream_id() != &session.request.source_stream_id)
        {
            let failure = EvaluationFailure::new(
                EvaluationFailureClassification::CursorLoadFailed,
                Vec::new(),
            );
            return self
                .commit_evaluation_failure(&mut session, None, None, failure)
                .await;
        }

        let evaluated_at = self.clock.now();
        let authentication = match self
            .transport
            .authenticate_peer(PeerAuthenticationRequest {
                principal_node_id: session.request.principal_node_id.clone(),
                source_stream_id: session.request.source_stream_id.clone(),
                direction: session.request.direction,
                observed_at: evaluated_at.clone(),
            })
            .await
        {
            Ok(authentication) => authentication,
            Err(failure) => {
                let failure = EvaluationFailure::new(
                    EvaluationFailureClassification::TransportUnavailable,
                    failure.evidence_refs,
                );
                return self
                    .commit_evaluation_failure(&mut session, cursor_before, None, failure)
                    .await;
            }
        };
        if !authentication.matches_request(&session.request, &evaluated_at) {
            let failure = EvaluationFailure::new(
                EvaluationFailureClassification::TransportUnavailable,
                authentication.evidence_refs(),
            );
            return self
                .commit_evaluation_failure(&mut session, cursor_before, None, failure)
                .await;
        }

        let plan_evaluation = match self
            .projection
            .evaluate_current_plan(ProjectionRequest {
                principal_node_id: session.request.principal_node_id.clone(),
                source_stream_id: session.request.source_stream_id.clone(),
                direction: session.request.direction,
                transport: authentication.clone(),
                evaluated_at: evaluated_at.clone(),
            })
            .await
        {
            Ok(evaluation) => evaluation,
            Err(failure) => {
                let failure = EvaluationFailure::new(
                    EvaluationFailureClassification::ProjectionUnavailable,
                    failure.evidence_refs,
                );
                return self
                    .commit_evaluation_failure(&mut session, cursor_before, None, failure)
                    .await;
            }
        };

        let plan = match plan_evaluation {
            PlanEvaluation::Blocked { plan } => {
                if !plan.matches_request(&session.request, &evaluated_at) {
                    let failure = EvaluationFailure::new(
                        EvaluationFailureClassification::ProjectionUnavailable,
                        plan.evidence_refs.clone(),
                    );
                    return self
                        .commit_evaluation_failure(&mut session, cursor_before, None, failure)
                        .await;
                }
                return self
                    .commit_blocked(&mut session, cursor_before, *plan)
                    .await;
            }
            PlanEvaluation::Ready { plan } => *plan,
        };
        if !plan.matches_request_and_authentication(
            &session.request,
            &authentication,
            &evaluated_at,
        ) {
            let failure = EvaluationFailure::new(
                EvaluationFailureClassification::ProjectionUnavailable,
                current_plan_evidence_refs(&plan),
            );
            return self
                .commit_evaluation_failure(&mut session, cursor_before, None, failure)
                .await;
        }
        let transport_evidence = match authentication {
            TransportAuthentication::Authenticated { evidence } => evidence,
            TransportAuthentication::NotReady { evidence_refs } => {
                let failure = EvaluationFailure::new(
                    EvaluationFailureClassification::ProjectionUnavailable,
                    evidence_refs,
                );
                return self
                    .commit_evaluation_failure(
                        &mut session,
                        cursor_before,
                        Some(plan.agreement_snapshot_ref.clone()),
                        failure,
                    )
                    .await;
            }
        };
        if plan.batch_limit == 0 {
            let failure = EvaluationFailure::new(
                EvaluationFailureClassification::ProjectionUnavailable,
                current_plan_evidence_refs(&plan),
            );
            return self
                .commit_evaluation_failure(
                    &mut session,
                    cursor_before,
                    Some(plan.agreement_snapshot_ref.clone()),
                    failure,
                )
                .await;
        }

        let page = match self
            .source
            .read_bounded_page(SourcePageRequest {
                source_stream_id: session.request.source_stream_id.clone(),
                cursor_before: cursor_before.clone(),
                selection: plan.selection.clone(),
                batch_limit: plan.batch_limit,
                transport_evidence: transport_evidence.clone(),
            })
            .await
        {
            Ok(page) => page,
            Err(failure) => {
                let classification = if failure.classification
                    == EvaluationFailureClassification::MalformedSourceBatch
                {
                    EvaluationFailureClassification::MalformedSourceBatch
                } else {
                    EvaluationFailureClassification::SourceUnavailable
                };
                let failure = EvaluationFailure::new(
                    classification,
                    ready_evidence_refs(&plan, failure.evidence_refs),
                );
                return self
                    .commit_evaluation_failure(
                        &mut session,
                        cursor_before,
                        Some(plan.agreement_snapshot_ref.clone()),
                        failure,
                    )
                    .await;
            }
        };
        if page
            .validate(
                &session.request.source_stream_id,
                cursor_before.as_ref(),
                plan.batch_limit,
            )
            .is_err()
        {
            let failure = EvaluationFailure::new(
                EvaluationFailureClassification::MalformedSourceBatch,
                ready_evidence_refs(&plan, Vec::new()),
            );
            return self
                .commit_evaluation_failure(
                    &mut session,
                    cursor_before,
                    Some(plan.agreement_snapshot_ref.clone()),
                    failure,
                )
                .await;
        }

        if page.batch.records.is_empty() {
            return self
                .complete_empty_page(&mut session, cursor_before, plan, page)
                .await;
        }
        self.transfer_page(&mut session, cursor_before, plan, transport_evidence, page)
            .await
    }

    /// Persists a cancellation only at requested or evaluating safe boundaries.
    pub async fn cancel_at_safe_boundary(
        &self,
        mut session: ActiveSyncSession,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        if !matches!(
            session.state(),
            SyncState::Requested | SyncState::Evaluating
        ) {
            return Err(SyncServiceError::UnsafeCancellationBoundary {
                state: session.state(),
            });
        }
        let summary = self.summary(
            &session,
            SyncState::Cancelled,
            SyncOutcomeClassification::Cancelled,
            0,
            false,
            Vec::new(),
            None,
            None,
            None,
            None,
        )?;
        self.persist_terminal(&mut session, summary, SyncTransition::CancelAtSafeBoundary)
            .await
    }

    /// Resubmits an exact pending candidate without issuing a new session identity.
    pub async fn retry_pending_continuity(
        &self,
        pending: PendingContinuityCommit,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        pending.validate()?;
        if !pending.verify_digest()? {
            return Err(SyncServiceError::PendingCandidateDigestMismatch);
        }
        if pending.session_id() != pending.exact_candidate().session_id() {
            return Err(SyncServiceError::PendingCandidateSessionMismatch);
        }

        let prior_state = pending.prior_lifecycle_state();
        let mut lifecycle = SyncLifecycle::from_pending(prior_state)?;
        match pending.exact_candidate().clone() {
            ContinuityCandidate::TerminalSummary(candidate) => {
                let transition =
                    transition_for_terminal(prior_state, candidate.summary().terminal_state)?;
                match self
                    .continuity
                    .persist_terminal_summary(candidate.clone())
                    .await
                {
                    Ok(_) => {
                        lifecycle.apply(transition, CommitProof::TerminalSummaryDurable)?;
                        Ok(SyncProcedureResult::Terminal {
                            summary: Box::new(candidate.into_summary()),
                        })
                    }
                    Err(error) if error.is_pending() => {
                        Ok(SyncProcedureResult::PendingContinuity {
                            pending: Box::new(pending),
                        })
                    }
                    Err(error) => Err(SyncServiceError::Continuity(error)),
                }
            }
            ContinuityCandidate::CompletedAtomic(candidate) => {
                candidate.validate()?;
                if prior_state != SyncState::CommittingCursor
                    || candidate.summary().terminal_state != SyncState::Completed
                {
                    return Err(SyncServiceError::InvalidPendingTransition);
                }
                match self.continuity.commit_completed(candidate.clone()).await {
                    Ok(result) => {
                        validate_completed_acknowledgement(&candidate, &result)?;
                        lifecycle.apply(
                            SyncTransition::CursorDurable,
                            CommitProof::AtomicCompletionDurable,
                        )?;
                        Ok(SyncProcedureResult::Terminal {
                            summary: Box::new(result.committed_summary),
                        })
                    }
                    Err(error) if error.is_pending() => {
                        Ok(SyncProcedureResult::PendingContinuity {
                            pending: Box::new(pending),
                        })
                    }
                    Err(error) => Err(SyncServiceError::Continuity(error)),
                }
            }
        }
    }

    async fn validate_retry_lineage(&self, request: &SyncRequest) -> Result<(), SyncServiceError> {
        let Some(previous_session_id) = request.previous_session_id.clone() else {
            return Ok(());
        };
        let previous = self
            .continuity
            .load_terminal_summary(
                request.principal_node_id.clone(),
                previous_session_id.clone(),
            )
            .await
            .map_err(SyncServiceError::RetryLineageUnavailable)?
            .ok_or(SyncServiceError::RetrySessionNotFound)?;
        if previous.validate().is_err()
            || previous.session_id != previous_session_id
            || previous.principal_node_id != request.principal_node_id
            || !matches!(
                previous.terminal_state,
                SyncState::Failed | SyncState::Blocked
            )
        {
            return Err(SyncServiceError::PriorSessionNotRetryable);
        }
        Ok(())
    }

    async fn complete_empty_page(
        &self,
        session: &mut ActiveSyncSession,
        cursor_before: Option<SourceBoundCursor>,
        plan: CurrentSyncPlan,
        page: SourcePage,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        if page.cursor_progress == SourceCursorProgress::Unchanged {
            let summary = self.summary(
                session,
                SyncState::Completed,
                SyncOutcomeClassification::CaughtUp,
                0,
                false,
                ready_evidence_refs(&plan, Vec::new()),
                Some(plan.agreement_snapshot_ref),
                cursor_before,
                None,
                None,
            )?;
            return self
                .persist_terminal(session, summary, SyncTransition::NoWorkNeeded)
                .await;
        }

        session
            .lifecycle
            .apply(SyncTransition::BeginCursorCommit, CommitProof::InMemory)?;
        let candidate_cursor = page.candidate_cursor(session.request.source_stream_id.clone())?;
        let outcome = if page.batch.caught_up {
            SyncOutcomeClassification::CaughtUp
        } else {
            SyncOutcomeClassification::ScanProgress
        };
        let summary = self.summary(
            session,
            SyncState::Completed,
            outcome,
            0,
            true,
            ready_evidence_refs(&plan, Vec::new()),
            Some(plan.agreement_snapshot_ref),
            cursor_before.clone(),
            Some(candidate_cursor.clone()),
            None,
        )?;
        let candidate =
            CompletedCommitCandidate::new(cursor_before, candidate_cursor, Vec::new(), summary)?;
        self.commit_completed(session, candidate).await
    }

    async fn transfer_page(
        &self,
        session: &mut ActiveSyncSession,
        cursor_before: Option<SourceBoundCursor>,
        plan: CurrentSyncPlan,
        transport_evidence: AuthenticatedTransportEvidence,
        page: SourcePage,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        session
            .lifecycle
            .apply(SyncTransition::BeginTransfer, CommitProof::InMemory)?;
        let records = page.batch.records.clone();
        let mut receipts = Vec::with_capacity(records.len());
        for record in &records {
            match self
                .sink
                .ingest_exact(SinkIngestRequest {
                    record: record.clone(),
                    current_plan: plan.clone(),
                    transport_evidence: transport_evidence.clone(),
                })
                .await
            {
                Ok(receipt) => receipts.push(receipt),
                Err(failure) => {
                    let summary = self.summary(
                        session,
                        SyncState::Failed,
                        SyncOutcomeClassification::TransportFailed,
                        records.len(),
                        false,
                        ready_evidence_refs(&plan, failure.evidence_refs),
                        Some(plan.agreement_snapshot_ref),
                        cursor_before,
                        None,
                        Some(ReceiptSummary::from_evidence(&receipts)),
                    )?;
                    return self
                        .persist_terminal(session, summary, SyncTransition::TransportFailure)
                        .await;
                }
            }
        }
        session
            .lifecycle
            .apply(SyncTransition::BatchDelivered, CommitProof::InMemory)?;

        let checkpoint_safe = receipts.len() == records.len()
            && receipts
                .iter()
                .zip(records.iter())
                .all(|(receipt, record)| {
                    receipt.validate_against(record) && receipt.checkpoint_safe()
                });
        if !checkpoint_safe {
            let outcome = if receipts
                .iter()
                .zip(records.iter())
                .any(|(receipt, record)| {
                    receipt.outcome_class() == ReceiptOutcomeClass::Rejected
                        && receipt.validate_against(record)
                }) {
                SyncOutcomeClassification::ReceiptRejected
            } else {
                SyncOutcomeClassification::ReceiptAmbiguous
            };
            let receipt_refs = receipts
                .iter()
                .map(|receipt| receipt.receipt_digest.clone())
                .collect();
            let evidence_refs = ready_evidence_refs(&plan, receipt_refs);
            let summary = self.summary(
                session,
                SyncState::Failed,
                outcome,
                records.len(),
                false,
                evidence_refs,
                Some(plan.agreement_snapshot_ref),
                cursor_before,
                None,
                Some(ReceiptSummary::from_evidence(&receipts)),
            )?;
            return self
                .persist_terminal(session, summary, SyncTransition::ReceiptsNotCheckpointSafe)
                .await;
        }

        session.lifecycle.apply(
            SyncTransition::ReceiptsCheckpointSafe,
            CommitProof::InMemory,
        )?;
        let candidate_cursor = page.candidate_cursor(session.request.source_stream_id.clone())?;
        let receipt_refs = receipts
            .iter()
            .map(|receipt| receipt.receipt_digest.clone())
            .collect();
        let evidence_refs = ready_evidence_refs(&plan, receipt_refs);
        let summary = self.summary(
            session,
            SyncState::Completed,
            SyncOutcomeClassification::Replicated,
            records.len(),
            true,
            evidence_refs,
            Some(plan.agreement_snapshot_ref),
            cursor_before.clone(),
            Some(candidate_cursor.clone()),
            Some(ReceiptSummary::from_evidence(&receipts)),
        )?;
        let candidate =
            CompletedCommitCandidate::new(cursor_before, candidate_cursor, receipts, summary)?;
        self.commit_completed(session, candidate).await
    }

    async fn commit_evaluation_failure(
        &self,
        session: &mut ActiveSyncSession,
        cursor_before: Option<SourceBoundCursor>,
        agreement_snapshot_ref: Option<crate::types::EvidenceRef>,
        failure: EvaluationFailure,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        let summary = self.summary(
            session,
            SyncState::Failed,
            failure.classification.into(),
            0,
            false,
            failure.evidence_refs,
            agreement_snapshot_ref,
            cursor_before,
            None,
            None,
        )?;
        self.persist_terminal(session, summary, SyncTransition::EvaluationFailure)
            .await
    }

    async fn commit_blocked(
        &self,
        session: &mut ActiveSyncSession,
        cursor_before: Option<SourceBoundCursor>,
        plan: BlockedSyncPlan,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        let summary = self.summary(
            session,
            SyncState::Blocked,
            plan.classification.into(),
            0,
            false,
            plan.evidence_refs,
            None,
            cursor_before,
            None,
            None,
        )?;
        self.persist_terminal(session, summary, SyncTransition::RefuseTransfer)
            .await
    }

    async fn persist_terminal(
        &self,
        session: &mut ActiveSyncSession,
        summary: TerminalSyncSessionSummary,
        transition: SyncTransition,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        summary.validate()?;
        let candidate = TerminalCommitCandidate::new(summary.clone())?;
        match self
            .continuity
            .persist_terminal_summary(candidate.clone())
            .await
        {
            Ok(_) => {
                session
                    .lifecycle
                    .apply(transition, CommitProof::TerminalSummaryDurable)?;
                Ok(SyncProcedureResult::Terminal {
                    summary: Box::new(summary),
                })
            }
            Err(error) if error.is_pending() => {
                let pending = PendingContinuityCommit::new(
                    session.request.principal_node_id.clone(),
                    session.state(),
                    ContinuityCandidate::TerminalSummary(candidate),
                )?;
                Ok(SyncProcedureResult::PendingContinuity {
                    pending: Box::new(pending),
                })
            }
            Err(error) => Err(SyncServiceError::Continuity(error)),
        }
    }

    async fn commit_completed(
        &self,
        session: &mut ActiveSyncSession,
        candidate: CompletedCommitCandidate,
    ) -> Result<SyncProcedureResult, SyncServiceError> {
        candidate.validate()?;
        if candidate.principal_node_id() != &session.request.principal_node_id
            || candidate.source_stream_id() != &session.request.source_stream_id
            || candidate.direction() != session.request.direction
        {
            return Err(SyncServiceError::InvalidCompletionCandidate);
        }
        match self.continuity.commit_completed(candidate.clone()).await {
            Ok(result) => {
                validate_completed_acknowledgement(&candidate, &result)?;
                session.lifecycle.apply(
                    SyncTransition::CursorDurable,
                    CommitProof::AtomicCompletionDurable,
                )?;
                Ok(SyncProcedureResult::Terminal {
                    summary: Box::new(result.committed_summary),
                })
            }
            Err(error) if error.is_pending() => {
                let pending = PendingContinuityCommit::new(
                    session.request.principal_node_id.clone(),
                    session.state(),
                    ContinuityCandidate::CompletedAtomic(candidate),
                )?;
                Ok(SyncProcedureResult::PendingContinuity {
                    pending: Box::new(pending),
                })
            }
            Err(ContinuityError::CursorCompareConflict { evidence_refs }) => {
                let completed_summary = candidate.summary().clone();
                let summary = self.summary(
                    session,
                    SyncState::Failed,
                    SyncOutcomeClassification::CursorCommitFailed,
                    completed_summary.records_examined,
                    false,
                    evidence_refs,
                    completed_summary.agreement_snapshot_ref,
                    candidate.expected().cloned(),
                    None,
                    completed_summary.receipt_summary,
                )?;
                self.persist_terminal(session, summary, SyncTransition::CursorWriteFailed)
                    .await
            }
            Err(error) => Err(SyncServiceError::Continuity(error)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn summary(
        &self,
        session: &ActiveSyncSession,
        terminal_state: SyncState,
        outcome: SyncOutcomeClassification,
        records_examined: usize,
        cursor_committed: bool,
        evidence_refs: Vec<crate::types::EvidenceRef>,
        agreement_snapshot_ref: Option<crate::types::EvidenceRef>,
        cursor_before: Option<SourceBoundCursor>,
        cursor_after: Option<SourceBoundCursor>,
        receipt_summary: Option<ReceiptSummary>,
    ) -> Result<TerminalSyncSessionSummary, SyncServiceError> {
        let summary = TerminalSyncSessionSummary {
            principal_node_id: session.request.principal_node_id.clone(),
            session_id: session.session_id.clone(),
            direction: session.request.direction,
            source_stream_id: session.request.source_stream_id.clone(),
            trigger: session.request.trigger,
            terminal_state,
            started_at: session.started_at.clone(),
            finished_at: self.clock.now(),
            outcome,
            records_examined,
            cursor_committed,
            evidence_refs,
            previous_session_id: session.request.previous_session_id.clone(),
            agreement_snapshot_ref,
            cursor_before,
            cursor_after,
            receipt_summary,
        };
        summary.validate()?;
        Ok(summary)
    }
}

fn transition_for_terminal(
    prior: SyncState,
    terminal: SyncState,
) -> Result<SyncTransition, SyncServiceError> {
    match (prior, terminal) {
        (SyncState::Evaluating, SyncState::Completed) => Ok(SyncTransition::NoWorkNeeded),
        (SyncState::Evaluating, SyncState::Blocked) => Ok(SyncTransition::RefuseTransfer),
        (SyncState::Evaluating, SyncState::Failed) => Ok(SyncTransition::EvaluationFailure),
        (SyncState::Transferring, SyncState::Failed) => Ok(SyncTransition::TransportFailure),
        (SyncState::AwaitingDurableReceipts, SyncState::Failed) => {
            Ok(SyncTransition::ReceiptsNotCheckpointSafe)
        }
        (SyncState::CommittingCursor, SyncState::Failed) => Ok(SyncTransition::CursorWriteFailed),
        (SyncState::Requested | SyncState::Evaluating, SyncState::Cancelled) => {
            Ok(SyncTransition::CancelAtSafeBoundary)
        }
        _ => Err(SyncServiceError::InvalidPendingTransition),
    }
}

/// Fail-closed application-procedure error.
#[derive(Debug, Error)]
pub enum SyncServiceError {
    /// The authority-free request was invalid.
    #[error(transparent)]
    Request(#[from] RequestValidationError),
    /// Retry-lineage continuity was unavailable before a new lifecycle existed.
    #[error("retry lineage is unavailable: {0}")]
    RetryLineageUnavailable(ContinuityError),
    /// The referenced prior session did not exist.
    #[error("referenced retry session was not found")]
    RetrySessionNotFound,
    /// The referenced prior session was not an immutable failed or blocked attempt.
    #[error("referenced prior session is not retryable")]
    PriorSessionNotRetryable,
    /// The infallible identity issuer reused the referenced prior identity.
    #[error("identity issuer reused the prior session identity")]
    IdentityIssuerReusedPriorSession,
    /// The lifecycle rejected an application transition.
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    /// A source-issued next cursor was empty.
    #[error(transparent)]
    Identifier(#[from] crate::types::IdentifierError),
    /// An immutable terminal summary was internally inconsistent.
    #[error(transparent)]
    Summary(#[from] crate::types::SummaryValidationError),
    /// A pending candidate digest could not be constructed or verified.
    #[error(transparent)]
    CandidateDigest(#[from] CandidateDigestError),
    /// An immutable continuity candidate was internally inconsistent.
    #[error(transparent)]
    Candidate(#[from] crate::types::CandidateValidationError),
    /// The retained digest does not match the exact candidate.
    #[error("pending continuity candidate digest mismatch")]
    PendingCandidateDigestMismatch,
    /// The retained session identity does not match the exact candidate.
    #[error("pending continuity candidate session mismatch")]
    PendingCandidateSessionMismatch,
    /// Candidate state and terminal state do not name an allowed transition.
    #[error("pending continuity candidate has an invalid lifecycle transition")]
    InvalidPendingTransition,
    /// Cancellation was requested after checkpoint-unsafe work began.
    #[error("cancellation is unsafe from {state:?}")]
    UnsafeCancellationBoundary {
        /// State preserved by the refusal.
        state: SyncState,
    },
    /// Atomic candidate content did not satisfy application invariants.
    #[error("atomic completion candidate is internally inconsistent")]
    InvalidCompletionCandidate,
    /// Atomic continuity acknowledged content other than the exact candidate.
    #[error("atomic continuity acknowledgement differs from exact candidate")]
    ContinuityAcknowledgementMismatch,
    /// A prepared session was neither requested nor already evaluating.
    #[error("prepared session cannot evaluate from {state:?}")]
    PreparedSessionNotEvaluating {
        /// State preserved by the refusal.
        state: SyncState,
    },
    /// Continuity definitively rejected an immutable candidate.
    #[error(transparent)]
    Continuity(ContinuityError),
}

impl From<SourcePageError> for SyncServiceError {
    fn from(_: SourcePageError) -> Self {
        Self::InvalidCompletionCandidate
    }
}

trait AuthenticationBinding {
    fn matches_request(&self, request: &SyncRequest, observed_at: &ClockSample) -> bool;
    fn evidence_refs(&self) -> Vec<crate::types::EvidenceRef>;
}

impl AuthenticationBinding for TransportAuthentication {
    fn matches_request(&self, request: &SyncRequest, observed_at: &ClockSample) -> bool {
        match self {
            Self::Authenticated { evidence } => {
                evidence.source_stream_id == request.source_stream_id
                    && evidence.direction == request.direction
                    && &evidence.authenticated_at == observed_at
            }
            Self::NotReady { .. } => true,
        }
    }

    fn evidence_refs(&self) -> Vec<crate::types::EvidenceRef> {
        match self {
            Self::Authenticated { evidence } => vec![evidence.evidence_ref.clone()],
            Self::NotReady { evidence_refs } => evidence_refs.clone(),
        }
    }
}

trait PlanBinding {
    fn matches_request_and_authentication(
        &self,
        request: &SyncRequest,
        authentication: &TransportAuthentication,
        evaluated_at: &ClockSample,
    ) -> bool;
}

impl PlanBinding for CurrentSyncPlan {
    fn matches_request_and_authentication(
        &self,
        request: &SyncRequest,
        authentication: &TransportAuthentication,
        evaluated_at: &ClockSample,
    ) -> bool {
        let transport_matches = match authentication {
            TransportAuthentication::Authenticated { evidence } => {
                self.transport_principal == evidence.transport_principal
                    && self.transport_evidence_ref == evidence.evidence_ref
            }
            TransportAuthentication::NotReady { .. } => false,
        };
        self.principal_node_id == request.principal_node_id
            && self.source_stream_id == request.source_stream_id
            && self.direction == request.direction
            && &self.evaluated_at == evaluated_at
            && transport_matches
    }
}

fn ready_evidence_refs(
    plan: &CurrentSyncPlan,
    additional: Vec<crate::types::EvidenceRef>,
) -> Vec<crate::types::EvidenceRef> {
    let mut evidence = Vec::with_capacity(2 + additional.len());
    evidence.push(plan.agreement_snapshot_ref.clone());
    evidence.push(plan.transport_evidence_ref.clone());
    evidence.extend(additional);
    evidence
}

fn current_plan_evidence_refs(plan: &CurrentSyncPlan) -> Vec<crate::types::EvidenceRef> {
    vec![
        plan.domain_authorization_ref.clone(),
        plan.node_authorization_ref.clone(),
        plan.agreement_snapshot_ref.clone(),
        plan.export_head_ref.clone(),
        plan.admit_head_ref.clone(),
        plan.transport_evidence_ref.clone(),
    ]
}

fn validate_completed_acknowledgement(
    candidate: &CompletedCommitCandidate,
    result: &CompletedCommitResult,
) -> Result<(), SyncServiceError> {
    if &result.committed_cursor != candidate.candidate()
        || &result.committed_summary != candidate.summary()
    {
        return Err(SyncServiceError::ContinuityAcknowledgementMismatch);
    }
    Ok(())
}
