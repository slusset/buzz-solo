#![deny(unsafe_code)]
#![warn(missing_docs)]
//! Principal-Node-owned synchronization application services.
//!
//! This crate contains no host, process, filesystem, scheduler, HTTP, or
//! WebSocket behavior. Runtime composition supplies every effect through the
//! inward-owned ports in [`ports`]. Exact event envelopes and portable relay
//! records remain owned by `buzz-core`.

/// Sync-session lifecycle validation.
pub mod lifecycle;
/// Inward-owned application ports.
pub mod ports;
/// The single Principal-Node synchronization procedure.
pub mod service;
/// Compiler-distinct domain and evidence types.
pub mod types;

pub use lifecycle::{
    CommitProof, InvalidTransition, InvalidTransitionReason, LifecycleError, SyncLifecycle,
    SyncTransition,
};
pub use ports::{
    AttemptClock, AuthenticatedPeerTransport, CurrentSyncProjection, PrincipalNodeSyncContinuity,
    ReplicationSink, ReplicationSource, SyncSessionIdentityIssuer,
};
pub use service::{PrincipalNodeSyncService, SyncServiceError};
pub use types::*;
