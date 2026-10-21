#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(dylint_lib = "octet", feature(register_tool))]
#![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]

//! One-way adapter from Campaign control events to pinned Choregraph history.
//!
//! This crate validates immutable Choregraph application envelopes and maps
//! generation-fenced branch plans back to Campaign publication decisions. It
//! does not choose semantic parents, execute operations, or claim durability.

extern crate alloc;

mod admission;
mod contracts;
mod envelope;
mod exclusivity;
mod fast_forward;
mod freshness;
mod headroom;
mod identity;
mod lineage;
mod occupancy;

pub use admission::genesis::admit_control_root;
pub use admission::{can_execute_selection_after_publication, plan_control_event_publication};
pub use choregraph_history::{
    BranchRef, Digest, EventClass, EventDraft, EventId, HISTORY_EVENT_VERSION_V1, HistoryError,
    HistoryGraph, HistoryLimits, MetadataIdentity, ParentChoices, ParentLink,
    ReferenceMutationPlan,
};
pub use contracts::{
    AdapterError, AdapterResult, ControlPayloadEnvelope, ControlProjectionRequest,
    ControlPublication, ControlPublicationRequest, ControlRootAdmission,
    DurablePublicationEvidence, PublishedSelectionRequest, ValidatedControlProjection,
};
pub use envelope::control_event_draft;
pub use exclusivity::{PendingDraft, pending_control_draft};
pub use fast_forward::validate_control_advance;
pub use freshness::{CacheValidationRequest, validate_cache_against_history};
pub use headroom::control_capacity_reason_id;
pub use identity::{
    branch_fact, derive_control_payload_id, derive_projection_binding, digest_from_id,
    event_id_from_event, head_id_from_event, id_from_digest,
};
pub use lineage::{plan_next_from_published_projection, validate_control_projection};
