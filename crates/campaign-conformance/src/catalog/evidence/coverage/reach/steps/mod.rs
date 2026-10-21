//! Every control event a consumer can publish on one state of the bounded search: candidate
//! batches (all admitted, with a coalesced duplicate, and of a duplicate alone), first guidance,
//! strict improvements, equal-rank updates, and revisions of every candidate, the ranked and the
//! exploratory plan's selection or stop, a plain, child, finding, and child-and-finding result and
//! a closure of every outstanding selection, a consumer and a policy retirement of every retained
//! candidate, the declared prune, and consumer stops under the adapter's reason and the
//! host-capacity reason and a policy stop. Records come from the Animus-shaped adapter; events
//! Campaign refuses are not publishable and are dropped.

/// Operations of the search instance, in suite operation order (at most three candidates).
pub(super) const OPERATIONS: [crate::vocabulary::OperationHandle; 3] = [
    crate::catalog::common::OP_A,
    crate::catalog::common::OP_B,
    crate::catalog::common::OP_C,
];

/// One search node: a state and the decisions of its outstanding selections.
#[derive(Clone)]
pub(super) struct Node {
    pub(super) state: campaign_core::FrontierState,
    pub(super) decisions: alloc::vec::Vec<campaign_core::SelectionDecision>,
}

/// What the search holds fixed for one admission configuration.
pub(super) struct Context<'r, 's, 'a> {
    pub(super) session: &'r mut crate::harness::Session<'s, 'a>,
    pub(super) profiles: [campaign_core::SelectionProfile; 2],
    pub(super) records: alloc::vec::Vec<(
        crate::vocabulary::OperationHandle,
        campaign_core::CandidateRecord,
    )>,
    pub(super) binding: campaign_core::ProjectionBinding,
}

/// A publishable event and, for a selection, its decision.
pub(super) type Published = (
    campaign_core::CampaignControlEvent,
    Option<campaign_core::SelectionDecision>,
);

impl Context<'_, '_, '_> {
    /// Every event publishable on `node`.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of a record mapping.
    pub(super) fn successors(
        &mut self,
        node: &Node,
    ) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Published>> {
        let state = &node.state;
        let mut published = alloc::vec::Vec::new();
        if state.stopped.is_none() {
            self.batches(state, &mut published)?;
            self.guidance(state, &mut published)?;
            self.selections(state, &mut published)?;
            self.removals(state, &mut published)?;
            self.stops(state, &mut published)?;
        }
        for decision in &node.decisions {
            self.settlements(state, decision, &mut published)?;
        }
        Ok(published)
    }

    fn batches(
        &self,
        state: &campaign_core::FrontierState,
        published: &mut alloc::vec::Vec<Published>,
    ) -> crate::verdicts::ConformanceResult<()> {
        let admitted: alloc::vec::Vec<&campaign_core::CandidateRecord> = self
            .records
            .iter()
            .map(|(_, record)| record)
            .filter(|record| state.candidate(record.candidate_id).is_ok())
            .collect();
        let fresh = self
            .records
            .iter()
            .map(|(_, record)| record)
            .find(|record| state.candidate(record.candidate_id).is_err());
        let mut batches = alloc::vec::Vec::new();
        if let Some(fresh) = fresh {
            batches.push(alloc::vec![fresh.clone()]);
        }
        if let Some(first) = admitted.first() {
            batches.push(alloc::vec![(*first).clone()]);
            if let Some(fresh) = fresh {
                batches.push(alloc::vec![fresh.clone(), (*first).clone()]);
            }
        }
        let rule = campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Coalesce,
        };
        let coalescing = crate::harness::records::coalescing_policy_id()?;
        for records in batches {
            keep(
                published,
                campaign_core::admit_candidates(state, records, rule, coalescing),
            );
        }
        Ok(())
    }

    fn guidance(
        &mut self,
        state: &campaign_core::FrontierState,
        published: &mut alloc::vec::Vec<Published>,
    ) -> crate::verdicts::ConformanceResult<()> {
        let campaign_id = state.admission.campaign_id;
        for (operation, record) in self.records.clone() {
            let Ok(candidate) = state.candidate(record.candidate_id) else {
                continue;
            };
            let context = crate::port::GuidanceContext {
                campaign_id,
                candidate_id: record.candidate_id,
            };
            let adapter = &mut *self.session.harness.adapter;
            let Some(latest) = candidate.latest_guidance.clone() else {
                let levels = [
                    crate::catalog::common::LEVELS_A,
                    crate::catalog::common::LEVELS_B,
                    crate::catalog::common::LEVELS_C,
                ];
                let position = OPERATIONS.iter().position(|known| *known == operation);
                let standard = position
                    .and_then(|at| levels.get(at))
                    .copied()
                    .unwrap_or_default();
                let first = preference(
                    operation,
                    crate::catalog::common::FIRST_GENERATION,
                    &standard,
                );
                let guidance = adapter.guidance(&first, &context)?;
                keep(published, campaign_core::admit_guidance(state, guidance));
                continue;
            };
            let generation = latest.generation.saturating_add(1);
            let levels: alloc::vec::Vec<u32> = latest
                .ranks
                .iter()
                .map(|rank| u32::try_from(*rank).unwrap_or(0))
                .collect();
            let raised: alloc::vec::Vec<u32> =
                levels.iter().map(|level| level.saturating_add(2)).collect();
            let lowered: alloc::vec::Vec<u32> =
                levels.iter().map(|level| level.saturating_sub(1)).collect();
            for levels in [raised, levels] {
                let record =
                    adapter.guidance(&preference(operation, generation, &levels), &context)?;
                keep(published, campaign_core::admit_guidance(state, record));
            }
            let revision = adapter.revision(
                &preference(operation, generation, &lowered),
                &crate::port::RevisionContext {
                    campaign_id,
                    candidate_id: record.candidate_id,
                    invalidated_guidance_id: latest.guidance_id,
                },
            )?;
            keep(published, campaign_core::revise_guidance(state, revision));
        }
        Ok(())
    }

    fn selections(
        &mut self,
        state: &campaign_core::FrontierState,
        published: &mut alloc::vec::Vec<Published>,
    ) -> crate::verdicts::ConformanceResult<()> {
        for profile in self.profiles.clone() {
            let entropy = if matches!(profile.mode, campaign_core::SelectionMode::Ranked) {
                None
            } else {
                Some(self.session.harness.adapter.entropy_ticket(
                    &crate::catalog::evidence::claims::EXPLORE,
                    &crate::port::EntropyContext {
                        campaign_id: state.admission.campaign_id,
                        state_id: state.state_id,
                        selection_profile_id: profile.profile_id,
                        counter: state.selection_count.saturating_add(1),
                    },
                )?)
            };
            let request = campaign_core::SelectionRequest {
                request_id: crate::harness::suite_id(campaign_core::IdKind::Request, "reach")?,
                campaign_id: state.admission.campaign_id,
                state_id: state.state_id,
                profile: profile.clone(),
                entropy: entropy.clone(),
                expected_generation: self.binding.generation,
                expected_head_id: self.binding.head_id,
            };
            let Ok(plan) = campaign_core::plan_next(state, request, &self.binding) else {
                continue;
            };
            match plan.decision {
                campaign_core::NextDecision::Select(decision) => {
                    if let Ok(event) = campaign_core::SelectionEvent::new(
                        decision.as_ref().clone(),
                        profile,
                        entropy,
                    ) {
                        published.push((
                            campaign_core::CampaignControlEvent::SelectionPublished(event),
                            Some(*decision),
                        ));
                    }
                }
                campaign_core::NextDecision::Stop(stop) => {
                    keep(published, campaign_core::admit_stop(state, stop.reason));
                }
                campaign_core::NextDecision::Wait(_) => {}
            }
        }
        Ok(())
    }
}

mod removals;

/// One abstract preference of `operation`.
fn preference(
    operation: crate::vocabulary::OperationHandle,
    generation: u64,
    levels: &[u32],
) -> crate::vocabulary::ScenarioPreference {
    crate::vocabulary::ScenarioPreference {
        operation,
        generation,
        score: crate::vocabulary::AbstractScore {
            levels: levels.to_vec(),
        },
    }
}

/// Keeps an event Campaign admits.
pub(super) fn keep(
    published: &mut alloc::vec::Vec<Published>,
    admitted: campaign_core::CampaignResult<campaign_core::CampaignControlEvent>,
) {
    if let Ok(event) = admitted {
        published.push((event, None));
    }
}
