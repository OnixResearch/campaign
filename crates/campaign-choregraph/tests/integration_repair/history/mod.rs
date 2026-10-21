static NO_PARENT_CHOICES: choregraph_history::ParentChoices =
    choregraph_history::ParentChoices::new();

#[derive(Clone)]
pub(crate) struct Fixture {
    pub(crate) graph: choregraph_history::HistoryGraph,
    pub(crate) branch: choregraph_history::BranchRef,
    pub(crate) payloads: Vec<campaign_choregraph::ControlPayloadEnvelope>,
}
impl Fixture {
    pub(crate) fn current(
        &self,
    ) -> Result<campaign_choregraph::ValidatedControlProjection, String> {
        crate::suite::identity::observed(self.validate())
    }
    /// The binding the adapter derives for this fixture's graph and branch.
    pub(crate) fn derived_binding(
        &self,
    ) -> campaign_choregraph::AdapterResult<campaign_core::ProjectionBinding> {
        campaign_choregraph::derive_projection_binding(
            &self.graph,
            &self.branch,
            crate::suite::identity::schema(),
            crate::suite::identity::adapter(),
            crate::suite::identity::domain(),
        )
    }
    /// The projection inputs of this fixture under the supplied expected binding.
    pub(crate) fn projection_request(
        &self,
        expected_binding: campaign_core::ProjectionBinding,
    ) -> campaign_choregraph::ControlProjectionRequest<'_> {
        campaign_choregraph::ControlProjectionRequest {
            graph: &self.graph,
            branch: &self.branch,
            parent_choices: &NO_PARENT_CHOICES,
            maximum_events: crate::suite::constants::MAX_EVENTS,
            expected_binding,
            payloads: &self.payloads,
        }
    }
    /// Validates the control branch with the adapter's typed result.
    pub(crate) fn validate(
        &self,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        campaign_choregraph::validate_control_projection(
            &self.projection_request(self.derived_binding()?),
        )
    }
    /// Validates the control branch as a fast-forward of a previously validated projection.
    pub(crate) fn advance(
        &self,
        previous: &campaign_choregraph::ValidatedControlProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        campaign_choregraph::validate_control_advance(
            previous,
            &self.projection_request(self.derived_binding()?),
        )
    }
    /// Plans the publication of one event as a cold-start host: loads the projection through
    /// the only cold-start entry, then publishes as its fast-forward.
    pub(crate) fn publish(
        &self,
        event: campaign_core::CampaignControlEvent,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ControlPublication> {
        self.publish_from(&self.validate()?, event)
    }
    /// Plans the publication of one event as a fast-forward of the projection it was computed
    /// from.
    pub(crate) fn publish_from(
        &self,
        previous: &campaign_choregraph::ValidatedControlProjection,
        event: campaign_core::CampaignControlEvent,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ControlPublication> {
        campaign_choregraph::plan_control_event_publication(
            campaign_choregraph::ControlPublicationRequest {
                current: self.projection_request(self.derived_binding()?),
                previous,
                control_event: event,
            },
        )
    }
    pub(crate) fn append(
        mut self,
        event: campaign_core::CampaignControlEvent,
    ) -> Result<(Self, campaign_choregraph::ControlPublication), String> {
        let publication = crate::suite::identity::observed(self.publish(event))?;
        self.payloads
            .push(campaign_choregraph::ControlPayloadEnvelope {
                history_event_id: publication.history_event_id(),
                control_event: publication.control_event().clone(),
            });
        self.graph = publication.graph_after_admission().clone();
        self.branch = publication.branch_plan().after.clone();
        Ok((self, publication))
    }
}
