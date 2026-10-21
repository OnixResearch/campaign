//! Retirements, prunes (declared and every explicit list), stops, and settlements a consumer
//! publishes on one search state.

impl super::Context<'_, '_, '_> {
    pub(super) fn removals(
        &mut self,
        state: &campaign_core::FrontierState,
        published: &mut alloc::vec::Vec<super::Published>,
    ) -> crate::verdicts::ConformanceResult<()> {
        let adapter = &mut *self.session.harness.adapter;
        let retirement = adapter.reason(crate::vocabulary::ScenarioReason::Retirement)?;
        for (_, record) in &self.records {
            for origin in [
                campaign_core::RetirementOrigin::Consumer,
                campaign_core::RetirementOrigin::Policy,
            ] {
                let request = campaign_core::RetirementRequest {
                    campaign_id: state.admission.campaign_id,
                    state_id: state.state_id,
                    policy_id: state.admission.policy_id,
                    adapter_id: state.admission.adapter_id,
                    candidate_id: record.candidate_id,
                    reason_id: retirement,
                    origin,
                };
                super::keep(published, campaign_core::retire_candidate(state, request));
            }
        }
        let request = campaign_core::RetentionPlanRequest {
            campaign_id: state.admission.campaign_id,
            state_id: state.state_id,
            retention_profile_id: crate::harness::suite_id(
                campaign_core::IdKind::RetentionProfile,
                "retention-profile",
            )?,
            rank_order: self.session.harness.profile.retention_rank_order.clone(),
            reason_id: adapter.reason(crate::vocabulary::ScenarioReason::Pruning)?,
        };
        let reason_id = request.reason_id;
        if let Ok(Some(prune)) = campaign_core::plan_pruning(state, request) {
            super::keep(published, campaign_core::prune_frontier(state, prune));
        }
        let prunable: alloc::vec::Vec<campaign_core::Id> = self
            .records
            .iter()
            .map(|(_, record)| record.candidate_id)
            .filter(|candidate_id| {
                state.candidate(*candidate_id).is_ok_and(|candidate| {
                    candidate.status == campaign_core::CandidateStatus::Retained
                }) && !state.has_outstanding_for_candidate(*candidate_id)
            })
            .collect();
        for subset in 1..(1_usize << prunable.len()) {
            let candidates = prunable
                .iter()
                .enumerate()
                .filter(|(position, _)| subset & (1 << position) != 0)
                .map(|(_, candidate_id)| *candidate_id)
                .collect();
            let explicit = campaign_core::PruneRequest {
                prune_plan_id: campaign_core::Id::absent(),
                campaign_id: state.admission.campaign_id,
                state_id: state.state_id,
                candidates,
                reason_id,
                retention_profile_id: None,
                rank_order: alloc::vec::Vec::new(),
                claims_artifact_deletion: false,
                claims_history_rewrite: false,
                claims_safe_deletion: false,
            };
            super::keep(published, campaign_core::prune_frontier(state, explicit));
        }
        Ok(())
    }

    pub(super) fn stops(
        &mut self,
        state: &campaign_core::FrontierState,
        published: &mut alloc::vec::Vec<super::Published>,
    ) -> crate::verdicts::ConformanceResult<()> {
        let policy = campaign_core::StoppingRule {
            finding_limit: Some(1),
        }
        .policy_reason_id()?
        .ok_or(campaign_core::CampaignError::InvalidConfig)?;
        let reasons = [
            self.session.consumer_stop()?,
            campaign_core::StopReason::ConsumerRequested {
                reason_id: campaign_choregraph::control_capacity_reason_id(),
            },
            campaign_core::StopReason::PolicyRequested { reason_id: policy },
        ];
        for reason in reasons {
            super::keep(published, campaign_core::admit_stop(state, reason));
        }
        Ok(())
    }

    pub(super) fn settlements(
        &mut self,
        state: &campaign_core::FrontierState,
        decision: &campaign_core::SelectionDecision,
        published: &mut alloc::vec::Vec<super::Published>,
    ) -> crate::verdicts::ConformanceResult<()> {
        let campaign_id = state.admission.campaign_id;
        let child = campaign_core::ChildObservation {
            reservation_id: campaign_core::Id::absent(),
            child_moment_id: campaign_core::derive_id(
                campaign_core::IdKind::ChildMoment,
                b"reach-child",
                &[decision.selection_id],
            ),
            discovery_id: campaign_core::Id::absent(),
            event_payload_id: campaign_core::derive_id(
                campaign_core::IdKind::EventPayload,
                b"reach-child-payload",
                &[decision.selection_id],
            ),
        }
        .derive(campaign_id, decision.selection_id)?;
        let adapter = &mut *self.session.harness.adapter;
        for (children, findings) in [(false, 0), (true, 0), (false, 1), (true, 1)] {
            let reserved = if children {
                alloc::vec![child.clone()]
            } else {
                alloc::vec::Vec::new()
            };
            let outcome = crate::vocabulary::ScenarioOutcome {
                children: alloc::vec::Vec::new(),
                findings,
            };
            let context = crate::port::ResultContext {
                campaign_id,
                selection: decision,
                children: &reserved,
                state,
            };
            let observation = adapter.result(&outcome, &context)?;
            super::keep(published, campaign_core::admit_result(state, observation));
        }
        let closure = adapter.closure(&crate::port::ClosureContext {
            campaign_id,
            selection: decision,
        })?;
        super::keep(published, campaign_core::close_unobserved(state, closure));
        Ok(())
    }
}
