#[cfg(test)]
mod tests;

/// Public Campaign core API value.
pub const DIGEST_LENGTH: usize = 32;
/// Public Campaign core API value.
pub const ID_VERSION_ONE: u32 = 1;

const KIND_ABSENT: u8 = 0;
const KIND_CAMPAIGN: u8 = 1;
const KIND_PROJECTION: u8 = 2;
const KIND_GRAPH: u8 = 3;
const KIND_CONTROL_BRANCH: u8 = 4;
const KIND_HEAD: u8 = 5;
const KIND_POLICY: u8 = 6;
const KIND_GUIDANCE_PROFILE: u8 = 7;
const KIND_ENTROPY_PROFILE: u8 = 8;
const KIND_BUDGET_PROFILE: u8 = 9;
const KIND_ADAPTER: u8 = 10;
const KIND_CONTROL_DOMAIN: u8 = 11;
const KIND_ROOT_MOMENT: u8 = 12;
const KIND_OPERATION_PROFILE: u8 = 13;
const KIND_OPERATION_SCHEMA: u8 = 14;
const KIND_COST_PROFILE: u8 = 15;
const KIND_CANDIDATE: u8 = 16;
const KIND_GUIDANCE_RECORD: u8 = 17;
const KIND_STATE: u8 = 18;
const KIND_SELECTION: u8 = 19;
const KIND_OPERATION: u8 = 20;
const KIND_EVENT: u8 = 21;
const KIND_RESULT: u8 = 22;
const KIND_CHILD_RESERVATION: u8 = 23;
const KIND_CHILD_MOMENT: u8 = 24;
const KIND_FINDING: u8 = 25;
const KIND_REASON: u8 = 26;
const KIND_ENTROPY_TICKET: u8 = 27;
const KIND_RECEIPT: u8 = 28;
const KIND_CACHE: u8 = 29;
const KIND_SCHEDULING_PROFILE: u8 = 30;
const KIND_ADMISSION_POLICY: u8 = 31;
const KIND_COALESCING_POLICY: u8 = 32;
const KIND_SOURCE_FACTS: u8 = 33;
const KIND_EVENT_PAYLOAD: u8 = 34;
const KIND_PRUNE_PLAN: u8 = 35;
const KIND_STOP: u8 = 36;
const KIND_DISCOVERY: u8 = 37;
const KIND_RESOURCE_DIMENSION: u8 = 38;
const KIND_SELECTION_PROFILE: u8 = 39;
const KIND_REQUEST: u8 = 40;
const KIND_DERIVED_RECORD: u8 = 41;
const KIND_RELAXATION: u8 = 42;
const KIND_INVALIDATION: u8 = 43;
const KIND_RETENTION_PROFILE: u8 = 44;
const KIND_ADMISSION_OUTCOMES: u8 = 45;
const KIND_ROLE_PROJECTION: u8 = 46;
const KIND_COST_RESERVATION: u8 = 47;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// Public Campaign core API value.
pub enum IdKind {
    /// Public Campaign core API variant.
    Absent = KIND_ABSENT,
    /// Public Campaign core API variant.
    Campaign = KIND_CAMPAIGN,
    /// Public Campaign core API variant.
    Projection = KIND_PROJECTION,
    /// Public Campaign core API variant.
    Graph = KIND_GRAPH,
    /// Public Campaign core API variant.
    ControlBranch = KIND_CONTROL_BRANCH,
    /// Public Campaign core API variant.
    Head = KIND_HEAD,
    /// Public Campaign core API variant.
    Policy = KIND_POLICY,
    /// Public Campaign core API variant.
    GuidanceProfile = KIND_GUIDANCE_PROFILE,
    /// Public Campaign core API variant.
    EntropyProfile = KIND_ENTROPY_PROFILE,
    /// Public Campaign core API variant.
    BudgetProfile = KIND_BUDGET_PROFILE,
    /// Public Campaign core API variant.
    Adapter = KIND_ADAPTER,
    /// Public Campaign core API variant.
    ControlDomain = KIND_CONTROL_DOMAIN,
    /// Public Campaign core API variant.
    RootMoment = KIND_ROOT_MOMENT,
    /// Public Campaign core API variant.
    OperationProfile = KIND_OPERATION_PROFILE,
    /// Public Campaign core API variant.
    OperationSchema = KIND_OPERATION_SCHEMA,
    /// Public Campaign core API variant.
    CostProfile = KIND_COST_PROFILE,
    /// Public Campaign core API variant.
    Candidate = KIND_CANDIDATE,
    /// Public Campaign core API variant.
    GuidanceRecord = KIND_GUIDANCE_RECORD,
    /// Public Campaign core API variant.
    State = KIND_STATE,
    /// Public Campaign core API variant.
    Selection = KIND_SELECTION,
    /// Public Campaign core API variant.
    Operation = KIND_OPERATION,
    /// Public Campaign core API variant.
    Event = KIND_EVENT,
    /// Public Campaign core API variant.
    Result = KIND_RESULT,
    /// Public Campaign core API variant.
    ChildReservation = KIND_CHILD_RESERVATION,
    /// Public Campaign core API variant.
    ChildMoment = KIND_CHILD_MOMENT,
    /// Public Campaign core API variant.
    Finding = KIND_FINDING,
    /// Public Campaign core API variant.
    Reason = KIND_REASON,
    /// Public Campaign core API variant.
    EntropyTicket = KIND_ENTROPY_TICKET,
    /// Public Campaign core API variant.
    Receipt = KIND_RECEIPT,
    /// Public Campaign core API variant.
    Cache = KIND_CACHE,
    /// Public Campaign core API variant.
    SchedulingProfile = KIND_SCHEDULING_PROFILE,
    /// Public Campaign core API variant.
    AdmissionPolicy = KIND_ADMISSION_POLICY,
    /// Public Campaign core API variant.
    CoalescingPolicy = KIND_COALESCING_POLICY,
    /// Public Campaign core API variant.
    SourceFacts = KIND_SOURCE_FACTS,
    /// Public Campaign core API variant.
    EventPayload = KIND_EVENT_PAYLOAD,
    /// Public Campaign core API variant.
    PrunePlan = KIND_PRUNE_PLAN,
    /// Public Campaign core API variant.
    Stop = KIND_STOP,
    /// Public Campaign core API variant.
    Discovery = KIND_DISCOVERY,
    /// Public Campaign core API variant.
    ResourceDimension = KIND_RESOURCE_DIMENSION,
    /// Public Campaign core API variant.
    SelectionProfile = KIND_SELECTION_PROFILE,
    /// Public Campaign core API variant.
    Request = KIND_REQUEST,
    /// Public Campaign core API variant.
    DerivedRecord = KIND_DERIVED_RECORD,
    /// Public Campaign core API variant.
    Relaxation = KIND_RELAXATION,
    /// Public Campaign core API variant.
    Invalidation = KIND_INVALIDATION,
    /// Declared retention (pruning) profile identity.
    RetentionProfile = KIND_RETENTION_PROFILE,
    /// Identity of one ordered candidate admission-outcome list.
    AdmissionOutcomes = KIND_ADMISSION_OUTCOMES,
    /// Identity of one frontier role projection.
    RoleProjection = KIND_ROLE_PROJECTION,
    /// Identity of one reserved-cost set.
    CostReservation = KIND_COST_RESERVATION,
}

impl IdKind {
    /// Public Campaign core API value.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// Public Campaign core API value.
pub struct Id {
    /// Public Campaign core API value.
    pub kind: IdKind,
    /// Public Campaign core API value.
    pub digest: [u8; DIGEST_LENGTH],
}

impl Id {
    /// Public Campaign core API value.
    #[must_use]
    pub const fn absent() -> Self {
        Self {
            kind: IdKind::Absent,
            digest: [0; DIGEST_LENGTH],
        }
    }

    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when the identity is absent or has another kind.
    // r[impl campaign.identity.missing]
    pub fn require_kind(self, kind: IdKind) -> crate::error::CampaignResult<Self> {
        if self.kind == IdKind::Absent {
            return Err(crate::error::CampaignError::MissingIdentity);
        }
        if self.kind != kind {
            return Err(crate::error::CampaignError::CrossedIdentity);
        }
        Ok(self)
    }

    /// Public Campaign core API operation.
    #[must_use]
    pub const fn is_absent(self) -> bool {
        matches!(self.kind, IdKind::Absent)
    }
}
