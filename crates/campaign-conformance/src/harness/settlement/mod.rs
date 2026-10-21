//! Results, closures, retirement, declared pruning, and stops mapped through the adapter.

/// Suite retention profile name.
const RETENTION_PROFILE: &str = "retention-profile";

/// Forbidden effect of a closure that closes another selection than the named one.
pub(crate) const CROSSED_CLOSURE: &str = "a closure closed a selection other than the named one";

impl super::Session<'_, '_> {
    /// Reserves the child observations of one execution: every child is a known child moment.
    ///
    /// # Errors
    ///
    /// Returns `InvalidIdentities` when a child handle maps to a root identity.
    pub(crate) fn reserve_children(
        &self,
        selection: &campaign_core::SelectionDecision,
        children: &[crate::vocabulary::MomentHandle],
    ) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::ChildObservation>> {
        let mut reserved = alloc::vec::Vec::with_capacity(children.len());
        for handle in children {
            let crate::port::MomentIdentity::Child {
                moment_id,
                event_payload_id,
            } = self.moment(*handle)?.identity
            else {
                return Err(crate::verdicts::ConformanceError::InvalidIdentities);
            };
            reserved.push(
                campaign_core::ChildObservation {
                    reservation_id: campaign_core::Id::absent(),
                    child_moment_id: moment_id,
                    discovery_id: campaign_core::Id::absent(),
                    event_payload_id,
                }
                .derive(self.state.admission.campaign_id, selection.selection_id)?,
            );
        }
        debug_assert_eq!(reserved.len(), children.len());
        Ok(reserved)
    }

    /// Maps one outcome of an executed selection to the adapter's result observation.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the mapping.
    pub(crate) fn observation(
        &mut self,
        selection: &campaign_core::SelectionDecision,
        outcome: &crate::vocabulary::ScenarioOutcome,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        let children = self.reserve_children(selection, &outcome.children)?;
        let context = crate::port::ResultContext {
            campaign_id: self.state.admission.campaign_id,
            selection,
            children: &children,
            state: &self.state,
        };
        self.harness.adapter.result(outcome, &context)
    }

    /// Admits and publishes one observed outcome; the transcript records the children Campaign
    /// accepted, not the children the scenario declared.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the result.
    pub(crate) fn settle(
        &mut self,
        selection: &campaign_core::SelectionDecision,
        outcome: &crate::vocabulary::ScenarioOutcome,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        let observation = self.observation(selection, outcome)?;
        let event = campaign_core::admit_result(&self.state, observation)?;
        let campaign_core::CampaignControlEvent::ResultObserved(result) = &event else {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "result admission produces a result event",
            });
        };
        let accepted = result.observation.clone();
        let settled = self.operation_of(self.selected_candidate(accepted.selection_id)?)?;
        let mut children = alloc::vec::Vec::with_capacity(accepted.children.len());
        for child in &accepted.children {
            children.push(self.moment_of(child.child_moment_id)?);
        }
        self.publish(event)?;
        self.record(
            crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::SETTLED, "")
                .with_operations(alloc::vec![settled])
                .with_moments(children),
        );
        Ok(accepted)
    }

    /// Candidate of an outstanding or settled selection known to this session.
    pub(crate) fn selected_candidate(
        &self,
        selection_id: campaign_core::Id,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        self.state
            .outstanding
            .iter()
            .find(|outstanding| outstanding.selection_id == selection_id)
            .map(|outstanding| outstanding.candidate_id)
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "the result settles an outstanding selection",
            })
    }

    /// Admits and publishes the adapter's acknowledged closure of one selection; the closure must
    /// close exactly the named selection.
    ///
    /// # Errors
    ///
    /// Returns `ForbiddenAccepted` for a closure of another selection, and the adapter, Campaign,
    /// or Choregraph error of the closure.
    // r[impl campaign.conformance.bounds]
    pub(crate) fn close(
        &mut self,
        selection: &campaign_core::SelectionDecision,
    ) -> crate::verdicts::ConformanceResult<()> {
        let closure = self.closure(selection)?;
        if closure.selection_id != selection.selection_id {
            return Err(crate::verdicts::ConformanceError::ForbiddenAccepted {
                forbidden: CROSSED_CLOSURE,
            });
        }
        let event = campaign_core::close_unobserved(&self.state, closure)?;
        let closed = self.operation_of(selection.candidate_id)?;
        self.publish(event)?;
        self.record(
            crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::CLOSED, "")
                .with_operations(alloc::vec![closed]),
        );
        Ok(())
    }

    /// Maps one closure through the adapter.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the mapping.
    pub(crate) fn closure(
        &mut self,
        selection: &campaign_core::SelectionDecision,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
        let context = crate::port::ClosureContext {
            campaign_id: self.state.admission.campaign_id,
            selection,
        };
        self.harness.adapter.closure(&context)
    }

    /// Consumer retirement request of one operation.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the reason mapping.
    pub(crate) fn retirement(
        &mut self,
        operation: crate::vocabulary::OperationHandle,
    ) -> crate::verdicts::ConformanceResult<campaign_core::RetirementRequest> {
        Ok(campaign_core::RetirementRequest {
            campaign_id: self.state.admission.campaign_id,
            state_id: self.state.state_id,
            policy_id: self.state.admission.policy_id,
            adapter_id: self.state.admission.adapter_id,
            candidate_id: self.candidate_id(operation)?,
            reason_id: self
                .harness
                .adapter
                .reason(crate::vocabulary::ScenarioReason::Retirement)?,
            origin: campaign_core::RetirementOrigin::Consumer,
        })
    }

    /// Retires and publishes one operation.
    ///
    /// # Errors
    ///
    /// Returns the Campaign or Choregraph error of the retirement.
    pub(crate) fn retire(
        &mut self,
        operation: crate::vocabulary::OperationHandle,
    ) -> crate::verdicts::ConformanceResult<()> {
        let request = self.retirement(operation)?;
        let event = campaign_core::retire_candidate(&self.state, request)?;
        self.publish(event)?;
        self.record(
            crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::RETIRED, "")
                .with_operations(alloc::vec![operation]),
        );
        Ok(())
    }

    /// Plans the declared retention of the profile's rank order.
    ///
    /// # Errors
    ///
    /// Returns the Campaign error of the plan.
    pub(crate) fn retention_plan(
        &mut self,
    ) -> crate::verdicts::ConformanceResult<Option<campaign_core::PruneRequest>> {
        let request = campaign_core::RetentionPlanRequest {
            campaign_id: self.state.admission.campaign_id,
            state_id: self.state.state_id,
            retention_profile_id: super::suite_id(
                campaign_core::IdKind::RetentionProfile,
                RETENTION_PROFILE,
            )?,
            rank_order: self.harness.profile.retention_rank_order.clone(),
            reason_id: self
                .harness
                .adapter
                .reason(crate::vocabulary::ScenarioReason::Pruning)?,
        };
        Ok(campaign_core::plan_pruning(&self.state, request)?)
    }

    /// Prunes and publishes the declared retention plan; returns the pruned operations.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when nothing is over the retention bound and the Campaign or
    /// Choregraph error of the prune.
    pub(crate) fn prune(
        &mut self,
    ) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<crate::vocabulary::OperationHandle>>
    {
        let request =
            self.retention_plan()?
                .ok_or(crate::verdicts::ConformanceError::Diverged {
                    expected: "the frontier exceeds its retention bound",
                })?;
        let mut pruned = alloc::vec::Vec::with_capacity(request.candidates.len());
        for candidate_id in &request.candidates {
            pruned.push(self.operation_of(*candidate_id)?);
        }
        let event = campaign_core::prune_frontier(&self.state, request)?;
        self.publish(event)?;
        pruned.sort();
        self.record(
            crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::PRUNED, "")
                .with_operations(pruned.clone()),
        );
        Ok(pruned)
    }

    /// Admits and publishes one stop; returns its publication.
    ///
    /// # Errors
    ///
    /// Returns the Campaign or Choregraph error of the stop.
    pub(crate) fn stop(
        &mut self,
        reason: campaign_core::StopReason,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let label = super::labels::stop(&reason, &self.identities.resource_dimension_ids);
        let event = campaign_core::admit_stop(&self.state, reason)?;
        let publication = self.publish(event)?;
        self.record(crate::ledger::TranscriptEntry::labelled(
            crate::ledger::kinds::STOPPED,
            &label,
        ));
        Ok(publication)
    }

    /// Consumer stop reason mapped through the adapter.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the reason mapping.
    pub(crate) fn consumer_stop(
        &mut self,
    ) -> crate::verdicts::ConformanceResult<campaign_core::StopReason> {
        Ok(campaign_core::StopReason::ConsumerRequested {
            reason_id: self
                .harness
                .adapter
                .reason(crate::vocabulary::ScenarioReason::ConsumerStop)?,
        })
    }
}
