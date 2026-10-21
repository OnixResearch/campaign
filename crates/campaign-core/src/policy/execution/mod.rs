#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// One observed control-branch value: branch identity, head, and compare-and-set generation.
pub struct BranchFact {
    /// Control-branch identity of this exact branch value.
    pub control_branch_id: crate::identity::Id,
    /// Branch head identity.
    pub head_id: crate::identity::Id,
    /// Compare-and-set generation.
    pub generation: u64,
}

impl BranchFact {
    fn validate(&self) -> crate::error::CampaignResult<()> {
        self.control_branch_id
            .require_kind(crate::identity::IdKind::ControlBranch)?;
        self.head_id.require_kind(crate::identity::IdKind::Head)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Host-observed facts about the publication of one selection event.
pub struct PublicationEvidence {
    /// Campaign control event identity that was published.
    pub event_id: crate::identity::Id,
    /// Selection carried by the published event.
    pub selection_id: crate::identity::Id,
    /// Branch value observed before the compare-and-set update.
    pub branch_before: BranchFact,
    /// Branch value observed after the compare-and-set update.
    pub branch_after: BranchFact,
    /// True only when the host observed durable event admission.
    pub durable_event: bool,
    /// True only when the host observed a durable branch update.
    pub durable_branch_update: bool,
}

/// Returns whether a published selection may execute: the selection must re-derive to itself,
/// the evidence must match the selection's publication fence exactly, and both durability flags
/// must hold.
///
/// # Errors
///
/// Returns `MalformedIdentity` when the selection's identities do not re-derive from its content,
/// `InvalidPublication` when the evidence does not match the planned event, selection, branch
/// fence, or single-generation advance, and a typed identity error for malformed kinds.
// r[impl campaign.publication_fence]
pub fn can_execute_published_selection(
    selection: &crate::model::SelectionDecision,
    evidence: &crate::policy::execution::PublicationEvidence,
) -> crate::error::CampaignResult<bool> {
    evidence
        .event_id
        .require_kind(crate::identity::IdKind::Event)?;
    evidence
        .selection_id
        .require_kind(crate::identity::IdKind::Selection)?;
    evidence.branch_before.validate()?;
    evidence.branch_after.validate()?;
    if selection.clone().derive()? != *selection {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let plan = &selection.publication_plan;
    if evidence.event_id != plan.event_id || evidence.selection_id != selection.selection_id {
        return Err(crate::error::CampaignError::InvalidPublication);
    }
    let planned_before = BranchFact {
        control_branch_id: plan.control_branch_id,
        head_id: plan.expected_head_id,
        generation: plan.expected_generation,
    };
    if evidence.branch_before != planned_before {
        return Err(crate::error::CampaignError::InvalidPublication);
    }
    let expected_after_generation = evidence
        .branch_before
        .generation
        .checked_add(crate::model::UNIT_COST)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    if evidence.branch_after.generation != expected_after_generation
        || evidence.branch_after.head_id == evidence.branch_before.head_id
    {
        return Err(crate::error::CampaignError::InvalidPublication);
    }
    debug_assert!(evidence.branch_after.generation > plan.expected_generation);
    debug_assert_eq!(evidence.branch_before.head_id, plan.expected_head_id);
    Ok(evidence.durable_event && evidence.durable_branch_update)
}
