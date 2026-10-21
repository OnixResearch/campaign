//! A wrapper around the healthy Animus-shaped adapter with eighth-audit loopholes open.

/// One eighth-audit loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Loads and validates caches cold whenever the held projection records an accepted stop (a
    /// host that drops a finished campaign's projection and reloads it on demand).
    ColdWhenStopped,
    /// Loads and validates caches cold whenever the held projection has settled a selection (a
    /// host that drops its held projection once a result is recorded).
    ColdWhenSettled,
    /// Loads and validates caches cold whenever the held projection is longer than `events`
    /// control events (a host that checkpoints long lineages and reloads them cold).
    ColdWhenHeldLongerThan { events: usize },
    /// Loads and validates caches cold whenever the presented lineage is at least `depth` events
    /// shorter than the held one (a host that treats a deep rewind as a reset).
    RewindResync { depth: usize },
    /// Loads and validates caches cold whenever the presented lineage is at least `gap` events
    /// longer than the held one (the "resync on gap" shell).
    GapResync { gap: usize },
    /// Loads and validates caches cold whenever the presented lineage is not shorter than the
    /// held one and its head is not a stop.
    ColdUnlessStopHead,
    /// Claims exhaustive search on a wait or stop receipt planned with an entropy ticket.
    TicketedWaitOrStopClaim,
    /// Claims exhaustive search on a refusal receipt of a duplicate within one batch.
    SameBatchRefusalClaim,
    /// Zeroes every level from the third to the second-to-last of scores of four or more levels.
    ZeroInteriorLevels,
    /// Carries the explore draw through a binary64 float (rounded to nearest) into the ticket.
    Binary64ExploreDraw,
    /// Keys every finding by its position in the result, so the findings of two results of one
    /// campaign share identities.
    FindingsKeyedByPosition,
}

/// Frame domain of the position-keyed finding identities.
const POSITION_FINDINGS: &[u8] = b"held-classes-position-finding";

/// The healthy Animus-shaped adapter with the chosen loopholes open.
pub(super) struct EighthAuditAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
}

impl EighthAuditAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
        }
    }

    fn resyncs(
        &self,
        payloads: &[campaign_choregraph::ControlPayloadEnvelope],
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> bool {
        let Some(held) = previous else {
            return false;
        };
        let held_events = held.control_events().len();
        let is_stop_head = payloads.last().is_some_and(|payload| {
            matches!(
                payload.control_event,
                campaign_core::CampaignControlEvent::StopAccepted(_)
            )
        });
        self.holes.iter().any(|hole| match hole {
            Hole::ColdWhenStopped => held.state().stopped.is_some(),
            Hole::ColdWhenSettled => !held.state().settled.is_empty(),
            Hole::ColdWhenHeldLongerThan { events } => held_events > *events,
            Hole::RewindResync { depth } => payloads.len().saturating_add(*depth) <= held_events,
            Hole::GapResync { gap } => payloads.len() >= held_events.saturating_add(*gap),
            Hole::ColdUnlessStopHead => payloads.len() >= held_events && !is_stop_head,
            Hole::TicketedWaitOrStopClaim
            | Hole::SameBatchRefusalClaim
            | Hole::ZeroInteriorLevels
            | Hole::Binary64ExploreDraw
            | Hole::FindingsKeyedByPosition => false,
        })
    }

    fn kept(
        &self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::vocabulary::ScenarioPreference {
        let mut kept = preference.clone();
        if self.holes.contains(&Hole::ZeroInteriorLevels) {
            let last = kept.score.levels.len().saturating_sub(1);
            for (index, level) in kept.score.levels.iter_mut().enumerate() {
                if index >= 2 && index < last {
                    *level = 0;
                }
            }
        }
        kept
    }

    fn overclaims(&self, receipt: &campaign_core::CampaignReceipt) -> bool {
        self.holes.iter().any(|hole| match hole {
            Hole::TicketedWaitOrStopClaim => {
                receipt.entropy_ticket_id.is_some()
                    && receipt.guidance_record_id.is_none()
                    && receipt.admission_outcomes_id.is_none()
            }
            Hole::SameBatchRefusalClaim => {
                receipt.admission_refusal.as_ref().is_some_and(|refusal| {
                    refusal.duplicate_of == campaign_core::DuplicateOrigin::SameBatch
                })
            }
            _ => false,
        })
    }
}

impl crate::port::MomentMapping for EighthAuditAdapter {
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

impl crate::port::RecordMapping for EighthAuditAdapter {
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
        let kept = self.kept(preference);
        self.inner.guidance(&kept, context)
    }
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let kept = self.kept(preference);
        self.inner.revision(&kept, context)
    }
    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        if self.holes.contains(&Hole::Binary64ExploreDraw) {
            let rounded = crate::vocabulary::ScenarioDraw {
                explore_draw: crate::catalog::ranking::tickets::draws::binary64(draw.explore_draw)
                    .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
                index_draw: draw.index_draw,
            };
            return self.inner.entropy_ticket(&rounded, context);
        }
        self.inner.entropy_ticket(draw, context)
    }
    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &crate::port::ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        let mut observation = self.inner.result(outcome, context)?;
        if self.holes.contains(&Hole::FindingsKeyedByPosition) {
            for (position, finding) in (0_u32..).zip(observation.findings.iter_mut()) {
                finding.finding_id = campaign_core::identity::derive_id_with(
                    campaign_core::IdKind::Finding,
                    POSITION_FINDINGS,
                    |writer| Ok(writer.u32(position)),
                )?;
            }
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

impl crate::port::ConsumerShell for EighthAuditAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        if self.resyncs(request.payloads, previous) {
            return campaign_choregraph::validate_control_projection(request);
        }
        self.inner.load_history(request, previous)
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.resyncs(request.payloads, request.previous) {
            let cold = campaign_choregraph::CacheValidationRequest {
                previous: None,
                ..*request
            };
            return self.inner.validate_cache(&cold, cache);
        }
        self.inner.validate_cache(request, cache)
    }
    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        self.inner.execution_gate(request)
    }
    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        let mut claims = self.inner.promotion_claims(receipt, signals);
        if self.overclaims(receipt) {
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
        self.inner.role_identities(candidate, work_item, result)
    }
}
