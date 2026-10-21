//! Declared candidate admission rule and per-candidate admission outcomes.

const LABEL_ADMISSION_RULE: &[u8] = b"candidate-admission-rule";
const LABEL_ADMISSION_OUTCOMES: &[u8] = b"candidate-admission-outcomes";
const DUPLICATE_REJECT: u32 = 1;
const DUPLICATE_COALESCE: u32 = 2;
const OUTCOME_ADMITTED: u32 = 1;
const OUTCOME_COALESCED_DUPLICATE: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Declared treatment of a candidate whose identity is already admitted or repeated in its batch.
pub enum DuplicateCandidateRule {
    /// Any duplicate denies the whole batch with `DuplicateIdentity`.
    Reject,
    /// Duplicates are removed from the batch and recorded as coalesced outcomes.
    Coalesce,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Declared candidate admission rule; its content derives the admission policy identity.
// r[impl campaign.fd.admission]
pub struct AdmissionRule {
    /// Declared duplicate-candidate treatment.
    pub duplicate_candidate: DuplicateCandidateRule,
}

impl AdmissionRule {
    /// Derives the content-addressed admission policy identity of this rule.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when identity framing fails.
    pub fn admission_policy_id(self) -> crate::error::CampaignResult<crate::identity::Id> {
        let code = if self.duplicate_candidate == DuplicateCandidateRule::Coalesce {
            DUPLICATE_COALESCE
        } else {
            DUPLICATE_REJECT
        };
        crate::identity::derive_id_with(
            crate::identity::IdKind::AdmissionPolicy,
            LABEL_ADMISSION_RULE,
            |writer| Ok(writer.u32(code)),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Admission decision recorded for one submitted candidate.
pub enum AdmissionDecision {
    /// The candidate entered the frontier.
    Admitted,
    /// The candidate duplicated an admitted identity and was coalesced into it.
    CoalescedDuplicate {
        /// Existing admitted candidate identity that absorbed the duplicate.
        existing_candidate_id: crate::identity::Id,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Policy-visible admission outcome for one submitted candidate.
pub struct AdmissionOutcome {
    /// Submitted candidate identity.
    pub candidate_id: crate::identity::Id,
    /// Decision taken by the declared admission rule.
    pub outcome: AdmissionDecision,
}

/// Public Campaign core API operation: derives the `AdmissionOutcomes` identity over a canonical outcome list.
///
/// # Errors
///
/// Returns a typed `CampaignError` when identity framing fails.
pub fn derive_admission_outcomes_id(
    outcomes: &[AdmissionOutcome],
) -> crate::error::CampaignResult<crate::identity::Id> {
    crate::identity::derive_id_with(
        crate::identity::IdKind::AdmissionOutcomes,
        LABEL_ADMISSION_OUTCOMES,
        |writer| {
            let mut writer = writer.seq_len(crate::identity::usize_to_u64(outcomes.len())?);
            for outcome in outcomes {
                writer = writer.id(outcome.candidate_id);
                writer = if let AdmissionDecision::CoalescedDuplicate {
                    existing_candidate_id,
                } = outcome.outcome
                {
                    writer
                        .u32(OUTCOME_COALESCED_DUPLICATE)
                        .id(existing_candidate_id)
                } else {
                    writer.u32(OUTCOME_ADMITTED)
                };
            }
            Ok(writer)
        },
    )
}
