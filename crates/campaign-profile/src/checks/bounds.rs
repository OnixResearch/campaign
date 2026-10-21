//! `bounds.maximum_*` keys mapped one to one onto `campaign_core::ExplorationBounds`.

/// An exported bound key with its value.
type NamedBound = (&'static str, u64);

impl crate::wire::BoundsDocument {
    /// Every bound with its exported key, in `ExplorationBounds` framing order.
    const fn named(&self) -> [NamedBound; 17] {
        [
            ("maximum_roots", self.maximum_roots),
            ("maximum_candidates", self.maximum_candidates),
            ("maximum_candidate_batch", self.maximum_candidate_batch),
            (
                "maximum_retained_candidates",
                self.maximum_retained_candidates,
            ),
            ("maximum_selections", self.maximum_selections),
            ("maximum_guidance_records", self.maximum_guidance_records),
            (
                "maximum_outstanding_selections",
                self.maximum_outstanding_selections,
            ),
            ("maximum_depth", self.maximum_depth),
            (
                "maximum_children_per_result",
                self.maximum_children_per_result,
            ),
            ("maximum_derived_records", self.maximum_derived_records),
            ("maximum_rank_dimensions", self.maximum_rank_dimensions),
            ("maximum_rank_magnitude", self.maximum_rank_magnitude),
            ("maximum_finding_records", self.maximum_finding_records),
            ("maximum_control_events", self.maximum_control_events),
            ("maximum_budget_dimensions", self.maximum_budget_dimensions),
            ("maximum_operation_schemas", self.maximum_operation_schemas),
            ("maximum_prune_batch", self.maximum_prune_batch),
        ]
    }

    /// Containment relations the Nickel `ProfileRelations` contract enforces: (bound, containing bound).
    const fn contained(&self) -> [(NamedBound, NamedBound); 4] {
        let candidates = ("maximum_candidates", self.maximum_candidates);
        [
            (
                ("maximum_candidate_batch", self.maximum_candidate_batch),
                candidates,
            ),
            (
                (
                    "maximum_retained_candidates",
                    self.maximum_retained_candidates,
                ),
                candidates,
            ),
            (
                (
                    "maximum_outstanding_selections",
                    self.maximum_outstanding_selections,
                ),
                ("maximum_selections", self.maximum_selections),
            ),
            (
                ("maximum_prune_batch", self.maximum_prune_batch),
                candidates,
            ),
        ]
    }

    /// Checks every bound and containment relation, then derives `bounds_id` through
    /// `ExplorationBounds::derive`.
    // r[impl campaign.configuration.invalid]
    pub(crate) fn into_bounds(
        self,
    ) -> crate::failure::ProfileResult<campaign_core::ExplorationBounds> {
        let named = self.named();
        if let Some((bound, _)) = named.iter().find(|(_, value)| *value == 0) {
            return Err(crate::failure::ProfileError::ZeroBound { bound });
        }
        for (bound, limit) in self.contained() {
            if bound.1 > limit.1 {
                return Err(crate::failure::ProfileError::BoundExceedsLimit {
                    bound: bound.0,
                    limit: limit.0,
                });
            }
        }
        if self.maximum_rank_magnitude > campaign_core::MAX_RANK_MAGNITUDE {
            return Err(crate::failure::ProfileError::RankMagnitudeOutOfRange);
        }
        let bounds = campaign_core::ExplorationBounds {
            bounds_id: campaign_core::Id::absent(),
            max_roots: self.maximum_roots,
            max_candidates: self.maximum_candidates,
            max_candidate_batch: self.maximum_candidate_batch,
            max_retained_candidates: self.maximum_retained_candidates,
            max_selections: self.maximum_selections,
            max_guidance_records: self.maximum_guidance_records,
            max_outstanding: self.maximum_outstanding_selections,
            max_depth: self.maximum_depth,
            max_child_moments: self.maximum_children_per_result,
            max_derived_records: self.maximum_derived_records,
            max_rank_dimensions: self.maximum_rank_dimensions,
            max_rank_magnitude: self.maximum_rank_magnitude,
            max_findings: self.maximum_finding_records,
            max_control_events: self.maximum_control_events,
            max_resource_dimensions: self.maximum_budget_dimensions,
            max_operation_schemas: self.maximum_operation_schemas,
            max_prune_batch: self.maximum_prune_batch,
        }
        .derive()?;
        debug_assert!(!bounds.bounds_id.is_absent());
        Ok(bounds)
    }
}
