mod dynamics;
pub use dynamics::{GuidanceDynamics, derive_revision_invalidation_id};

const LABEL_GUIDANCE: &[u8] = b"guidance";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.guidance]
pub struct GuidanceRecord {
    /// Public Campaign core API value.
    pub guidance_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub candidate_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub generation: u64,
    /// Public Campaign core API value.
    pub ranks: alloc::vec::Vec<i64>,
    /// Public Campaign core API value.
    pub source_facts_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub ordering_profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub adapter_id: crate::identity::Id,
}

impl GuidanceRecord {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    // r[impl campaign.guidance]
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.validate_without_guidance_id()?;
        self.guidance_id = crate::identity::derive_id_with(
            crate::identity::IdKind::GuidanceRecord,
            LABEL_GUIDANCE,
            |mut writer| {
                writer = writer.id(self.campaign_id);
                writer = writer.id(self.candidate_id);
                writer = writer.u64(self.generation);
                writer = writer.seq_len(crate::identity::usize_to_u64(self.ranks.len())?);
                for rank in &self.ranks {
                    writer = writer.i64(*rank);
                }
                writer = writer.id(self.source_facts_id);
                writer = writer.id(self.ordering_profile_id);
                writer = writer.id(self.adapter_id);
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
        self.guidance_id
            .require_kind(crate::identity::IdKind::GuidanceRecord)?;
        if self.campaign_id != campaign_id {
            return Err(crate::error::CampaignError::CrossedIdentity);
        }
        let derived = self.clone().derive()?;
        if derived.guidance_id != self.guidance_id {
            return Err(crate::error::CampaignError::MalformedIdentity);
        }
        Ok(())
    }

    fn validate_without_guidance_id(&self) -> crate::error::CampaignResult<()> {
        self.campaign_id
            .require_kind(crate::identity::IdKind::Campaign)?;
        self.candidate_id
            .require_kind(crate::identity::IdKind::Candidate)?;
        if self.ranks.is_empty() {
            return Err(crate::error::CampaignError::InvalidGuidanceShape);
        }
        self.source_facts_id
            .require_kind(crate::identity::IdKind::SourceFacts)?;
        self.ordering_profile_id
            .require_kind(crate::identity::IdKind::GuidanceProfile)?;
        self.adapter_id
            .require_kind(crate::identity::IdKind::Adapter)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct GuidanceEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub guidance: GuidanceRecord,
    /// Dynamic by which the guidance entered the frontier.
    pub dynamics: GuidanceDynamics,
    /// `AdmissionOutcomes` identity of the recorded dynamic, bound by receipts.
    pub admission_outcomes_id: crate::identity::Id,
}

impl GuidanceEvent {
    /// Derives the control event identity from the guidance record and its recorded outcome.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when a bound identity is missing or framing fails.
    pub fn expected_event_id(&self) -> crate::error::CampaignResult<crate::identity::Id> {
        super::events::derive_event_id(
            &super::events::ControlEventKind::GuidanceAdmitted,
            self.campaign_id,
            self.state_before_id,
            &[self.guidance.guidance_id, self.admission_outcomes_id],
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value: explicit revision request that invalidates the latest guidance record of one candidate.
pub struct GuidanceRevisionRequest {
    /// Candidate whose latest guidance is revised.
    pub candidate_id: crate::identity::Id,
    /// Successor record with a higher generation and any valid ranks.
    pub successor: GuidanceRecord,
    /// Latest guidance record that the revision invalidates.
    pub invalidated_guidance_id: crate::identity::Id,
    /// Bounded consumer reason identity.
    pub reason_id: crate::identity::Id,
}
