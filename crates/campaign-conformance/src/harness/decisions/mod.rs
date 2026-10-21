//! Selection requests, published-projection planning, publication, and the execution gate.

pub(crate) mod entropy;
mod published;

/// Suite scheduling policy name.
const SCHEDULING_POLICY: &str = "scheduling-policy";

/// Suite selection request name.
const SELECTION_REQUEST: &str = "selection-request";

/// One planned decision and the exact request it answered.
#[derive(Clone, Debug)]
pub(crate) struct Planned {
    pub(crate) request: campaign_core::SelectionRequest,
    pub(crate) plan: campaign_core::DecisionPlan,
    pub(crate) binding: campaign_core::ProjectionBinding,
}

impl Planned {
    /// The selection this plan made.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when the plan waited or stopped.
    pub(crate) fn selection(
        &self,
    ) -> crate::verdicts::ConformanceResult<campaign_core::SelectionDecision> {
        let campaign_core::NextDecision::Select(selection) = &self.plan.decision else {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "the planner selects a candidate",
            });
        };
        Ok(selection.as_ref().clone())
    }
}

impl super::Session<'_, '_> {
    /// Selection profile of the suite: ranked, or exploratory with the profile's declared
    /// threshold and exploration order.
    ///
    /// # Errors
    ///
    /// Returns a typed Campaign error for an invalid profile.
    pub(crate) fn selection_profile(
        &self,
        is_exploratory: bool,
    ) -> crate::verdicts::ConformanceResult<campaign_core::SelectionProfile> {
        let mode = if is_exploratory {
            campaign_core::SelectionMode::Exploratory {
                threshold_parts_per_million: self
                    .harness
                    .profile
                    .exploratory_threshold_parts_per_million,
                entropy_source_id: self.identities.entropy_source_id,
                exploration_order: self.harness.profile.exploration_order.clone(),
            }
        } else {
            campaign_core::SelectionMode::Ranked
        };
        Ok(campaign_core::SelectionProfile {
            profile_id: campaign_core::Id::absent(),
            scheduling_policy_id: super::suite_id(
                campaign_core::IdKind::SchedulingProfile,
                SCHEDULING_POLICY,
            )?,
            admission_policy_id: self.rule.admission_policy_id()?,
            coalescing_policy_id: super::records::coalescing_policy_id()?,
            mode,
            decay: campaign_core::SelectionCountDecay::None,
            stopping: campaign_core::StoppingRule {
                finding_limit: None,
            },
            implementation: campaign_core::PolicyImplementation::ProductionCompiled {
                policy_id: self.state.admission.policy_id,
            },
        }
        .derive()?)
    }

    /// Builds the request for the current state: ranked without a draw, exploratory with an
    /// adapter-issued ticket carrying the draw.
    ///
    /// # Errors
    ///
    /// Returns the adapter or Campaign error of the request.
    pub(crate) fn request(
        &mut self,
        draw: Option<crate::vocabulary::ScenarioDraw>,
        binding: &campaign_core::ProjectionBinding,
    ) -> crate::verdicts::ConformanceResult<campaign_core::SelectionRequest> {
        let profile = self.selection_profile(draw.is_some())?;
        let entropy = match draw {
            Some(draw) => Some(self.ticket(&draw, &profile)?),
            None => None,
        };
        self.policy_inputs.push((profile.clone(), entropy.clone()));
        Ok(campaign_core::SelectionRequest {
            request_id: super::suite_id(campaign_core::IdKind::Request, SELECTION_REQUEST)?,
            campaign_id: self.state.admission.campaign_id,
            state_id: self.state.state_id,
            profile,
            entropy,
            expected_generation: binding.generation,
            expected_head_id: binding.head_id,
        })
    }

    /// Plans the next decision against the projection the adapter loads and records it.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the plan.
    pub(crate) fn plan(
        &mut self,
        draw: Option<crate::vocabulary::ScenarioDraw>,
    ) -> crate::verdicts::ConformanceResult<Planned> {
        let current = self.load()?;
        let request = self.request(draw, current.binding())?;
        let plan = campaign_choregraph::plan_next_from_published_projection(
            &current,
            campaign_choregraph::PublishedSelectionRequest {
                request: request.clone(),
            },
        )?;
        self.record_decision(&plan.decision)?;
        debug_assert_eq!(plan.state_id, self.state.state_id);
        debug_assert_eq!(request.expected_head_id, current.binding().head_id);
        Ok(Planned {
            request,
            plan,
            binding: current.binding().clone(),
        })
    }

    fn record_decision(
        &mut self,
        decision: &campaign_core::NextDecision,
    ) -> crate::verdicts::ConformanceResult<()> {
        let entry = match decision {
            campaign_core::NextDecision::Select(selection) => {
                crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::SELECTED, "")
                    .with_operations(alloc::vec![self.operation_of(selection.candidate_id)?])
                    .with_ordinal(selection.ordinal)
            }
            campaign_core::NextDecision::Wait(reason) => crate::ledger::TranscriptEntry::labelled(
                crate::ledger::kinds::WAITED,
                super::labels::wait(reason),
            ),
            campaign_core::NextDecision::Stop(stop) => crate::ledger::TranscriptEntry::labelled(
                crate::ledger::kinds::STOPPED,
                &super::labels::stop(&stop.reason, &self.identities.resource_dimension_ids),
            ),
        };
        self.record(entry);
        Ok(())
    }

    /// Plans and requires a selection.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when the planner waits or stops.
    pub(crate) fn select(
        &mut self,
        draw: Option<crate::vocabulary::ScenarioDraw>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::SelectionDecision> {
        self.plan(draw)?.selection()
    }

    /// Publishes one selection event.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of a stale fence and the Campaign error of the event.
    pub(crate) fn publish_selection(
        &mut self,
        selection: &campaign_core::SelectionDecision,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let event = self.selection_event(selection)?;
        self.publish(event)
    }

    /// Selects, publishes durably, and requires the gate to grant execution.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when the planner does not select or the gate refuses a durable
    /// publication.
    pub(crate) fn execute(
        &mut self,
        draw: Option<crate::vocabulary::ScenarioDraw>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::SelectionDecision> {
        let planned = self.plan(draw)?;
        self.execute_planned(&planned)
    }

    /// Publishes the selection of one plan durably and requires the gate to grant execution.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when the plan does not select or the gate refuses a durable
    /// publication.
    pub(crate) fn execute_planned(
        &mut self,
        planned: &Planned,
    ) -> crate::verdicts::ConformanceResult<campaign_core::SelectionDecision> {
        let selection = planned.selection()?;
        let publication = self.publish_selection(&selection)?;
        let evidence = durable_evidence(&publication, Durability::COMMITTED);
        let authority = self.gate(
            &selection,
            Some(&publication),
            Some(&evidence),
            "a selection executed without Campaign's publication gate",
        )?;
        if authority != crate::port::ExecutionAuthority::Granted {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: published::GRANTED_PUBLICATION,
            });
        }
        Ok(selection)
    }
}

/// Durability a host observed for one planned publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Durability {
    /// The history event was durably admitted.
    pub(crate) is_event_durable: bool,
    /// The branch compare-and-set was durably committed.
    pub(crate) is_branch_update_durable: bool,
}

impl Durability {
    /// Event and branch update both durable.
    pub(crate) const COMMITTED: Self = Self {
        is_event_durable: true,
        is_branch_update_durable: true,
    };
    /// Nothing durable yet.
    pub(crate) const PENDING: Self = Self {
        is_event_durable: false,
        is_branch_update_durable: false,
    };
}

/// Committed evidence of one branch transition from `before` onto `after`'s head.
pub(crate) fn committed(
    before: &campaign_choregraph::BranchRef,
    after: &campaign_choregraph::BranchRef,
) -> campaign_choregraph::DurablePublicationEvidence {
    campaign_choregraph::DurablePublicationEvidence {
        history_event_id: after.head,
        branch_before: before.clone(),
        branch_after: after.clone(),
        durable_event: true,
        durable_branch_update: true,
    }
}

/// Durability facts of one planned publication as its host observed them.
pub(crate) fn durable_evidence(
    publication: &campaign_choregraph::ControlPublication,
    durability: Durability,
) -> campaign_choregraph::DurablePublicationEvidence {
    campaign_choregraph::DurablePublicationEvidence {
        history_event_id: publication.history_event_id(),
        branch_before: publication.branch_plan().before.clone(),
        branch_after: publication.branch_plan().after.clone(),
        durable_event: durability.is_event_durable,
        durable_branch_update: durability.is_branch_update_durable,
    }
}
