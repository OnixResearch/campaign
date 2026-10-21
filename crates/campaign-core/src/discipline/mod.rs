//! Frontier-discipline API.

mod claims;
mod classification;
mod comparison;
pub(crate) mod declared;

pub use claims::{
    FrontierDynamics, PolicyImplementation, select_production_policy, validate_dynamics,
};
pub use classification::{
    FrontierRoleIdentity, FrontierRoleKind, FrontierRoleProjection, RoleCollapseFixture,
    check_no_role_collapse, derive_role_projection_id, project_frontier_roles,
};
pub use comparison::{
    DifferentialReport, EvidenceQuestion, compare_decisions, validate_differential_evidence,
};
pub use declared::{
    PlannedRequest, PriorityRule, SchedulingComparison, SchedulingRules, SelectionDifference,
    TieBreak, compare_scheduling, explain_selection_difference,
};
