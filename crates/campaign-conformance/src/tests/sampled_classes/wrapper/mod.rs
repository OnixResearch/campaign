//! A wrapper around the healthy Animus-shaped adapter that misbehaves only on one input class the
//! suite once never presented (fifth closeout audit), or that relabels its candidate identities.

/// Lineage gap (in control events) from which the gap-resync shell reloads cold.
pub(super) const RESYNC_GAP: usize = 2;

/// Significant bits a JSON-number cost catalog keeps (the `f64` mantissa).
const MANTISSA_BITS: u32 = 53;

mod shell;

/// Frame domain of relabelled candidate generation profiles.
const RELABEL_DOMAIN: &[u8] = b"campaign-conformance-relabelled-candidates";

/// One sampled-class loophole, or a correct variation the generators must accept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Own gate that compares the presented decision with the published one only when the
    /// publication publishes a selection; for any other publication it trusts durable evidence
    /// that matches that publication's own planned transition.
    NonSelectionPublicationGate,
    /// Loads and validates caches through the held projection only while the presented lineage
    /// is less than `RESYNC_GAP` events ahead of it; a host that has fallen further behind
    /// reloads cold ("resync on gap").
    GapResyncHistory,
    /// Maps abstract cost units through an `f64` mantissa (keeps 53 significant bits).
    MantissaCostUnits,
    /// Carries both scenario draws through `u32`.
    U32Draws,
    /// Serves the held projection when a load is refused because the lineage exceeds the
    /// projection bound ("keep the last good projection when history grows too long").
    HeldOnProjectionLimit,
    /// Serves the held projection when a load is refused because the presented envelope
    /// identities cross the campaign's admitted ones (`Campaign(CrossedIdentity)`).
    HeldOnCrossedIdentity,
    /// Ignores the scenario's explore draw and always writes 0 (every exploratory request takes
    /// the explore branch).
    AlwaysExplore,
    /// Not a hole: declares a second resource dimension and maps every cost exactly into its
    /// declared dimension.
    CorrectSecondDimension,
    /// Not a hole: candidate identities relabelled by a salt in the candidate generation profile,
    /// so they order differently from the healthy adapter's.
    Relabelled(u64),
}

/// Name of the second resource dimension a correct multi-dimension adapter declares.
const SECOND_DIMENSION: &str = "sampled-correct-second-dimension";

pub(super) struct UnpresentedClassAdapter {
    pub(super) inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
}

impl UnpresentedClassAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
        }
    }

    pub(super) fn opens(&self, hole: Hole) -> bool {
        self.holes.contains(&hole)
    }

    /// Salt of the relabelled candidate identities, if any.
    fn relabel_salt(&self) -> Option<u64> {
        self.holes.iter().find_map(|hole| match hole {
            Hole::Relabelled(salt) => Some(*salt),
            _ => None,
        })
    }

    /// Whether the gap-resync shell reloads this presented lineage cold.
    pub(super) fn resyncs(
        &self,
        payloads: usize,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> bool {
        self.opens(Hole::GapResyncHistory)
            && previous.is_some_and(|held| {
                payloads >= held.control_events().len().saturating_add(RESYNC_GAP)
            })
    }

    /// The candidate of a correct two-dimension consumer: every cost in its own dimension.
    fn two_dimension_candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        let dimensions = crate::port::MomentMapping::identities(self)?.resource_dimension_ids;
        let mut first_only = operation.clone();
        for cost in &mut first_only.costs {
            cost.dimension = 0;
        }
        let mut record =
            crate::port::RecordMapping::candidate(&mut self.inner, &first_only, context)?;
        let mut declared = alloc::vec::Vec::with_capacity(operation.costs.len());
        for cost in &operation.costs {
            let index = usize::try_from(cost.dimension)
                .map_err(|_| crate::verdicts::ConformanceError::InvalidIdentities)?;
            declared.push(campaign_core::ResourceAmount {
                dimension_id: dimensions
                    .get(index)
                    .copied()
                    .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?,
                amount: cost.units,
            });
        }
        record.declared_costs = declared;
        Ok(record)
    }
}

/// `units` with only its 53 most significant bits kept, as a cost catalog of `f64` numbers.
fn through_mantissa(units: u64) -> u64 {
    let width = u64::BITS.saturating_sub(units.leading_zeros());
    let dropped = width.saturating_sub(MANTISSA_BITS);
    units
        .checked_shr(dropped)
        .and_then(|kept| kept.checked_shl(dropped))
        .unwrap_or(units)
}

/// The low 32-bit word of a draw.
fn through_u32(draw: u64) -> u64 {
    draw & u64::from(u32::MAX)
}

impl crate::port::MomentMapping for UnpresentedClassAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        let mut identities = self.inner.identities()?;
        if self.opens(Hole::CorrectSecondDimension) {
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

impl crate::port::RecordMapping for UnpresentedClassAdapter {
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        if self.opens(Hole::CorrectSecondDimension) {
            return self.two_dimension_candidate(operation, context);
        }
        if let Some(salt) = self.relabel_salt() {
            let mut record = self.inner.candidate(operation, context)?;
            record.operation_generation_profile_id = campaign_core::identity::derive_id_with(
                campaign_core::IdKind::Policy,
                RELABEL_DOMAIN,
                |writer| Ok(writer.u64(salt)),
            )?;
            return Ok(record);
        }
        let mut mapped = operation.clone();
        if self.opens(Hole::MantissaCostUnits) {
            for cost in &mut mapped.costs {
                cost.units = through_mantissa(cost.units);
            }
        }
        self.inner.candidate(&mapped, context)
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
        let mut issued = *draw;
        if self.opens(Hole::AlwaysExplore) {
            issued.explore_draw = 0;
        }
        if self.opens(Hole::U32Draws) {
            issued.explore_draw = through_u32(draw.explore_draw);
            issued.index_draw = through_u32(draw.index_draw);
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
