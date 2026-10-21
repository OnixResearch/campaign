//! Spec rule identities attached to checker violations.

/// A Campaign specification scenario whose validation rule rejected the repository.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Rule {
    spec_id: &'static str,
}

impl Rule {
    pub(crate) const SOURCE_INVENTORY_COMPLETE: Self =
        Self::new("campaign.source_inventory.complete");
    pub(crate) const SOURCE_INVENTORY_MISSING: Self =
        Self::new("campaign.source_inventory.missing");
    pub(crate) const CHOREGRAPH_DEPENDENCY_VALID: Self =
        Self::new("campaign.choregraph_dependency.valid");
    pub(crate) const CHOREGRAPH_DEPENDENCY_MOVING: Self =
        Self::new("campaign.choregraph_dependency.moving");
    pub(crate) const BOUNDARY_SHARED: Self = Self::new("campaign.boundary.shared");
    pub(crate) const BOUNDARY_HISTORY_LEAK: Self = Self::new("campaign.boundary.history_leak");
    pub(crate) const BOUNDARY_CONSUMER_LEAK: Self = Self::new("campaign.boundary.consumer_leak");
    pub(crate) const FUNCTIONAL_CORE_EFFECT: Self = Self::new("campaign.functional_core.effect");
    pub(crate) const REPLAY_BOUNDARY_INVALID: Self = Self::new("campaign.replay_boundary.invalid");
    pub(crate) const FINDINGS_SEMANTIC_LEAK: Self = Self::new("campaign.findings.semantic_leak");
    pub(crate) const NON_VM_FOREIGN_TYPES: Self =
        Self::new("campaign.non_vm_candidate_fixture.foreign_types");
    pub(crate) const CONSUMER_PILOTS_LEAK: Self = Self::new("campaign.consumer_pilots.leak");
    pub(crate) const SEED_RUNNER_BOUNDARY: Self =
        Self::new("campaign.chaoscontrol_migration.seed_runner");
    pub(crate) const CONFIGURATION_STALE: Self = Self::new("campaign.configuration.stale");
    pub(crate) const CONSUMER_PILOTS_VALID: Self = Self::new("campaign.consumer_pilots.valid");
    /// The requirement statement itself: a migrated consumer pins one immutable Campaign revision.
    pub(crate) const CHAOSCONTROL_MIGRATION: Self = Self::new("campaign.chaoscontrol_migration");
    pub(crate) const CHAOSCONTROL_MIGRATION_PARITY: Self =
        Self::new("campaign.chaoscontrol_migration.parity");
    pub(crate) const CHAOSCONTROL_MIGRATION_AUTHORITY: Self =
        Self::new("campaign.chaoscontrol_migration.authority");

    /// Every rule, so fixture tests can prove each one has a negative control.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 18] = [
        Self::SOURCE_INVENTORY_COMPLETE,
        Self::SOURCE_INVENTORY_MISSING,
        Self::CHOREGRAPH_DEPENDENCY_VALID,
        Self::CHOREGRAPH_DEPENDENCY_MOVING,
        Self::BOUNDARY_SHARED,
        Self::BOUNDARY_HISTORY_LEAK,
        Self::BOUNDARY_CONSUMER_LEAK,
        Self::FUNCTIONAL_CORE_EFFECT,
        Self::REPLAY_BOUNDARY_INVALID,
        Self::FINDINGS_SEMANTIC_LEAK,
        Self::NON_VM_FOREIGN_TYPES,
        Self::CONSUMER_PILOTS_LEAK,
        Self::SEED_RUNNER_BOUNDARY,
        Self::CONFIGURATION_STALE,
        Self::CONSUMER_PILOTS_VALID,
        Self::CHAOSCONTROL_MIGRATION,
        Self::CHAOSCONTROL_MIGRATION_PARITY,
        Self::CHAOSCONTROL_MIGRATION_AUTHORITY,
    ];

    const fn new(spec_id: &'static str) -> Self {
        Self { spec_id }
    }

    pub(crate) const fn spec_id(self) -> &'static str {
        self.spec_id
    }
}
