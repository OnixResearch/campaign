const LABEL_SELECTION: &[u8] = b"selection";

const LABEL_OPERATION: &[u8] = b"operation";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct OutstandingSelection {
    /// Public Campaign core API value.
    pub selection_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub candidate_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub parent_moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub operation_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub ordinal: u64,
    /// Public Campaign core API value.
    pub reserved_costs: alloc::vec::Vec<super::bounds::ResourceAmount>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct PublicationPlan {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub control_branch_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub expected_head_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub expected_generation: u64,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub selection_id: crate::identity::Id,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.selection]
pub struct SelectionDecision {
    /// Public Campaign core API value.
    pub selection_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub ordinal: u64,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub candidate_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub parent_moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub operation_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub guidance_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub entropy_id: Option<crate::identity::Id>,
    /// Public Campaign core API value.
    pub reserved_costs: alloc::vec::Vec<super::bounds::ResourceAmount>,
    /// Public Campaign core API value.
    pub publication_plan: PublicationPlan,
}

impl SelectionDecision {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    // r[impl campaign.selection]
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.reserved_costs.sort_by_key(|cost| cost.dimension_id);
        self.publication_plan.campaign_id = self.campaign_id;
        self.publication_plan.state_before_id = self.state_before_id;
        self.selection_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Selection,
            LABEL_SELECTION,
            |mut writer| {
                writer = writer.id(self.campaign_id);
                writer = writer.id(self.state_before_id);
                writer = writer.u64(self.ordinal);
                writer = writer.id(self.candidate_id);
                writer = writer.id(self.parent_moment_id);
                writer = writer.id(self.guidance_id);
                writer = writer.id(self.profile_id);
                writer = writer.id(self.publication_plan.control_branch_id);
                writer = writer.id(self.publication_plan.expected_head_id);
                writer = writer.u64(self.publication_plan.expected_generation);
                writer = writer.option_id(self.entropy_id);
                writer = super::bounds::write_resources(writer, &self.reserved_costs)?;
                Ok(writer)
            },
        )?;
        self.operation_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Operation,
            LABEL_OPERATION,
            |mut writer| {
                writer = writer.id(self.selection_id);
                writer = writer.id(self.candidate_id);
                writer = writer.u64(self.ordinal);
                Ok(writer)
            },
        )?;
        self.publication_plan.selection_id = self.selection_id;
        self.publication_plan.event_id = super::events::derive_event_id(
            &super::events::ControlEventKind::SelectionPublished,
            self.campaign_id,
            self.state_before_id,
            &[self.selection_id],
        )?;
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Published selection with the exact policy inputs replay re-validates.
pub struct SelectionEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub decision: SelectionDecision,
    /// Selection profile that planned the decision; its identity is the decision's `profile_id`.
    pub profile: alloc::boxed::Box<super::selection::SelectionProfile>,
    /// Explicit entropy ticket of an exploratory decision; its identity is the decision's
    /// `entropy_id`. Ranked decisions carry none.
    pub entropy: Option<super::entropy::EntropyTicket>,
}

impl SelectionEvent {
    /// Builds the publication event of one planned selection with the profile and entropy ticket
    /// of the request that planned it.
    ///
    /// # Errors
    ///
    /// Returns `CrossedIdentity` when the profile or ticket identity differs from the one the
    /// decision names.
    pub fn new(
        decision: SelectionDecision,
        profile: super::selection::SelectionProfile,
        entropy: Option<super::entropy::EntropyTicket>,
    ) -> crate::error::CampaignResult<Self> {
        let event = Self {
            event_id: decision.publication_plan.event_id,
            decision,
            profile: alloc::boxed::Box::new(profile),
            entropy,
        };
        event.check_policy_inputs()?;
        debug_assert_eq!(event.event_id, event.decision.publication_plan.event_id);
        Ok(event)
    }

    /// Checks that the carried profile and ticket are the ones the decision names.
    ///
    /// # Errors
    ///
    /// Returns `CrossedIdentity` when the profile or ticket identity differs from the decision.
    pub fn check_policy_inputs(&self) -> crate::error::CampaignResult<()> {
        let ticket_id = self.entropy.as_ref().map(|ticket| ticket.entropy_id);
        if self.profile.profile_id != self.decision.profile_id
            || ticket_id != self.decision.entropy_id
        {
            return Err(crate::error::CampaignError::CrossedIdentity);
        }
        Ok(())
    }
}
