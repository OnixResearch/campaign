const LABEL_PROJECTION: &[u8] = b"projection-binding";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.history_projection]
pub struct ProjectionBinding {
    /// Public Campaign core API value.
    pub projection_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub graph_id: crate::identity::Id,
    /// Identity of the exact control-branch value (name, head, and generation).
    pub control_branch_id: crate::identity::Id,
    /// Stable identity of the control-branch name: equal for every value of one named branch,
    /// so a projection can be tied to the admission's branch across fast-forward advances.
    pub control_branch_name_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub generation: u64,
    /// Public Campaign core API value.
    pub head_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub projection_schema_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub adapter_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub control_domain_id: crate::identity::Id,
}

impl ProjectionBinding {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.validate_kinds_without_projection()?;
        self.projection_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Projection,
            LABEL_PROJECTION,
            |mut writer| {
                writer = writer.id(self.graph_id);
                writer = writer.id(self.control_branch_id);
                writer = writer.id(self.control_branch_name_id);
                writer = writer.u64(self.generation);
                writer = writer.id(self.head_id);
                writer = writer.id(self.projection_schema_id);
                writer = writer.id(self.adapter_id);
                writer = writer.id(self.control_domain_id);
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
        self.projection_id
            .require_kind(crate::identity::IdKind::Projection)?;
        let derived = self.clone().derive()?;
        if derived.projection_id != self.projection_id {
            return Err(crate::error::CampaignError::StaleProjection);
        }
        Ok(())
    }

    fn validate_kinds_without_projection(&self) -> crate::error::CampaignResult<()> {
        self.graph_id.require_kind(crate::identity::IdKind::Graph)?;
        self.control_branch_id
            .require_kind(crate::identity::IdKind::ControlBranch)?;
        self.control_branch_name_id
            .require_kind(crate::identity::IdKind::ControlBranch)?;
        self.head_id.require_kind(crate::identity::IdKind::Head)?;
        self.projection_schema_id
            .require_kind(crate::identity::IdKind::Projection)?;
        self.adapter_id
            .require_kind(crate::identity::IdKind::Adapter)?;
        self.control_domain_id
            .require_kind(crate::identity::IdKind::ControlDomain)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.initial_admission]
pub struct RootMoment {
    /// Public Campaign core API value.
    pub moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub projection_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub adapter_id: crate::identity::Id,
}

impl RootMoment {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn validate(&self, projection: &ProjectionBinding) -> crate::error::CampaignResult<()> {
        self.moment_id
            .require_kind(crate::identity::IdKind::RootMoment)?;
        self.projection_id
            .require_kind(crate::identity::IdKind::Projection)?;
        self.adapter_id
            .require_kind(crate::identity::IdKind::Adapter)?;
        if self.projection_id != projection.projection_id
            || self.adapter_id != projection.adapter_id
        {
            return Err(crate::error::CampaignError::CrossedIdentity);
        }
        Ok(())
    }
}
