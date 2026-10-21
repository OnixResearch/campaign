#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Answer to one differential-evidence question that the core may not be able to measure.
pub enum EvidenceQuestion {
    /// The question was not measured; no equality may be inferred.
    NotMeasured,
    /// The question was measured by its owner.
    Measured {
        /// Whether the measured values were equal.
        equal: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Differential comparison that keeps field identity, semantic work, time, and memory separate.
// r[impl campaign.fd.evidence]
pub struct DifferentialReport {
    /// Whether every decision field except planning work is equal.
    pub field_identity_equal: bool,
    /// Whether the reported semantic planning work is equal; `None` when not measured.
    pub semantic_work_equal: Option<bool>,
    /// Wall-clock time question; the pure core never measures time.
    pub time: EvidenceQuestion,
    /// Memory question; the pure core never measures memory.
    pub memory: EvidenceQuestion,
}

/// Compares a production decision with a reference decision, answering field identity and
/// semantic work as separate questions from each plan's own reported work.
#[must_use]
// r[impl campaign.fd.evidence]
pub fn compare_decisions(
    production: &crate::model::DecisionPlan,
    reference: &crate::model::DecisionPlan,
) -> DifferentialReport {
    let is_field_identity_equal = production.campaign_id == reference.campaign_id
        && production.state_id == reference.state_id
        && production.decision == reference.decision
        && production.scheduling_policy_id == reference.scheduling_policy_id
        && production.admission_policy_id == reference.admission_policy_id
        && production.coalescing_policy_id == reference.coalescing_policy_id;
    DifferentialReport {
        field_identity_equal: is_field_identity_equal,
        semantic_work_equal: Some(production.work == reference.work),
        time: EvidenceQuestion::NotMeasured,
        memory: EvidenceQuestion::NotMeasured,
    }
}

/// Validates a differential report against the two decisions it describes.
///
/// # Errors
///
/// Returns `ConflatedEvidenceQuestion` when the report leaves semantic work unanswered while
/// fields are equal, when its field or work answer differs from the separately measured answer
/// (for example work equality copied from field equality), or when it answers time or memory,
/// which the pure core never measures.
// r[impl campaign.fd.evidence]
pub fn validate_differential_evidence(
    report: &DifferentialReport,
    production: &crate::model::DecisionPlan,
    reference: &crate::model::DecisionPlan,
) -> crate::error::CampaignResult<()> {
    let measured = compare_decisions(production, reference);
    if report.field_identity_equal && report.semantic_work_equal.is_none() {
        return Err(crate::error::CampaignError::ConflatedEvidenceQuestion);
    }
    if report.field_identity_equal != measured.field_identity_equal {
        return Err(crate::error::CampaignError::ConflatedEvidenceQuestion);
    }
    if report
        .semantic_work_equal
        .is_some_and(|equal| Some(equal) != measured.semantic_work_equal)
    {
        return Err(crate::error::CampaignError::ConflatedEvidenceQuestion);
    }
    if report.time != measured.time || report.memory != measured.memory {
        return Err(crate::error::CampaignError::ConflatedEvidenceQuestion);
    }
    debug_assert_eq!(report.field_identity_equal, measured.field_identity_equal);
    Ok(())
}
