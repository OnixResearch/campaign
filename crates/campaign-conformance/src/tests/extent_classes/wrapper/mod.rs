//! A wrapper around the healthy Animus-shaped adapter that misbehaves only on one list length,
//! fork depth, or settlement order the suite once never presented (sixth closeout audit).

mod shell;

/// One extent-class loophole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Hole {
    /// Loads and validates caches through the held projection only while the presented lineage
    /// is less than `gap` events ahead of it; a host that has fallen further behind reloads cold
    /// ("resync on gap").
    GapResync { gap: usize },
    /// Keeps only the first two levels of every abstract score (a consumer that stores its
    /// score as a primary and a secondary key).
    TwoLevelScores,
    /// Reports at most `CAPPED_CHILDREN` of the reserved children of a result (a consumer that
    /// records a fixed number of discovered moments per execution).
    CappedChildren,
    /// Reports at most `CAPPED_FINDINGS` findings of a result (a consumer that keeps the top
    /// findings of an execution).
    CappedFindings,
    /// Files each outcome under the oldest in-flight selection of the executed candidate (the
    /// mirror of the re-audit `ResultKeyedByCandidate` control, which uses the newest one).
    OldestInFlight,
}

/// Children a capped-children result mapping reports: the most any result fixture once presented.
const CAPPED_CHILDREN: usize = 2;

/// Findings a capped-findings result mapping reports: the most any result fixture once presented.
const CAPPED_FINDINGS: u32 = 3;

/// The healthy Animus-shaped adapter with the chosen loopholes open.
pub(super) struct UnsampledLengthAdapter {
    inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter,
    holes: alloc::vec::Vec<Hole>,
}

impl UnsampledLengthAdapter {
    pub(super) fn new(holes: &[Hole]) -> Self {
        Self {
            inner: crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            holes: holes.to_vec(),
        }
    }

    fn opens(&self, hole: Hole) -> bool {
        self.holes.contains(&hole)
    }

    /// Whether the gap-resync shell reloads this presented lineage cold.
    fn resyncs(
        &self,
        payloads: usize,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> bool {
        self.holes.iter().any(|hole| match hole {
            Hole::GapResync { gap } => previous
                .is_some_and(|held| payloads >= held.control_events().len().saturating_add(*gap)),
            Hole::TwoLevelScores
            | Hole::CappedChildren
            | Hole::CappedFindings
            | Hole::OldestInFlight => false,
        })
    }

    /// The preference the two-level-score mapping keeps.
    fn truncated(
        &self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::vocabulary::ScenarioPreference {
        let mut kept = preference.clone();
        if self.opens(Hole::TwoLevelScores) {
            kept.score.levels.truncate(2);
        }
        kept
    }
}

impl crate::port::MomentMapping for UnsampledLengthAdapter {
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

impl crate::port::RecordMapping for UnsampledLengthAdapter {
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
        let kept = self.truncated(preference);
        self.inner.guidance(&kept, context)
    }
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let kept = self.truncated(preference);
        self.inner.revision(&kept, context)
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
        let mut kept = outcome.clone();
        if self.opens(Hole::CappedFindings) {
            kept.findings = kept.findings.min(CAPPED_FINDINGS);
        }
        let children = if self.opens(Hole::CappedChildren) {
            context
                .children
                .get(..CAPPED_CHILDREN.min(context.children.len()))
                .unwrap_or_default()
        } else {
            context.children
        };
        let capped = crate::port::ResultContext {
            children,
            ..*context
        };
        let mut observation = self.inner.result(&kept, &capped)?;
        if self.opens(Hole::OldestInFlight)
            && let Some(oldest) = context
                .state
                .outstanding
                .iter()
                .filter(|outstanding| outstanding.candidate_id == context.selection.candidate_id)
                .min_by_key(|outstanding| outstanding.ordinal)
        {
            observation.selection_id = oldest.selection_id;
            observation.parent_moment_id = oldest.parent_moment_id;
            observation.operation_id = oldest.operation_id;
            let mut children = alloc::vec::Vec::with_capacity(observation.children.len());
            for child in observation.children {
                children.push(child.derive(context.campaign_id, oldest.selection_id)?);
            }
            observation.children = children;
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
