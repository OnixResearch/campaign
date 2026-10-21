#[cfg(test)]
mod tests;

const LABEL_BOUNDS: &[u8] = b"exploration-bounds-v2";

pub(crate) const BOUND_COUNT: usize = 17;

// r[impl campaign.budgets]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Named exploration bounds; every value is at least one and is framed into `bounds_id` in
/// field order. Public Campaign core API value.
pub struct ExplorationBounds {
    /// Content-derived bounds identity.
    pub bounds_id: crate::identity::Id,
    /// Maximum admitted root moments.
    pub max_roots: u64,
    /// Maximum candidates ever admitted to the campaign.
    pub max_candidates: u64,
    /// Maximum candidates in one admission batch.
    pub max_candidate_batch: u64,
    /// Retained-candidate pruning target; it does not limit admission.
    pub max_retained_candidates: u64,
    /// Maximum selections ever published.
    pub max_selections: u64,
    /// Maximum counted guidance records.
    pub max_guidance_records: u64,
    /// Maximum simultaneously outstanding selections.
    pub max_outstanding: u64,
    /// Maximum candidate depth below a root moment.
    pub max_depth: u64,
    /// Maximum child moments in one result.
    pub max_child_moments: u64,
    /// Maximum derived records in one result.
    pub max_derived_records: u64,
    /// Maximum guidance rank dimensions.
    pub max_rank_dimensions: u64,
    /// Maximum absolute guidance rank value, at most [`MAX_RANK_MAGNITUDE`].
    pub max_rank_magnitude: u64,
    /// Maximum findings over the whole campaign.
    pub max_findings: u64,
    /// Maximum control events in the campaign history.
    pub max_control_events: u64,
    /// Maximum resource budget dimensions.
    pub max_resource_dimensions: u64,
    /// Maximum admitted operation schemas.
    pub max_operation_schemas: u64,
    /// Maximum candidates pruned by one event.
    pub max_prune_batch: u64,
}

/// Smallest control-event bound: the admission event plus the reserve for one stop.
pub const MIN_CONTROL_EVENTS: u64 = 2;

/// Largest declarable `max_rank_magnitude`: every admitted rank and its negation are `i64` values.
pub const MAX_RANK_MAGNITUDE: u64 = i64::MAX.unsigned_abs();

impl ExplorationBounds {
    /// Derives the content identity after checking that every bound is at least one.
    ///
    /// # Errors
    ///
    /// Returns `InvalidConfig` when any bound is zero, `max_rank_magnitude` exceeds
    /// [`MAX_RANK_MAGNITUDE`], or `max_control_events` leaves no room for the admission event and a stop.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.validate_values_without_id()?;
        if self.max_control_events < MIN_CONTROL_EVENTS
            || self.max_rank_magnitude > MAX_RANK_MAGNITUDE
        {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        self.bounds_id = crate::identity::derive_id_with(
            crate::identity::IdKind::BudgetProfile,
            LABEL_BOUNDS,
            |mut writer| {
                writer = writer.u64(self.max_roots);
                writer = writer.u64(self.max_candidates);
                writer = writer.u64(self.max_candidate_batch);
                writer = writer.u64(self.max_retained_candidates);
                writer = writer.u64(self.max_selections);
                writer = writer.u64(self.max_guidance_records);
                writer = writer.u64(self.max_outstanding);
                writer = writer.u64(self.max_depth);
                writer = writer.u64(self.max_child_moments);
                writer = writer.u64(self.max_derived_records);
                writer = writer.u64(self.max_rank_dimensions);
                writer = writer.u64(self.max_rank_magnitude);
                writer = writer.u64(self.max_findings);
                writer = writer.u64(self.max_control_events);
                writer = writer.u64(self.max_resource_dimensions);
                writer = writer.u64(self.max_operation_schemas);
                writer = writer.u64(self.max_prune_batch);
                Ok(writer)
            },
        )?;
        Ok(self)
    }

    /// Re-derives the bounds identity.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when the identity is absent, crossed, or does not match the values.
    pub fn validate(&self) -> crate::error::CampaignResult<()> {
        self.bounds_id
            .require_kind(crate::identity::IdKind::BudgetProfile)?;
        let derived = self.clone().derive()?;
        if derived.bounds_id != self.bounds_id {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        Ok(())
    }

    /// Returns every bound in its framing order.
    pub(crate) const fn values(&self) -> [u64; BOUND_COUNT] {
        [
            self.max_roots,
            self.max_candidates,
            self.max_candidate_batch,
            self.max_retained_candidates,
            self.max_selections,
            self.max_guidance_records,
            self.max_outstanding,
            self.max_depth,
            self.max_child_moments,
            self.max_derived_records,
            self.max_rank_dimensions,
            self.max_rank_magnitude,
            self.max_findings,
            self.max_control_events,
            self.max_resource_dimensions,
            self.max_operation_schemas,
            self.max_prune_batch,
        ]
    }

    fn validate_values_without_id(&self) -> crate::error::CampaignResult<()> {
        if self.values().contains(&0) {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct ResourceAmount {
    /// Public Campaign core API value.
    pub dimension_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub amount: u64,
}

impl ResourceAmount {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn validate(&self) -> crate::error::CampaignResult<()> {
        self.dimension_id
            .require_kind(crate::identity::IdKind::ResourceDimension)?;
        Ok(())
    }
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
pub fn write_resources(
    mut writer: crate::identity::FrameWriter,
    resources: &[ResourceAmount],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    writer = writer.seq_len(crate::identity::usize_to_u64(resources.len())?);
    for resource in resources {
        writer = writer.id(resource.dimension_id);
        writer = writer.u64(resource.amount);
    }
    Ok(writer)
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
#[cfg_attr(
    dylint_lib = "octet",
    allow(
        tigerstyle::usize_in_public_api,
        reason = "Campaign MODEL owner: this stable boundary accepts native collection lengths and checks conversion to u64"
    )
)]
pub fn check_len(len: usize, max: u64) -> crate::error::CampaignResult<()> {
    if crate::identity::usize_to_u64(len)? > max {
        return Err(crate::error::CampaignError::BoundExceeded);
    }
    Ok(())
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
#[cfg_attr(
    dylint_lib = "octet",
    allow(
        tigerstyle::usize_in_public_api,
        reason = "Campaign MODEL owner: preserve the native-length API and its typed empty-collection error"
    )
)]
pub fn check_len_at_least_one(
    len: usize,
    max: u64,
    empty_error: crate::error::CampaignError,
) -> crate::error::CampaignResult<()> {
    if len == super::EMPTY_COUNT {
        return Err(empty_error);
    }
    check_len(len, max)
}
