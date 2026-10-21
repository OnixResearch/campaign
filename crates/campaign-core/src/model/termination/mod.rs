const LABEL_CLOSURE: &[u8] = b"unobserved-closure";

const LABEL_RETIREMENT_INVALIDATION: &[u8] = b"candidate-retirement-invalidation";

const ORIGIN_CONSUMER: u32 = 1;

const ORIGIN_POLICY: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.unobserved]
pub struct UnobservedClosure {
    /// Public Campaign core API value.
    pub closure_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub selection_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub reason_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub uncertainty_acknowledged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct ClosureEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub closure: UnobservedClosure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Declared source of an explicit candidate retirement.
pub enum RetirementOrigin {
    /// The consumer requested the retirement with its own bounded reason.
    Consumer,
    /// The admitted Campaign policy requested the retirement with a bounded reason.
    Policy,
}

impl RetirementOrigin {
    /// Stable wire code framed into retirement identities.
    #[must_use]
    pub const fn code(&self) -> u32 {
        match self {
            Self::Consumer => ORIGIN_CONSUMER,
            Self::Policy => ORIGIN_POLICY,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Exact current-state request to retire one retained candidate.
// r[impl campaign.retirement]
pub struct RetirementRequest {
    /// Campaign that owns the candidate.
    pub campaign_id: crate::identity::Id,
    /// Current frontier state the request was built against.
    pub state_id: crate::identity::Id,
    /// Admitted Campaign policy identity.
    pub policy_id: crate::identity::Id,
    /// Admitted adapter identity.
    pub adapter_id: crate::identity::Id,
    /// Retained candidate to retire.
    pub candidate_id: crate::identity::Id,
    /// Bounded retirement reason identity.
    pub reason_id: crate::identity::Id,
    /// Declared retirement source.
    pub origin: RetirementOrigin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Control event that explicitly invalidates one retained candidate.
pub struct RetirementEvent {
    /// Control event identity.
    pub event_id: crate::identity::Id,
    /// Campaign that owns the candidate.
    pub campaign_id: crate::identity::Id,
    /// Frontier state the retirement was admitted against.
    pub state_before_id: crate::identity::Id,
    /// Retired candidate identity.
    pub candidate_id: crate::identity::Id,
    /// Bounded retirement reason identity.
    pub reason_id: crate::identity::Id,
    /// Admitted Campaign policy identity.
    pub policy_id: crate::identity::Id,
    /// Admitted adapter identity.
    pub adapter_id: crate::identity::Id,
    /// Declared retirement source.
    pub origin: RetirementOrigin,
    /// Explicit invalidation identity naming the retired candidate.
    pub invalidation_id: crate::identity::Id,
}

impl RetirementEvent {
    /// Reports the retirement as an explicit invalidation of its candidate, never as an addition.
    #[must_use]
    // r[impl campaign.fd.dynamics.invalidate]
    pub const fn dynamics(&self) -> crate::discipline::FrontierDynamics {
        crate::discipline::FrontierDynamics::Invalidation {
            invalidation_id: self.invalidation_id,
            cleared_dependency_id: self.candidate_id,
        }
    }
}

/// Derives the invalidation identity that a retirement event carries.
///
/// # Errors
///
/// Returns a typed `CampaignError` when a framed identity has the wrong kind.
pub fn derive_retirement_invalidation_id(
    event: &RetirementEvent,
) -> crate::error::CampaignResult<crate::identity::Id> {
    event
        .campaign_id
        .require_kind(crate::identity::IdKind::Campaign)?;
    event
        .candidate_id
        .require_kind(crate::identity::IdKind::Candidate)?;
    event
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    event
        .policy_id
        .require_kind(crate::identity::IdKind::Policy)?;
    event
        .adapter_id
        .require_kind(crate::identity::IdKind::Adapter)?;
    debug_assert_eq!(event.candidate_id.kind, crate::identity::IdKind::Candidate);
    debug_assert_eq!(event.reason_id.kind, crate::identity::IdKind::Reason);
    crate::identity::derive_id_with(
        crate::identity::IdKind::Invalidation,
        LABEL_RETIREMENT_INVALIDATION,
        |mut writer| {
            writer = writer.id(event.campaign_id);
            writer = writer.id(event.state_before_id);
            writer = writer.id(event.candidate_id);
            writer = writer.id(event.reason_id);
            writer = writer.id(event.policy_id);
            writer = writer.id(event.adapter_id);
            writer = writer.u32(event.origin.code());
            Ok(writer)
        },
    )
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
/// # Panics
///
/// Panics only if successful identity validation violates its kind contract.
/// Invalid external identities return a typed error before these assertions.
pub fn derive_closure_id(
    closure: &UnobservedClosure,
) -> crate::error::CampaignResult<crate::identity::Id> {
    closure
        .campaign_id
        .require_kind(crate::identity::IdKind::Campaign)?;
    closure
        .selection_id
        .require_kind(crate::identity::IdKind::Selection)?;
    closure
        .reason_id
        .require_kind(crate::identity::IdKind::Reason)?;
    assert_eq!(closure.campaign_id.kind, crate::identity::IdKind::Campaign);
    assert_eq!(
        closure.selection_id.kind,
        crate::identity::IdKind::Selection
    );
    crate::identity::derive_id_with(
        crate::identity::IdKind::Event,
        LABEL_CLOSURE,
        |mut writer| {
            writer = writer.id(closure.campaign_id);
            writer = writer.id(closure.selection_id);
            writer = writer.id(closure.reason_id);
            writer = writer.bool(closure.uncertainty_acknowledged);
            Ok(writer)
        },
    )
}
