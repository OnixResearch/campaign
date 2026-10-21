//! Session construction, exploration moments, and control-event publication.

mod origin;

/// Root moment every session admits with its campaign.
pub(crate) const ROOT: crate::vocabulary::MomentHandle = crate::vocabulary::MomentHandle(0);

impl<'s, 'a> super::Session<'s, 'a> {
    /// Opens one campaign on a graph of the suite's event bound (see [`Self::open_hosted`]).
    ///
    /// # Errors
    ///
    /// Returns the typed error of the adapter, Campaign, or Choregraph step that failed.
    pub(crate) fn open(
        harness: &'s mut super::Driver<'a>,
        budget_units: u64,
        configure: impl FnOnce(&mut campaign_core::ExplorationBounds),
    ) -> crate::verdicts::ConformanceResult<Self> {
        Self::open_hosted(harness, budget_units, super::GRAPH_MAX_EVENTS, configure)
    }

    /// Opens one campaign on a graph admitting at most `graph_max_events` events: admits the
    /// root exploration moment, binds the admission to that real branch value, and publishes the
    /// admission as the control branch root.
    ///
    /// # Errors
    ///
    /// Returns the typed error of the adapter, Campaign, or Choregraph step that failed.
    pub(crate) fn open_hosted(
        harness: &'s mut super::Driver<'a>,
        budget_units: u64,
        graph_max_events: u32,
        configure: impl FnOnce(&mut campaign_core::ExplorationBounds),
    ) -> crate::verdicts::ConformanceResult<Self> {
        let identities = harness.adapter.identities()?;
        let limits = campaign_choregraph::HistoryLimits::new(
            graph_max_events,
            super::HISTORY_MAX_PARENTS,
            super::HISTORY_MAX_METADATA,
            super::HISTORY_MAX_LABEL_BYTES,
            super::HISTORY_MAX_EVENTS,
        )?;
        let empty = campaign_choregraph::HistoryGraph::new(limits)?;
        let root_moment = crate::vocabulary::ScenarioMoment {
            handle: ROOT,
            parent: None,
        };
        let (graph, root) = admit_moment(&mut *harness.adapter, &empty, root_moment, None)?;
        let branch = campaign_choregraph::BranchRef {
            name: alloc::string::String::from(super::CONTROL_BRANCH),
            head: root.event,
            generation: super::ROOT_GENERATION,
        };
        let mut bounds = harness.profile.bounds.clone();
        configure(&mut bounds);
        let projection = campaign_choregraph::derive_projection_binding(
            &graph,
            &branch,
            identities.projection_schema_id,
            identities.adapter_id,
            identities.control_domain_id,
        )?;
        let event = campaign_core::admit_campaign(origin::admission(
            &identities,
            root.identity,
            projection,
            bounds.derive()?,
            budget_units,
        )?)?;
        let control_root = campaign_choregraph::admit_control_root(
            &graph,
            event.clone(),
            identities.projection_schema_id,
            identities.adapter_id,
            identities.control_domain_id,
        )?;
        let state = campaign_core::reconstruct(core::slice::from_ref(&event))?;
        let rule = campaign_core::AdmissionRule {
            duplicate_candidate: harness.profile.duplicate_candidate,
        };
        let root_branch = branch.clone();
        let branch = campaign_choregraph::BranchRef {
            head: control_root.envelope.history_event_id,
            generation: super::ADMITTED_GENERATION,
            ..branch
        };
        let payloads = alloc::vec![control_root.envelope];
        let origin = origin::cold_start(
            &identities,
            &control_root.graph_after_admission,
            &branch,
            &payloads,
        )?;
        debug_assert_eq!(state.admission.adapter_id, identities.adapter_id);
        debug_assert!(control_root.graph_after_admission.contains(root.event));
        debug_assert_eq!(*origin.state(), state);
        Ok(Self {
            harness,
            identities,
            graph: control_root.graph_after_admission,
            transitions: alloc::vec![super::decisions::committed(&root_branch, &branch)],
            branch,
            payloads,
            state,
            rule,
            moments: alloc::vec![root],
            operations: alloc::vec::Vec::new(),
            policy_inputs: alloc::vec::Vec::new(),
            projection: None,
            origin,
            witness: None,
        })
    }

    /// Admits one abstract moment as the adapter's exploration event and maps its identity.
    ///
    /// # Errors
    ///
    /// Returns the adapter or Choregraph error of the moment.
    pub(crate) fn add_moment(
        &mut self,
        moment: crate::vocabulary::ScenarioMoment,
    ) -> crate::verdicts::ConformanceResult<super::MomentEntry> {
        let parent_event = match moment.parent {
            Some(parent) => Some(self.moment(parent)?.event),
            None => None,
        };
        let (graph, entry) = admit_moment(
            &mut *self.harness.adapter,
            &self.graph,
            moment,
            parent_event,
        )?;
        self.graph = graph;
        self.moments.push(entry);
        debug_assert!(self.graph.contains(entry.event));
        Ok(entry)
    }

    /// Campaign binding of the current graph and control branch.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of a malformed binding.
    pub(crate) fn binding(
        &self,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ProjectionBinding> {
        Ok(campaign_choregraph::derive_projection_binding(
            &self.graph,
            &self.branch,
            self.identities.projection_schema_id,
            self.identities.adapter_id,
            self.identities.control_domain_id,
        )?)
    }

    /// Plans the publication of one control event against the current branch, as a fast-forward
    /// of the projection the session last validated (the admitted root before the first load).
    ///
    /// # Errors
    ///
    /// Returns the adapter error of a stale fence or malformed event.
    pub(crate) fn plan_publication(
        &self,
        event: campaign_core::CampaignControlEvent,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let choices = campaign_choregraph::ParentChoices::new();
        Ok(campaign_choregraph::plan_control_event_publication(
            campaign_choregraph::ControlPublicationRequest {
                current: campaign_choregraph::ControlProjectionRequest {
                    graph: &self.graph,
                    branch: &self.branch,
                    parent_choices: &choices,
                    maximum_events: super::HISTORY_MAX_EVENTS,
                    expected_binding: self.binding()?,
                    payloads: &self.payloads,
                },
                previous: self.held(),
                control_event: event,
            },
        )?)
    }

    /// The projection the session last validated: the last load, or the admitted root before
    /// the first load.
    fn held(&self) -> &campaign_choregraph::ValidatedControlProjection {
        self.projection.as_ref().unwrap_or(&self.origin)
    }

    /// Holds Campaign's own validation of the current history, advanced from the projection the
    /// session holds, without asking the adapter; later publications and loads advance from it,
    /// so a long lineage is not revalidated from its root on every publication.
    ///
    /// # Errors
    ///
    /// Returns Campaign's refusal of the current history.
    pub(crate) fn hold_current(&mut self) -> crate::verdicts::ConformanceResult<()> {
        let choices = campaign_choregraph::ParentChoices::new();
        let advanced = campaign_choregraph::validate_control_advance(
            self.held(),
            &campaign_choregraph::ControlProjectionRequest {
                graph: &self.graph,
                branch: &self.branch,
                parent_choices: &choices,
                maximum_events: super::HISTORY_MAX_EVENTS,
                expected_binding: self.binding()?,
                payloads: &self.payloads,
            },
        )?;
        debug_assert_eq!(*advanced.state(), self.state);
        self.projection = Some(advanced);
        Ok(())
    }

    /// Publishes one control event: commits the planned graph and branch advance, applies the
    /// event incrementally to the session state, and runs the session's witness, if any, against
    /// the state reached.
    ///
    /// # Errors
    ///
    /// Returns the adapter or Campaign error of the publication and the error of the witness.
    pub(crate) fn publish(
        &mut self,
        event: campaign_core::CampaignControlEvent,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let publication = self.plan_publication(event)?;
        let state =
            campaign_core::apply_control_event(self.state.clone(), publication.control_event())?;
        self.graph = publication.graph_after_admission().clone();
        self.branch = publication.branch_plan().after.clone();
        self.payloads
            .push(campaign_choregraph::ControlPayloadEnvelope {
                history_event_id: publication.history_event_id(),
                control_event: publication.control_event().clone(),
            });
        self.state = state;
        let transition =
            super::decisions::committed(&publication.branch_plan().before, &self.branch);
        self.transitions.push(transition);
        debug_assert_eq!(self.branch.head, publication.history_event_id());
        debug_assert_eq!(self.payloads.len(), self.control_events().len());
        if let Some(witness) = self.witness {
            witness(self)?;
        }
        Ok(publication)
    }

    /// Control events in published order.
    pub(crate) fn control_events(&self) -> alloc::vec::Vec<campaign_core::CampaignControlEvent> {
        self.payloads
            .iter()
            .map(|payload| payload.control_event.clone())
            .collect()
    }
}

/// Admits one abstract moment as the adapter's exploration event and maps its identity.
fn admit_moment(
    adapter: &mut dyn crate::port::ConformanceAdapter,
    graph: &campaign_choregraph::HistoryGraph,
    moment: crate::vocabulary::ScenarioMoment,
    parent_event: Option<campaign_choregraph::EventId>,
) -> crate::verdicts::ConformanceResult<(campaign_choregraph::HistoryGraph, super::MomentEntry)> {
    let draft = adapter.exploration_draft(&moment, parent_event)?;
    let admitted = graph.admit(draft)?;
    let identity = adapter.moment_identity(&moment, admitted.event)?;
    Ok((
        admitted.graph,
        super::MomentEntry {
            handle: moment.handle,
            event: admitted.event,
            identity,
        },
    ))
}
