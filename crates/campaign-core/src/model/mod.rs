//! Stable Campaign model API, composed from capability-owned contracts.

mod admission;
mod bounds;
mod caches;
mod candidates;
mod decision;
mod entropy;
mod events;
mod guidance;
mod observation;
mod projection;
mod pruning;
mod publication;
mod receipts;
mod selection;
mod snapshots;
mod stopping;
mod termination;

pub use admission::{CampaignAdmission, derive_campaign_event_id};
pub use bounds::{
    ExplorationBounds, MAX_RANK_MAGNITUDE, MIN_CONTROL_EVENTS, ResourceAmount, check_len,
    check_len_at_least_one, write_resources,
};
pub use caches::{CacheProjection, derive_cache_id};
pub use candidates::{
    AdmissionDecision, AdmissionOutcome, AdmissionRefusal, AdmissionRefusalReason, AdmissionRule,
    CandidateBatchEvent, CandidateRecord, CandidateState, CandidateStatus, DuplicateCandidateRule,
    DuplicateOrigin, derive_admission_outcomes_id,
};
pub use decision::{DecisionPlan, NextDecision, PlanningWork, WaitReason};
pub use entropy::EntropyTicket;
pub use events::{CampaignControlEvent, ControlEventKind, derive_event_id};
pub use guidance::{
    GuidanceDynamics, GuidanceEvent, GuidanceRecord, GuidanceRevisionRequest,
    derive_revision_invalidation_id,
};
pub use observation::{ChildObservation, FindingRef, ResultEvent, ResultObservation};
pub use projection::{ProjectionBinding, RootMoment};
pub use pruning::{
    PruneEvent, PruneRequest, RetentionPlanRequest, derive_prune_invalidation_id,
    derive_prune_plan_id,
};
pub use publication::{OutstandingSelection, PublicationPlan, SelectionDecision, SelectionEvent};
pub use receipts::{
    CampaignReceipt, EventReceiptRequest, NON_CLAIM_COUNT, NonClaim, ReceiptClaims,
    RefusalReceiptRequest, derive_cost_reservation_id, derive_receipt_id,
};
pub use selection::{
    SelectionCountDecay, SelectionMode, SelectionProfile, SelectionRequest, StoppingRule,
};
pub use snapshots::write_ids;
pub use stopping::{StopDecision, StopEvent, StopReason, derive_stop_id};
pub use termination::{
    ClosureEvent, RetirementEvent, RetirementOrigin, RetirementRequest, UnobservedClosure,
    derive_closure_id, derive_retirement_invalidation_id,
};

/// Public Campaign core API value.
pub const CAMPAIGN_SCHEMA_VERSION: u32 = 1;

/// Public Campaign core API value.
pub const ENTROPY_SCALE: u64 = 1_000_000;

/// Public Campaign core API value.
pub const ENTROPY_PREFIX_BYTES: usize = 8;

/// Exclusive end of the entropy ticket index-draw window.
pub const ENTROPY_INDEX_DRAW_END: usize = 16;

/// Public Campaign core API value.
pub const RANK_FIRST_DIMENSION: usize = 0;

/// Public Campaign core API value.
pub const EMPTY_COUNT: usize = 0;

/// Public Campaign core API value.
pub const NO_SELECTIONS: u64 = 0;

/// Public Campaign core API value.
pub const UNIT_COST: u64 = 1;
