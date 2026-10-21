//! Values the suite hands to an adapter mapping and the values an adapter returns.

/// Consumer identities bound into every campaign the suite admits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterIdentities {
    /// Campaign adapter identity (`Adapter`); binds the adapter's conversion profiles.
    pub adapter_id: campaign_core::Id,
    /// Campaign projection schema identity (`Projection`).
    pub projection_schema_id: campaign_core::Id,
    /// Campaign control-domain identity (`ControlDomain`).
    pub control_domain_id: campaign_core::Id,
    /// Guidance ordering profile (`GuidanceProfile`): the consumer's score-to-rank conversion.
    pub guidance_profile_id: campaign_core::Id,
    /// Explicit entropy source (`EntropyProfile`) whose tickets the adapter issues.
    pub entropy_source_id: campaign_core::Id,
    /// Declared operation schemas (`OperationSchema`), at least one.
    pub operation_schema_ids: alloc::vec::Vec<campaign_core::Id>,
    /// Declared resource dimensions (`ResourceDimension`), indexed by abstract cost dimension.
    pub resource_dimension_ids: alloc::vec::Vec<campaign_core::Id>,
}

/// Consumer identity of one admitted exploration moment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MomentIdentity {
    /// Root moment admitted with the campaign.
    Root {
        /// `RootMoment` identity.
        moment_id: campaign_core::Id,
    },
    /// Child moment discovered by an execution.
    Child {
        /// `ChildMoment` identity.
        moment_id: campaign_core::Id,
        /// `EventPayload` identity of the child's exploration event.
        event_payload_id: campaign_core::Id,
    },
}

impl MomentIdentity {
    /// Campaign moment identity of either role.
    #[must_use]
    pub const fn moment_id(&self) -> campaign_core::Id {
        let (Self::Root { moment_id } | Self::Child { moment_id, .. }) = self;
        *moment_id
    }
}

/// Campaign facts an operation mapping binds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateContext {
    /// Campaign identity.
    pub campaign_id: campaign_core::Id,
    /// Consumer identity of the operation's parent moment.
    pub parent_moment_id: campaign_core::Id,
    /// Required candidate depth (parent depth plus one).
    pub depth: u64,
}

/// Campaign facts a preference mapping binds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuidanceContext {
    /// Campaign identity.
    pub campaign_id: campaign_core::Id,
    /// Admitted candidate the preference ranks.
    pub candidate_id: campaign_core::Id,
}

/// Campaign facts an explicit revision binds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevisionContext {
    /// Campaign identity.
    pub campaign_id: campaign_core::Id,
    /// Admitted candidate whose latest guidance is revised.
    pub candidate_id: campaign_core::Id,
    /// Latest guidance record the revision invalidates.
    pub invalidated_guidance_id: campaign_core::Id,
}

/// Campaign facts an entropy ticket binds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntropyContext {
    /// Campaign identity.
    pub campaign_id: campaign_core::Id,
    /// Current state identity.
    pub state_id: campaign_core::Id,
    /// Selection profile identity of the request.
    pub selection_profile_id: campaign_core::Id,
    /// Consumer ticket counter (the next selection ordinal).
    pub counter: u64,
}

/// Campaign facts an outcome mapping binds.
#[derive(Clone, Copy, Debug)]
pub struct ResultContext<'a> {
    /// Campaign identity.
    pub campaign_id: campaign_core::Id,
    /// Published selection the consumer executed.
    pub selection: &'a campaign_core::SelectionDecision,
    /// Child observations reserved by the suite for this selection; the result reports every one.
    pub children: &'a [campaign_core::ChildObservation],
    /// Current authoritative state.
    pub state: &'a campaign_core::FrontierState,
}

/// Campaign facts an unobserved closure binds.
#[derive(Clone, Copy, Debug)]
pub struct ClosureContext<'a> {
    /// Campaign identity.
    pub campaign_id: campaign_core::Id,
    /// Published selection the consumer could not observe.
    pub selection: &'a campaign_core::SelectionDecision,
}

/// Facts the consumer execution gate receives.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionGateRequest<'a> {
    /// Selection the consumer wants to execute.
    pub selection: &'a campaign_core::SelectionDecision,
    /// Planned publication of the selection event, if it was ever planned.
    pub publication: Option<&'a campaign_choregraph::ControlPublication>,
    /// Durability facts the host observed for that publication, if any.
    pub evidence: Option<&'a campaign_choregraph::DurablePublicationEvidence>,
}

/// Decision of the consumer execution gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionAuthority {
    /// The consumer may execute the selected operation.
    Granted,
    /// The consumer must not execute the selected operation.
    Refused,
}

/// Consumer-side facts that must never become promotion, deletion, or acceptance authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CONFORMANCE owner: each flag is an independent consumer signal the authority fixture presents"
)]
pub struct AuthoritySignals {
    /// The candidate held the highest rank when selected.
    pub is_top_ranked: bool,
    /// The candidate was selected and published.
    pub is_selected: bool,
    /// The consumer execution reported a zero exit status.
    pub has_zero_exit_status: bool,
    /// The consumer holds an external check reference for the result.
    pub has_check_reference: bool,
}
