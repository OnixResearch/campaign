//! A wrapper around the healthy Animus-shaped adapter that misbehaves only on one input class the
//! suite once sampled by hand instead of generating (fourth closeout audit).

mod shell;

/// Index draw a constant-draw ticket writes regardless of the scenario.
const CONSTANT_INDEX_DRAW: u64 = 5;

/// Name of the second resource dimension a collapsing adapter declares.
const SECOND_DIMENSION: &str = "collapsed-second-dimension";

/// One generated-class loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Own gate: durable flags, exact published decision, event identity, and the before and
    /// after heads only; never compares branch names or generations of the evidence.
    HeadOnlyEvidenceGate,
    /// Delegates to Campaign's gate with the published decision whenever the presented decision
    /// has the published selection identity, candidate, operation, and ordinal.
    ExecutionFieldsGate,
    /// Own gate: durable flags, exact published decision, `branch_before` equal to the planned
    /// value, and self-consistent evidence (event is the after head, one generation later on the
    /// same branch); never compares the evidence's event with the publication's event.
    SelfConsistentEvidenceGate,
    /// Cache validation that falls back to core replay of the presented payloads on payload,
    /// metadata, parent, envelope, schema, projection-identity, history, and Campaign denials
    /// other than `InvalidCache`.
    CacheFallbackOnUnpresentedDenials,
    /// With a held projection, validates the cache cold and refuses every cache the cold check
    /// accepts.
    RefuseHeldCaches,
    /// Serves the held projection when a load is refused with a metadata, envelope-class, schema,
    /// or projection-identity denial, a Campaign denial other than `StaleState`, or a history
    /// denial other than an ambiguous lineage.
    JournalOnResidualLoadDenials,
    /// Declares a second resource dimension but maps every abstract cost onto the first one.
    CollapsedCostDimensions,
    /// Maps abstract cost units through a `u8` (saturating at 255), onto the first dimension.
    U8Units,
    /// Ignores the scenario's index draw and always writes the same index draw.
    ConstantIndexDraw,
    /// Maps guidance levels with the second level replaced by the first.
    FirstLevelOnly,
}

pub(super) struct GeneratedClassAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
}

impl GeneratedClassAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
        }
    }

    fn opens(&self, hole: Hole) -> bool {
        self.holes.contains(&hole)
    }

    /// The preference a first-level-only mapping converts: its second level replaced by the
    /// first.
    fn flattened(
        &self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::vocabulary::ScenarioPreference {
        let mut flattened = preference.clone();
        if self.opens(Hole::FirstLevelOnly)
            && let (Some(first), Some(second)) = (
                flattened.score.levels.first().copied(),
                flattened.score.levels.get_mut(1),
            )
        {
            *second = first;
        }
        flattened
    }
}

impl crate::port::MomentMapping for GeneratedClassAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        let mut identities = self.inner.identities()?;
        if self.opens(Hole::CollapsedCostDimensions) {
            identities
                .resource_dimension_ids
                .push(crate::harness::suite_id(
                    campaign_core::IdKind::ResourceDimension,
                    SECOND_DIMENSION,
                )?);
        }
        Ok(identities)
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

impl crate::port::RecordMapping for GeneratedClassAdapter {
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        let is_collapsed = self.opens(Hole::CollapsedCostDimensions) || self.opens(Hole::U8Units);
        if !is_collapsed {
            return self.inner.candidate(operation, context);
        }
        let mut collapsed = operation.clone();
        for cost in &mut collapsed.costs {
            cost.dimension = 0;
            if self.opens(Hole::U8Units) {
                cost.units = cost.units.min(u64::from(u8::MAX));
            }
        }
        self.inner.candidate(&collapsed, context)
    }
    fn guidance(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::GuidanceContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
        let flattened = self.flattened(preference);
        self.inner.guidance(&flattened, context)
    }
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let flattened = self.flattened(preference);
        self.inner.revision(&flattened, context)
    }
    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        let mut issued = *draw;
        if self.opens(Hole::ConstantIndexDraw) {
            issued.index_draw = CONSTANT_INDEX_DRAW;
        }
        self.inner.entropy_ticket(&issued, context)
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

impl crate::port::ConsumerShell for GeneratedClassAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        let loaded = self.inner.load_history(request, previous);
        if self.opens(Hole::JournalOnResidualLoadDenials)
            && let (Err(error), Some(held)) = (&loaded, previous)
            && shell::is_residual_load_denial(error)
        {
            return Ok(held.clone());
        }
        loaded
    }
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        if self.opens(Hole::RefuseHeldCaches) && request.previous.is_some() {
            let cold = campaign_choregraph::CacheValidationRequest {
                previous: None,
                ..*request
            };
            return self
                .inner
                .validate_cache(&cold, cache)
                .and(Err(campaign_choregraph::AdapterError::NonFastForwardBranch));
        }
        let validated = self.inner.validate_cache(request, cache);
        match validated {
            Err(error)
                if self.opens(Hole::CacheFallbackOnUnpresentedDenials)
                    && shell::is_unpresented_cache_denial(&error) =>
            {
                shell::replayed_cache(request, cache)
            }
            other => other,
        }
    }
    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        if self.opens(Hole::HeadOnlyEvidenceGate) {
            return shell::head_only_gate(request);
        }
        if self.opens(Hole::SelfConsistentEvidenceGate) {
            return shell::self_consistent_gate(request);
        }
        if self.opens(Hole::ExecutionFieldsGate)
            && let Some(published) = shell::published_decision(request)
            && shell::has_execution_fields(published, request.selection)
        {
            return self
                .inner
                .execution_gate(&crate::port::ExecutionGateRequest {
                    selection: published,
                    publication: request.publication,
                    evidence: request.evidence,
                });
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
        self.inner.role_identities(candidate, work_item, result)
    }
}
