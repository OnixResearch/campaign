static NO_PARENT_CHOICES: choregraph_history::ParentChoices =
    choregraph_history::ParentChoices::new();

pub(crate) struct Fixture {
    pub(crate) graph: choregraph_history::HistoryGraph,
    pub(crate) branch: choregraph_history::BranchRef,
    pub(crate) payloads: Vec<campaign_choregraph::ControlPayloadEnvelope>,
}
impl Fixture {
    fn projection_request(
        &self,
    ) -> Result<campaign_choregraph::ControlProjectionRequest<'_>, String> {
        let expected_binding =
            crate::suite::identity::observed(campaign_choregraph::derive_projection_binding(
                &self.graph,
                &self.branch,
                crate::suite::identity::schema(),
                crate::suite::identity::adapter(),
                crate::suite::identity::domain(),
            ))?;
        Ok(campaign_choregraph::ControlProjectionRequest {
            graph: &self.graph,
            branch: &self.branch,
            parent_choices: &NO_PARENT_CHOICES,
            maximum_events: crate::suite::constants::MAX_EVENTS,
            expected_binding,
            payloads: &self.payloads,
        })
    }
    pub(crate) fn current(
        &self,
    ) -> Result<campaign_choregraph::ValidatedControlProjection, String> {
        crate::suite::identity::observed(campaign_choregraph::validate_control_projection(
            &self.projection_request()?,
        ))
    }
    /// Publishes one event as a fast-forward of the projection it was computed from.
    pub(crate) fn append(
        mut self,
        event: campaign_core::CampaignControlEvent,
    ) -> Result<Self, String> {
        let previous = self.current()?;
        let publication =
            crate::suite::identity::observed(campaign_choregraph::plan_control_event_publication(
                campaign_choregraph::ControlPublicationRequest {
                    current: self.projection_request()?,
                    previous: &previous,
                    control_event: event,
                },
            ))?;
        self.payloads
            .push(campaign_choregraph::ControlPayloadEnvelope {
                history_event_id: publication.history_event_id(),
                control_event: publication.control_event().clone(),
            });
        self.graph = publication.graph_after_admission().clone();
        self.branch = publication.branch_plan().after.clone();
        Ok(self)
    }
}
