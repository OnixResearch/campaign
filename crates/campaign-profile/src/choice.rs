//! Declared selection facts and the runtime identities needed to derive a `SelectionProfile`.

/// Selection mode as the profile declares it; the exploratory entropy source is a runtime identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclaredSelectionMode {
    /// Candidates are chosen by rank only.
    Ranked,
    /// Candidates are chosen by rank or, below the threshold, by the exploration order.
    Exploratory {
        /// Exploration threshold in parts per million, at most `campaign_core::ENTROPY_SCALE`.
        threshold_parts_per_million: u64,
        /// Non-empty, unique rank-dimension indices below `maximum_rank_dimensions`.
        exploration_order: Vec<u32>,
    },
}

/// Runtime identities a profile cannot know, supplied by the consumer when it builds a `SelectionProfile`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionRuntime {
    /// Compiled production policy identity (`IdKind::Policy`).
    pub production_policy_id: campaign_core::Id,
    /// Scheduling policy identity (`IdKind::SchedulingProfile`).
    pub scheduling_policy_id: campaign_core::Id,
    /// Coalescing policy identity (`IdKind::CoalescingPolicy`).
    pub coalescing_policy_id: campaign_core::Id,
    /// Entropy source identity (`IdKind::EntropyProfile`); required exactly for exploratory profiles.
    pub entropy_source_id: Option<campaign_core::Id>,
}

/// Declared selection facts after validation.
pub(crate) struct DeclaredSelection {
    pub(crate) mode: DeclaredSelectionMode,
    pub(crate) decay: campaign_core::SelectionCountDecay,
}

impl crate::wire::SelectionDocument {
    /// Checks the mode-dependent keys, threshold scale, exploration order, decay penalty, and that the decay
    /// stays representable under `bounds`.
    pub(crate) fn into_declared(
        self,
        bounds: &campaign_core::ExplorationBounds,
    ) -> crate::failure::ProfileResult<DeclaredSelection> {
        let crate::wire::TieBreakTag::CandidateIdentity = self.tie_break;
        let mode = match (
            self.mode,
            self.exploratory_threshold_parts_per_million,
            self.exploration_order,
        ) {
            (crate::wire::SelectionModeTag::Ranked, None, None) => DeclaredSelectionMode::Ranked,
            (crate::wire::SelectionModeTag::Exploratory, Some(threshold), Some(order)) => {
                if threshold > campaign_core::ENTROPY_SCALE {
                    return Err(crate::failure::ProfileError::ThresholdOverScale);
                }
                DeclaredSelectionMode::Exploratory {
                    threshold_parts_per_million: threshold,
                    exploration_order: crate::checks::ordering::validate(
                        crate::checks::ordering::EXPLORATION_ORDER,
                        order,
                        bounds.max_rank_dimensions,
                    )?,
                }
            }
            _ => return Err(crate::failure::ProfileError::ExploratoryFieldsMismatch),
        };
        let decay = match (self.decay.rule, self.decay.penalty_per_selection) {
            (crate::wire::DecayRuleTag::None, None) => campaign_core::SelectionCountDecay::None,
            (crate::wire::DecayRuleTag::SubtractFromFirstRank, Some(0)) => {
                return Err(crate::failure::ProfileError::ZeroDecayPenalty);
            }
            (crate::wire::DecayRuleTag::SubtractFromFirstRank, Some(penalty_per_selection)) => {
                campaign_core::SelectionCountDecay::SubtractFromFirstRank {
                    penalty_per_selection,
                }
            }
            _ => return Err(crate::failure::ProfileError::DecayPenaltyMismatch),
        };
        if let Err(error) = decay.validate_within_bounds(bounds) {
            debug_assert_eq!(error, campaign_core::CampaignError::InvalidConfig);
            return Err(crate::failure::ProfileError::DecayExceedsRankDomain);
        }
        Ok(DeclaredSelection { mode, decay })
    }
}

impl DeclaredSelectionMode {
    /// Binds the runtime entropy source into the core selection mode.
    pub(crate) fn with_entropy_source(
        &self,
        entropy_source_id: Option<campaign_core::Id>,
    ) -> crate::failure::ProfileResult<campaign_core::SelectionMode> {
        match (self, entropy_source_id) {
            (Self::Ranked, None) => Ok(campaign_core::SelectionMode::Ranked),
            (Self::Ranked, Some(_)) => Err(crate::failure::ProfileError::UnexpectedEntropySource),
            (Self::Exploratory { .. }, None) => {
                Err(crate::failure::ProfileError::MissingEntropySource)
            }
            (
                Self::Exploratory {
                    threshold_parts_per_million,
                    exploration_order,
                },
                Some(entropy_source_id),
            ) => Ok(campaign_core::SelectionMode::Exploratory {
                threshold_parts_per_million: *threshold_parts_per_million,
                entropy_source_id,
                exploration_order: exploration_order.clone(),
            }),
        }
    }
}
