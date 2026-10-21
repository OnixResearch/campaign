//! Product-neutral `no_std + alloc` adaptive exploration policy core.
//!
//! The crate consumes explicit values and returns deterministic admission,
//! frontier, selection, pruning, stop, cache, and receipt decisions. It does
//! not implement Choregraph history, execution, persistence, retries, replay,
//! minimization, clocks, randomness, or release authority.

#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(dylint_lib = "octet", feature(register_tool))]
#![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]

// r[impl campaign.functional_core]
extern crate alloc;

/// Public Campaign core API value.
pub mod discipline;
/// Public Campaign core API value.
pub mod error;
/// Public Campaign core API value.
pub mod frontier;
/// Public Campaign core API value.
pub mod identity;
/// Public Campaign core API value.
pub mod model;
/// Public Campaign core API value.
pub mod policy;

/// Public Campaign core API value.
pub use discipline::*;
/// Public Campaign core API value.
pub use error::{CampaignError, CampaignResult};
/// Public Campaign core API value.
pub use frontier::{
    FrontierState, KnownMoment, PayloadBinding, Settlement, StableDiscovery, apply_control_event,
    reconstruct,
};
/// Public Campaign core API value.
pub use identity::{DIGEST_LENGTH, FrameWriter, Id, IdKind, derive_id};
/// Public Campaign core API value.
pub use model::*;
/// Public Campaign core API value.
pub use policy::*;
