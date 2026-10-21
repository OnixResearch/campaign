mod coalescing;
mod denial;
pub use coalescing::{
    AdmissionDecision, AdmissionOutcome, AdmissionRule, DuplicateCandidateRule,
    derive_admission_outcomes_id,
};
pub use denial::{AdmissionRefusal, AdmissionRefusalReason, DuplicateOrigin};

const LABEL_CANDIDATE: &[u8] = b"candidate";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.candidate_admission]
pub struct CandidateRecord {
    /// Public Campaign core API value.
    pub candidate_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub parent_moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub operation_profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub operation_schema_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub source_adapter_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub depth: u64,
    /// Public Campaign core API value.
    pub cost_profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub declared_costs: alloc::vec::Vec<super::bounds::ResourceAmount>,
    /// Public Campaign core API value.
    pub operation_generation_profile_id: crate::identity::Id,
}

impl CandidateRecord {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.validate_without_candidate_id()?;
        self.declared_costs.sort_by_key(|cost| cost.dimension_id);
        self.candidate_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Candidate,
            LABEL_CANDIDATE,
            |mut writer| {
                writer = writer.id(self.campaign_id);
                writer = writer.id(self.parent_moment_id);
                writer = writer.id(self.operation_profile_id);
                writer = writer.id(self.operation_schema_id);
                writer = writer.id(self.source_adapter_id);
                writer = writer.u64(self.depth);
                writer = writer.id(self.cost_profile_id);
                writer = super::bounds::write_resources(writer, &self.declared_costs)?;
                writer = writer.id(self.operation_generation_profile_id);
                Ok(writer)
            },
        )?;
        Ok(self)
    }

    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn validate(&self, campaign_id: crate::identity::Id) -> crate::error::CampaignResult<()> {
        self.candidate_id
            .require_kind(crate::identity::IdKind::Candidate)?;
        if self.campaign_id != campaign_id {
            return Err(crate::error::CampaignError::CrossedIdentity);
        }
        let derived = self.clone().derive()?;
        if derived.candidate_id != self.candidate_id
            || derived.declared_costs != self.declared_costs
        {
            return Err(crate::error::CampaignError::MalformedIdentity);
        }
        Ok(())
    }

    fn validate_without_candidate_id(&self) -> crate::error::CampaignResult<()> {
        self.campaign_id
            .require_kind(crate::identity::IdKind::Campaign)?;
        self.parent_moment_id.require_kind_any(&[
            crate::identity::IdKind::RootMoment,
            crate::identity::IdKind::ChildMoment,
        ])?;
        self.operation_profile_id
            .require_kind(crate::identity::IdKind::OperationProfile)?;
        self.operation_schema_id
            .require_kind(crate::identity::IdKind::OperationSchema)?;
        self.source_adapter_id
            .require_kind(crate::identity::IdKind::Adapter)?;
        self.cost_profile_id
            .require_kind(crate::identity::IdKind::CostProfile)?;
        self.operation_generation_profile_id
            .require_kind(crate::identity::IdKind::Policy)?;
        if self.declared_costs.is_empty() {
            return Err(crate::error::CampaignError::InvalidCostProfile);
        }
        let mut ids = alloc::vec::Vec::with_capacity(self.declared_costs.len());
        for cost in &self.declared_costs {
            cost.validate()?;
            if cost.amount == 0 {
                return Err(crate::error::CampaignError::InvalidCostProfile);
            }
            ids.push(cost.dimension_id);
        }
        ids.sort();
        if !crate::identity::unique_sorted_ids(&ids) {
            return Err(crate::error::CampaignError::DuplicateIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct CandidateBatchEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Admitted candidates in canonical identity order; coalesced duplicates are omitted.
    pub candidates: alloc::vec::Vec<CandidateRecord>,
    /// Declared admission rule that decided every outcome.
    pub admission_rule: AdmissionRule,
    /// Admission policy identity derived from `admission_rule`.
    pub admission_policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub coalescing_policy_id: crate::identity::Id,
    /// One outcome per submitted candidate in canonical identity order.
    pub outcomes: alloc::vec::Vec<AdmissionOutcome>,
    /// `AdmissionOutcomes` identity over `outcomes`, bound by receipts.
    pub admission_outcomes_id: crate::identity::Id,
}

impl CandidateBatchEvent {
    /// Derives the control event identity from the admitted candidates, policies, and outcomes.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when a bound identity is missing or framing fails.
    pub fn expected_event_id(&self) -> crate::error::CampaignResult<crate::identity::Id> {
        let mut ids = alloc::vec::Vec::with_capacity(self.candidates.len());
        for candidate in &self.candidates {
            ids.push(candidate.candidate_id);
        }
        ids.push(self.admission_policy_id);
        ids.push(self.coalescing_policy_id);
        ids.push(self.admission_outcomes_id);
        super::events::derive_event_id(
            &super::events::ControlEventKind::CandidatesAdmitted,
            self.campaign_id,
            self.state_before_id,
            &ids,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub enum CandidateStatus {
    /// Public Campaign core API variant.
    Retained,
    /// Public Campaign core API variant.
    Retired {
        /// Bounded reason identity for retirement.
        reason_id: crate::identity::Id,
    },
    /// Public Campaign core API variant.
    Pruned {
        /// Prune plan identity that removed the candidate from the frontier.
        prune_plan_id: crate::identity::Id,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct CandidateState {
    /// Public Campaign core API value.
    pub candidate: CandidateRecord,
    /// Public Campaign core API value.
    pub status: CandidateStatus,
    /// Public Campaign core API value.
    pub selection_count: u64,
    /// Public Campaign core API value.
    pub latest_guidance: Option<super::guidance::GuidanceRecord>,
}

trait RequireAnyKind {
    fn require_kind_any(
        self,
        kinds: &[crate::identity::IdKind],
    ) -> crate::error::CampaignResult<Self>
    where
        Self: Sized;
}

impl RequireAnyKind for crate::identity::Id {
    fn require_kind_any(
        self,
        kinds: &[crate::identity::IdKind],
    ) -> crate::error::CampaignResult<Self> {
        if self.is_absent() {
            return Err(crate::error::CampaignError::MissingIdentity);
        }
        if kinds.contains(&self.kind) {
            return Ok(self);
        }
        Err(crate::error::CampaignError::CrossedIdentity)
    }
}
