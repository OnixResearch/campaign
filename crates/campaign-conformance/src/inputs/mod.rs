//! Caller-supplied suite profile: Campaign bounds and profile inputs.

/// Smallest bound values the declared scenarios need.
const REQUIRED_CANDIDATES: u64 = 8;
const REQUIRED_CANDIDATE_BATCH: u64 = 3;
const REQUIRED_RETAINED: u64 = 4;
const REQUIRED_SELECTIONS: u64 = 4;
const REQUIRED_GUIDANCE_RECORDS: u64 = 12;
const REQUIRED_OUTSTANDING: u64 = 2;
const REQUIRED_DEPTH: u64 = 3;
const REQUIRED_CHILD_MOMENTS: u64 = 2;
const REQUIRED_FINDINGS: u64 = 3;
const REQUIRED_CONTROL_EVENTS: u64 = 24;
const REQUIRED_PRUNE_BATCH: u64 = 2;
const REQUIRED_RANK_MAGNITUDE: u64 = 1_000_000;

/// Smallest exploratory threshold with an explore draw strictly between zero and the threshold.
const REQUIRED_THRESHOLD: u64 = 2;

/// Bounds and profile inputs the suite runs under.
///
/// Campaign's own tests build this value explicitly; the profile loader will produce it from the
/// checked `campaign.profile.v2` export. Scenario-specific limits (for example a one-slot
/// outstanding bound for the capacity wait) are applied by each fixture on top of these bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuiteProfile {
    /// Exploration bounds; re-derived by the suite after each fixture's declared overrides.
    pub bounds: campaign_core::ExplorationBounds,
    /// Declared duplicate-candidate rule of ordinary candidate batches.
    pub duplicate_candidate: campaign_core::DuplicateCandidateRule,
    /// Exploratory threshold in parts per million; must allow both entropy branches and an explore
    /// draw strictly between zero and the threshold.
    pub exploratory_threshold_parts_per_million: u64,
    /// Declared exploration order (rank-dimension indices).
    pub exploration_order: alloc::vec::Vec<u32>,
    /// Declared retention rank order (rank-dimension indices).
    pub retention_rank_order: alloc::vec::Vec<u32>,
}

impl SuiteProfile {
    /// Checks that the profile can express every declared scenario.
    ///
    /// # Errors
    ///
    /// Returns `InvalidProfile` naming the first unmet requirement.
    pub fn validate(&self) -> crate::verdicts::ConformanceResult<()> {
        let bounds = &self.bounds;
        let minimums = [
            (bounds.max_candidates, REQUIRED_CANDIDATES, "max_candidates"),
            (
                bounds.max_candidate_batch,
                REQUIRED_CANDIDATE_BATCH,
                "max_candidate_batch",
            ),
            (
                bounds.max_retained_candidates,
                REQUIRED_RETAINED,
                "max_retained_candidates",
            ),
            (bounds.max_selections, REQUIRED_SELECTIONS, "max_selections"),
            (
                bounds.max_guidance_records,
                REQUIRED_GUIDANCE_RECORDS,
                "max_guidance_records",
            ),
            (
                bounds.max_outstanding,
                REQUIRED_OUTSTANDING,
                "max_outstanding",
            ),
            (bounds.max_depth, REQUIRED_DEPTH, "max_depth"),
            (
                bounds.max_child_moments,
                REQUIRED_CHILD_MOMENTS,
                "max_child_moments",
            ),
            (bounds.max_findings, REQUIRED_FINDINGS, "max_findings"),
            (
                bounds.max_control_events,
                REQUIRED_CONTROL_EVENTS,
                "max_control_events",
            ),
            (
                bounds.max_prune_batch,
                REQUIRED_PRUNE_BATCH,
                "max_prune_batch",
            ),
            (
                bounds.max_rank_magnitude,
                REQUIRED_RANK_MAGNITUDE,
                "max_rank_magnitude",
            ),
            (
                bounds.max_rank_dimensions,
                u64::from(crate::vocabulary::SCENARIO_RANK_DIMENSIONS),
                "max_rank_dimensions",
            ),
        ];
        for (value, required, requirement) in minimums {
            if value < required {
                return Err(crate::verdicts::ConformanceError::InvalidProfile { requirement });
            }
        }
        self.validate_orders()?;
        debug_assert!(bounds.max_outstanding >= REQUIRED_OUTSTANDING);
        debug_assert!(!self.exploration_order.is_empty());
        Ok(())
    }

    fn validate_orders(&self) -> crate::verdicts::ConformanceResult<()> {
        let threshold = self.exploratory_threshold_parts_per_million;
        if !(REQUIRED_THRESHOLD..campaign_core::ENTROPY_SCALE).contains(&threshold) {
            return Err(crate::verdicts::ConformanceError::InvalidProfile {
                requirement: "exploratory threshold must allow both entropy branches and an explore draw strictly inside the explore range",
            });
        }
        for order in [&self.exploration_order, &self.retention_rank_order] {
            let is_within = order
                .iter()
                .all(|index| *index < crate::vocabulary::SCENARIO_RANK_DIMENSIONS);
            if order.is_empty() || !is_within {
                return Err(crate::verdicts::ConformanceError::InvalidProfile {
                    requirement: "declared orders must name scenario rank dimensions",
                });
            }
        }
        Ok(())
    }
}
