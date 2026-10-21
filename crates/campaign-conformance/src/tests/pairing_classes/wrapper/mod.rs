//! A wrapper around the healthy Animus-shaped adapter with eleventh-audit pairing loopholes open.
//! Every hole keys only on facts Campaign shows the consumer (the requests, contexts, loaded
//! projections, and results it is handed), never on fixture names or suite identities.

/// One eleventh-audit pairing loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Own gate over durability, the exact published decision and event, the planned head
    /// before, the event's head after, one branch name before and after, and one generation step:
    /// names and generations are compared with each other only, never with the publication's
    /// plan.
    RelativeCasGate,
    /// The same gate comparing branch names with the publication's plan, and generations with
    /// each other only (one step).
    RelativeGenerationGate,
    /// Grants a selection presented without a publication or evidence when the last loaded
    /// projection already carries its publication (a recovering consumer).
    LoadedRecoveryGate,
    /// Reports the candidate identity as the computed result of a result that reported findings
    /// or child moments (a consumer that files such a result under its candidate).
    ReportedResultCollapse,
    /// Builds revisions naming the last guidance record the adapter itself mapped for the
    /// candidate instead of the context's invalidated record.
    RevisionOwnMemory,
    /// Issues entropy tickets with an adapter-owned sequence number as the counter.
    TicketOwnCounter,
}

/// The healthy Animus-shaped adapter with the chosen loopholes open, and what it remembers.
pub(super) struct EleventhAuditAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    /// The projection the shell last loaded.
    last_view: Option<campaign_choregraph::ValidatedControlProjection>,
    /// Last guidance record the adapter mapped, per candidate.
    mapped: alloc::vec::Vec<(campaign_core::Id, campaign_core::Id)>,
    /// Adapter-owned ticket sequence.
    issued: u64,
}

impl EleventhAuditAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            last_view: None,
            mapped: alloc::vec::Vec::new(),
            issued: 0,
        }
    }

    fn remember_mapped(&mut self, candidate_id: campaign_core::Id, guidance_id: campaign_core::Id) {
        if let Some(entry) = self
            .mapped
            .iter_mut()
            .find(|(candidate, _)| *candidate == candidate_id)
        {
            entry.1 = guidance_id;
        } else {
            self.mapped.push((candidate_id, guidance_id));
        }
    }

    /// Whether the last loaded projection publishes exactly `selection`.
    fn has_loaded(&self, selection: &campaign_core::SelectionDecision) -> bool {
        self.last_view.as_ref().is_some_and(|view| {
            view.control_events().iter().any(|event| {
                matches!(
                    event,
                    campaign_core::CampaignControlEvent::SelectionPublished(published)
                        if published.decision == *selection
                )
            })
        })
    }
}

impl crate::port::MomentMapping for EleventhAuditAdapter {
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

impl crate::port::RecordMapping for EleventhAuditAdapter {
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
        let record = self.inner.guidance(preference, context)?;
        self.remember_mapped(context.candidate_id, record.guidance_id);
        Ok(record)
    }
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let mut request = self.inner.revision(preference, context)?;
        if self.holes.contains(&Hole::RevisionOwnMemory)
            && let Some((_, last)) = self
                .mapped
                .iter()
                .find(|(candidate, _)| *candidate == context.candidate_id)
        {
            request.invalidated_guidance_id = *last;
        }
        self.remember_mapped(context.candidate_id, request.successor.guidance_id);
        Ok(request)
    }
    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        if self.holes.contains(&Hole::TicketOwnCounter) {
            self.issued = self.issued.saturating_add(1);
            let owned = crate::port::EntropyContext {
                counter: self.issued,
                ..*context
            };
            return self.inner.entropy_ticket(draw, &owned);
        }
        self.inner.entropy_ticket(draw, context)
    }
    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &crate::port::ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        self.inner.result(outcome, context)
    }
    fn closure(
        &mut self,
        context: &crate::port::ClosureContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
        self.inner.closure(context)
    }
}

impl crate::port::ConsumerShell for EleventhAuditAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        let loaded = self.inner.load_history(request, previous);
        if let Ok(view) = &loaded {
            self.last_view = Some(view.clone());
        }
        loaded
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        self.inner.validate_cache(request, cache)
    }
    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        let is_missing = request.publication.is_none() || request.evidence.is_none();
        if self.holes.contains(&Hole::LoadedRecoveryGate)
            && is_missing
            && self.has_loaded(request.selection)
        {
            return crate::port::ExecutionAuthority::Granted;
        }
        if self.holes.contains(&Hole::RelativeCasGate) {
            return relative_cas_gate(request, false);
        }
        if self.holes.contains(&Hole::RelativeGenerationGate) {
            return relative_cas_gate(request, true);
        }
        self.inner.execution_gate(request)
    }
    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        self.inner.promotion_claims(receipt, signals)
    }
    fn role_identities(
        &mut self,
        candidate: &campaign_core::CandidateRecord,
        work_item: &campaign_core::SelectionDecision,
        result: &campaign_core::ResultObservation,
    ) -> campaign_core::RoleCollapseFixture {
        let mut roles = self.inner.role_identities(candidate, work_item, result);
        let is_reported = !result.findings.is_empty() || !result.children.is_empty();
        if self.holes.contains(&Hole::ReportedResultCollapse) && is_reported {
            roles.computed_result_id = candidate.candidate_id;
        }
        roles
    }
}

/// Own gate over durability, the exact published decision and event, the planned head before,
/// the event's head after, one branch name before and after (the plan's names when
/// `planned_names`), and one generation step; generations, and unless `planned_names` names, are
/// compared with each other only, never with the publication's plan.
fn relative_cas_gate(
    request: &crate::port::ExecutionGateRequest<'_>,
    planned_names: bool,
) -> crate::port::ExecutionAuthority {
    let (Some(publication), Some(evidence)) = (request.publication, request.evidence) else {
        return crate::port::ExecutionAuthority::Refused;
    };
    let campaign_core::CampaignControlEvent::SelectionPublished(published) =
        publication.control_event()
    else {
        return crate::port::ExecutionAuthority::Refused;
    };
    let plan = publication.branch_plan();
    let (before, after) = (&evidence.branch_before, &evidence.branch_after);
    let is_granted = evidence.durable_event
        && evidence.durable_branch_update
        && published.decision == *request.selection
        && evidence.history_event_id == publication.history_event_id()
        && before.head == plan.before.head
        && after.head == publication.history_event_id()
        && before.name == after.name
        && (!planned_names || before.name == plan.before.name)
        && before.generation.checked_add(1) == Some(after.generation);
    debug_assert_eq!(plan.after.head, publication.history_event_id());
    debug_assert!(!is_granted || (evidence.durable_event && evidence.durable_branch_update));
    if is_granted {
        crate::port::ExecutionAuthority::Granted
    } else {
        crate::port::ExecutionAuthority::Refused
    }
}
