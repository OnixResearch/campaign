const LABEL_CAMPAIGN: &[u8] = b"campaign-admission";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.identity]
pub struct CampaignAdmission {
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Campaign admission schema version; must equal `CAMPAIGN_SCHEMA_VERSION`.
    pub schema_version: u32,
    /// Public Campaign core API value.
    pub projection: super::projection::ProjectionBinding,
    /// Public Campaign core API value.
    pub policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub guidance_profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub entropy_profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub budget_profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub adapter_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub control_domain_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub bounds: super::bounds::ExplorationBounds,
    /// Public Campaign core API value.
    pub resource_budgets: alloc::vec::Vec<super::bounds::ResourceAmount>,
    /// Public Campaign core API value.
    pub roots: alloc::vec::Vec<super::projection::RootMoment>,
    /// Canonically sorted operation schemas that candidates may declare.
    pub operation_schema_ids: alloc::vec::Vec<crate::identity::Id>,
}

impl CampaignAdmission {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    // r[impl campaign.identity]
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.validate_without_campaign_id()?;
        let mut roots = self.roots.clone();
        roots.sort_by_key(|root| root.moment_id);
        let mut budgets = self.resource_budgets.clone();
        budgets.sort_by_key(|budget| budget.dimension_id);
        self.roots = roots;
        self.resource_budgets = budgets;
        self.operation_schema_ids.sort();
        self.campaign_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Campaign,
            LABEL_CAMPAIGN,
            |mut writer| {
                writer = writer.u32(self.schema_version);
                writer = writer.id(self.projection.projection_id);
                writer = writer.id(self.policy_id);
                writer = writer.id(self.guidance_profile_id);
                writer = writer.id(self.entropy_profile_id);
                writer = writer.id(self.budget_profile_id);
                writer = writer.id(self.adapter_id);
                writer = writer.id(self.control_domain_id);
                writer = writer.id(self.bounds.bounds_id);
                writer = super::bounds::write_resources(writer, &self.resource_budgets)?;
                writer = writer.seq_len(crate::identity::usize_to_u64(self.roots.len())?);
                for root in &self.roots {
                    writer = writer.id(root.moment_id);
                }
                writer = super::snapshots::write_ids(writer, &self.operation_schema_ids)?;
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
    pub fn validate(&self) -> crate::error::CampaignResult<()> {
        self.campaign_id
            .require_kind(crate::identity::IdKind::Campaign)?;
        let derived = self.clone().derive()?;
        if &derived != self {
            return Err(crate::error::CampaignError::MalformedIdentity);
        }
        Ok(())
    }

    // r[impl campaign.initial_admission]
    fn validate_without_campaign_id(&self) -> crate::error::CampaignResult<()> {
        if self.schema_version != super::CAMPAIGN_SCHEMA_VERSION {
            return Err(crate::error::CampaignError::UnsupportedVersion);
        }
        self.projection.validate()?;
        self.policy_id
            .require_kind(crate::identity::IdKind::Policy)?;
        self.guidance_profile_id
            .require_kind(crate::identity::IdKind::GuidanceProfile)?;
        self.entropy_profile_id
            .require_kind(crate::identity::IdKind::EntropyProfile)?;
        self.budget_profile_id
            .require_kind(crate::identity::IdKind::BudgetProfile)?;
        self.adapter_id
            .require_kind(crate::identity::IdKind::Adapter)?;
        self.control_domain_id
            .require_kind(crate::identity::IdKind::ControlDomain)?;
        if self.adapter_id != self.projection.adapter_id
            || self.control_domain_id != self.projection.control_domain_id
        {
            return Err(crate::error::CampaignError::CrossedIdentity);
        }
        self.bounds.validate()?;
        super::bounds::check_len_at_least_one(
            self.roots.len(),
            self.bounds.max_roots,
            crate::error::CampaignError::EmptyRoots,
        )?;
        let mut root_ids = alloc::vec::Vec::with_capacity(self.roots.len());
        for root in &self.roots {
            root.validate(&self.projection)?;
            root_ids.push(root.moment_id);
        }
        root_ids.sort();
        if !crate::identity::unique_sorted_ids(&root_ids) {
            return Err(crate::error::CampaignError::DuplicateRoot);
        }
        super::bounds::check_len(
            self.resource_budgets.len(),
            self.bounds.max_resource_dimensions,
        )?;
        let mut budget_ids = alloc::vec::Vec::with_capacity(self.resource_budgets.len());
        for budget in &self.resource_budgets {
            budget.validate()?;
            budget_ids.push(budget.dimension_id);
        }
        budget_ids.sort();
        if !crate::identity::unique_sorted_ids(&budget_ids) {
            return Err(crate::error::CampaignError::DuplicateIdentity);
        }
        self.validate_operation_schemas()
    }

    fn validate_operation_schemas(&self) -> crate::error::CampaignResult<()> {
        super::bounds::check_len_at_least_one(
            self.operation_schema_ids.len(),
            self.bounds.max_operation_schemas,
            crate::error::CampaignError::UnsupportedSchema,
        )?;
        for schema_id in &self.operation_schema_ids {
            schema_id.require_kind(crate::identity::IdKind::OperationSchema)?;
        }
        let sorted = crate::identity::sorted_copy(&self.operation_schema_ids);
        if !crate::identity::unique_sorted_ids(&sorted) {
            return Err(crate::error::CampaignError::DuplicateIdentity);
        }
        debug_assert!(!sorted.is_empty());
        Ok(())
    }

    /// Returns true when the campaign admitted the operation schema.
    ///
    /// Binary-searches the canonical (sorted) schema order that `derive` produces and every
    /// current frontier state carries, in logarithmic time.
    #[must_use]
    pub fn supports_operation_schema(&self, schema_id: crate::identity::Id) -> bool {
        self.operation_schema_ids.binary_search(&schema_id).is_ok()
    }
}

/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
pub fn derive_campaign_event_id(
    admission: &CampaignAdmission,
) -> crate::error::CampaignResult<crate::identity::Id> {
    super::events::derive_event_id(
        &super::events::ControlEventKind::CampaignAdmitted,
        admission.campaign_id,
        crate::identity::Id::absent(),
        &[
            admission.projection.projection_id,
            admission.bounds.bounds_id,
        ],
    )
}
