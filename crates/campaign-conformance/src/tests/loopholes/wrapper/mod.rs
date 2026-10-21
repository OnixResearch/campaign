//! A wrapper around the healthy Animus-shaped adapter that opens closeout-audit loopholes.

/// One loophole the closeout audit found an adapter could open while passing every fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Loophole {
    /// Own execution gate: never reads the selection and never consults Campaign; grants any
    /// selection whenever a durable publication and matching evidence are presented.
    OwnGate,
    /// Claims exhaustive search and authorization on every receipt that is not a selection
    /// receipt.
    NonSelectionOverclaim,
    /// Claims exhaustive search on every receipt of one kind only.
    OverclaimOn(ReceiptKind),
    /// Reports one stored record (the candidate) under three fabricated, distinct role identities.
    FabricatedRoles,
    /// Reports only the first reserved child of every result.
    DroppedChildren,
    /// Validates a cache by its state identity only, never by its history binding.
    StateOnlyCache,
    /// Validates the presented history, then returns the projection of a graph with one more
    /// event: the same state under a binding that was never presented.
    ForgedBinding,
}

/// Receipt kinds other than a selection receipt, told apart by their decision identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReceiptKind {
    /// Receipt of an accepted control event.
    Event,
    /// Receipt of a reject-rule admission refusal.
    Refusal,
    /// Receipt of a planned wait.
    Wait,
    /// Receipt of a planned stop.
    Stop,
}

impl ReceiptKind {
    fn of(receipt: &campaign_core::CampaignReceipt) -> Option<Self> {
        if receipt.admission_refusal.is_some() {
            return Some(Self::Refusal);
        }
        [
            (campaign_core::IdKind::Event, Self::Event),
            (campaign_core::IdKind::Stop, Self::Stop),
            (campaign_core::IdKind::DerivedRecord, Self::Wait),
        ]
        .into_iter()
        .find(|(decision_kind, _)| *decision_kind == receipt.decision_id.kind)
        .map(|(_, kind)| kind)
    }
}

/// The authority loopholes of the first audit reproducer (`LoopholeAdapter`).
pub(super) const AUTHORITY: &[Loophole] = &[
    Loophole::OwnGate,
    Loophole::NonSelectionOverclaim,
    Loophole::FabricatedRoles,
];

/// The history loopholes of the second audit reproducer (`ShallowAdapter`).
pub(super) const SHALLOW: &[Loophole] = &[
    Loophole::DroppedChildren,
    Loophole::StateOnlyCache,
    Loophole::ForgedBinding,
];

/// Frame domain of the forged graph event.
const FORGED_DOMAIN: &[u8] = b"campaign-conformance-loophole";

/// Healthy adapter wrapped with a set of loopholes.
pub(super) struct LoopholeAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    loopholes: alloc::vec::Vec<Loophole>,
}

impl LoopholeAdapter {
    pub(super) fn new(loopholes: &[Loophole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            loopholes: loopholes.to_vec(),
        }
    }

    fn opens(&self, loophole: Loophole) -> bool {
        self.loopholes.contains(&loophole)
    }
}

impl crate::port::MomentMapping for LoopholeAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        self.inner.identities()
    }

    fn exploration_draft(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        parent_event: Option<campaign_choregraph::EventId>,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::EventDraft> {
        self.inner.exploration_draft(moment, parent_event)
    }

    fn moment_identity(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        event: campaign_choregraph::EventId,
    ) -> crate::verdicts::ConformanceResult<crate::port::MomentIdentity> {
        self.inner.moment_identity(moment, event)
    }

    fn reason(
        &mut self,
        reason: crate::vocabulary::ScenarioReason,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        self.inner.reason(reason)
    }
}

impl crate::port::RecordMapping for LoopholeAdapter {
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        self.inner.candidate(operation, context)
    }

    fn guidance(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::GuidanceContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
        self.inner.guidance(preference, context)
    }

    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        self.inner.revision(preference, context)
    }

    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        self.inner.entropy_ticket(draw, context)
    }

    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &crate::port::ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        let mut observation = self.inner.result(outcome, context)?;
        if self.opens(Loophole::DroppedChildren) {
            observation.children.truncate(1);
        }
        Ok(observation)
    }

    fn closure(
        &mut self,
        context: &crate::port::ClosureContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
        self.inner.closure(context)
    }
}

impl crate::port::ConsumerShell for LoopholeAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        let loaded = self.inner.load_history(request, previous)?;
        if !self.opens(Loophole::ForgedBinding) {
            return Ok(loaded);
        }
        let forged = request
            .graph
            .admit(campaign_choregraph::EventDraft::v1(
                campaign_choregraph::EventClass::Application,
                campaign_choregraph::Digest::derive(FORGED_DOMAIN, b"forged-graph"),
                alloc::vec::Vec::new(),
                campaign_choregraph::Digest::derive(FORGED_DOMAIN, b"schema"),
            ))?
            .graph;
        let binding = campaign_choregraph::derive_projection_binding(
            &forged,
            request.branch,
            request.expected_binding.projection_schema_id,
            request.expected_binding.adapter_id,
            request.expected_binding.control_domain_id,
        )?;
        self.inner.load_history(
            &campaign_choregraph::ControlProjectionRequest {
                graph: &forged,
                branch: request.branch,
                parent_choices: request.parent_choices,
                maximum_events: request.maximum_events,
                expected_binding: binding,
                payloads: request.payloads,
            },
            previous,
        )
    }

    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if !self.opens(Loophole::StateOnlyCache) {
            return self.inner.validate_cache(request, cache);
        }
        let events: alloc::vec::Vec<campaign_core::CampaignControlEvent> = request
            .payloads
            .iter()
            .map(|payload| payload.control_event.clone())
            .collect();
        let state = campaign_core::reconstruct(&events)?;
        if state.state_id != cache.state_id {
            return Err(campaign_core::CampaignError::InvalidCache.into());
        }
        Ok(state)
    }

    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        if !self.opens(Loophole::OwnGate) {
            return self.inner.execution_gate(request);
        }
        let (Some(publication), Some(evidence)) = (request.publication, request.evidence) else {
            return crate::port::ExecutionAuthority::Refused;
        };
        let is_granted = evidence.durable_event
            && evidence.durable_branch_update
            && evidence.history_event_id == publication.history_event_id()
            && evidence.branch_before == publication.branch_plan().before
            && evidence.branch_after == publication.branch_plan().after;
        if is_granted {
            crate::port::ExecutionAuthority::Granted
        } else {
            crate::port::ExecutionAuthority::Refused
        }
    }

    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        let mut claims = self.inner.promotion_claims(receipt, signals);
        if self.opens(Loophole::NonSelectionOverclaim)
            && receipt.decision_id.kind != campaign_core::IdKind::Selection
        {
            claims.exhaustive_search = true;
            claims.authorization = signals.has_check_reference;
        }
        if let Some(kind) = ReceiptKind::of(receipt)
            && self.opens(Loophole::OverclaimOn(kind))
        {
            claims.exhaustive_search = true;
        }
        claims
    }

    fn role_identities(
        &mut self,
        candidate: &campaign_core::CandidateRecord,
        work_item: &campaign_core::SelectionDecision,
        result: &campaign_core::ResultObservation,
    ) -> campaign_core::RoleCollapseFixture {
        if !self.opens(Loophole::FabricatedRoles) {
            return self.inner.role_identities(candidate, work_item, result);
        }
        campaign_core::RoleCollapseFixture {
            candidate_record_id: candidate.candidate_id,
            queued_work_id: campaign_core::derive_id(
                campaign_core::IdKind::Selection,
                b"one-record",
                &[candidate.candidate_id],
            ),
            computed_result_id: campaign_core::derive_id(
                campaign_core::IdKind::Result,
                b"one-record",
                &[candidate.candidate_id],
            ),
        }
    }
}
