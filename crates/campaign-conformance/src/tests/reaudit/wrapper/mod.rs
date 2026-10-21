//! A wrapper around the healthy Animus-shaped adapter that opens the closeout re-audit loopholes:
//! each misbehaves only on an input class the suite once never presented.

mod shell;

/// One re-audit loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Own gate that never consults Campaign and compares only the selection identity (plus
    /// evidence consistency and durability) with the published event.
    IdOnlyGate,
    /// Grants a publication presented without evidence when its event names the selection.
    EvidenceOptionalGate,
    /// Grants every exploratory selection without a publication.
    ExploratoryGateBypass,
    /// Claims exhaustive search and safe deletion on non-selection receipts when nothing is
    /// selected.
    QuietStopClaim,
    /// Claims exhaustive search on every receipt that binds an entropy ticket.
    ExploratoryReceiptOverclaim,
    /// Claims operation success on a zero exit status without a check reference.
    MixedSignalOverclaim,
    /// Claims operation success on the receipts of result events it loaded.
    ResultEventOverclaim,
    /// Reuses the candidate identity for the work item and result of exploratory executions.
    ExploratoryRoleCollapse,
    /// Drops children that already have a stable discovery.
    DedupRediscoveredChildren,
    /// Swaps the payloads of the children of a forked result.
    CrossWiredForkPayloads,
    /// Drops every finding.
    DroppedFindings,
    /// Adds one fabricated finding to every result.
    FabricatedFindings,
    /// Attributes every result to the newest outstanding selection of its candidate.
    ResultKeyedByCandidate,
    /// Validates a cache by its fresh binding and reconstructed state identity only, never the
    /// payload lineage.
    BindingStateCache,
    /// Validates a cache by its projection identity and the presented lineage only, never the
    /// cached state.
    ProjectionIdOnlyCache,
    /// Serves the last loaded projection when a history lags, rewinds, forks, or renames.
    JournalOnUnpresentedDenials,
    /// Returns another control domain on every identities call.
    UnstableIdentities,
    /// Saturates every level at one byte before mapping it to a rank.
    SaturatingRanks,
    /// Encodes both ticket draws big-endian.
    BigEndianTicket,
}

/// Frame domain of wrapper-owned identities.
const REAUDIT_DOMAIN: &[u8] = b"campaign-conformance-reaudit";

/// Healthy adapter wrapped with a set of re-audit loopholes.
pub(super) struct HoledAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
    journal: Option<campaign_choregraph::ValidatedControlProjection>,
    result_events: alloc::vec::Vec<campaign_core::Id>,
    identity_calls: core::cell::Cell<u32>,
}

impl HoledAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
            journal: None,
            result_events: alloc::vec::Vec::new(),
            identity_calls: core::cell::Cell::new(0),
        }
    }

    fn opens(&self, hole: Hole) -> bool {
        self.holes.contains(&hole)
    }
}

fn saturated(
    mut record: campaign_core::GuidanceRecord,
    preference: &crate::vocabulary::ScenarioPreference,
) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
    record.ranks = preference
        .score
        .levels
        .iter()
        .map(|level| i64::from(u8::try_from(*level).unwrap_or(u8::MAX)))
        .collect();
    record.guidance_id = campaign_core::Id::absent();
    Ok(record.derive()?)
}

fn fabricated_finding(
    selection_id: campaign_core::Id,
) -> crate::verdicts::ConformanceResult<campaign_core::FindingRef> {
    Ok(campaign_core::FindingRef {
        finding_id: campaign_core::identity::derive_id_with(
            campaign_core::IdKind::Finding,
            REAUDIT_DOMAIN,
            |writer| Ok(writer.bytes(b"fabricated").id(selection_id)),
        )?,
        priority_facts_id: campaign_core::identity::derive_id_with(
            campaign_core::IdKind::SourceFacts,
            REAUDIT_DOMAIN,
            |writer| Ok(writer.bytes(b"fabricated").id(selection_id)),
        )?,
    })
}

impl crate::port::MomentMapping for HoledAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        let mut identities = self.inner.identities()?;
        if self.opens(Hole::UnstableIdentities) {
            let call = self.identity_calls.get();
            self.identity_calls.set(call.saturating_add(1));
            identities.control_domain_id = campaign_core::identity::derive_id_with(
                campaign_core::IdKind::ControlDomain,
                REAUDIT_DOMAIN,
                |writer| Ok(writer.u32(call)),
            )?;
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

impl crate::port::RecordMapping for HoledAdapter {
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
        if self.opens(Hole::SaturatingRanks) {
            return saturated(record, preference);
        }
        Ok(record)
    }

    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let mut request = self.inner.revision(preference, context)?;
        if self.opens(Hole::SaturatingRanks) {
            let successor = request.successor.clone();
            request.successor = saturated(successor, preference)?;
        }
        Ok(request)
    }

    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        let mut ticket = self.inner.entropy_ticket(draw, context)?;
        if !self.opens(Hole::BigEndianTicket) {
            return Ok(ticket);
        }
        let (explore, rest) = ticket
            .bytes
            .split_at_mut(campaign_core::ENTROPY_PREFIX_BYTES);
        explore.copy_from_slice(&draw.explore_draw.to_be_bytes());
        rest.get_mut(..campaign_core::ENTROPY_PREFIX_BYTES)
            .ok_or(campaign_core::CampaignError::InvalidEntropy)?
            .copy_from_slice(&draw.index_draw.to_be_bytes());
        ticket.entropy_id = campaign_core::Id::absent();
        Ok(ticket.derive()?)
    }

    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &crate::port::ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        let mut observation = self.inner.result(outcome, context)?;
        if self.opens(Hole::DedupRediscoveredChildren) {
            observation
                .children
                .retain(|child| context.state.discovery(child.child_moment_id).is_none());
        }
        if self.opens(Hole::CrossWiredForkPayloads) && observation.children.len() > 1 {
            let payloads: alloc::vec::Vec<campaign_core::Id> = observation
                .children
                .iter()
                .rev()
                .map(|child| child.event_payload_id)
                .collect();
            let mut rewired = alloc::vec::Vec::with_capacity(payloads.len());
            for (child, event_payload_id) in observation.children.iter().zip(payloads) {
                rewired.push(
                    campaign_core::ChildObservation {
                        event_payload_id,
                        ..child.clone()
                    }
                    .derive(context.campaign_id, context.selection.selection_id)?,
                );
            }
            observation.children = rewired;
        }
        if self.opens(Hole::DroppedFindings) {
            observation.findings.clear();
        }
        if self.opens(Hole::FabricatedFindings) {
            observation
                .findings
                .push(fabricated_finding(context.selection.selection_id)?);
        }
        if self.opens(Hole::ResultKeyedByCandidate)
            && let Some(newest) = context
                .state
                .outstanding
                .iter()
                .filter(|outstanding| outstanding.candidate_id == context.selection.candidate_id)
                .max_by_key(|outstanding| outstanding.ordinal)
        {
            observation.selection_id = newest.selection_id;
            observation.parent_moment_id = newest.parent_moment_id;
            observation.operation_id = newest.operation_id;
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
