//! Control histories under host-configured envelope identities, projection bound, and admission
//! generation.

pub(crate) mod campaign;

static NO_PARENT_CHOICES: choregraph_history::ParentChoices =
    choregraph_history::ParentChoices::new();

/// Projection schema, adapter, and control-domain identities a host configures for envelopes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Envelope {
    pub(crate) schema: campaign_core::Id,
    pub(crate) adapter: campaign_core::Id,
    pub(crate) domain: campaign_core::Id,
}

impl Envelope {
    /// The identities the suite's admission binds.
    pub(crate) fn canonical() -> Self {
        Self {
            schema: crate::suite::identity::schema(),
            adapter: crate::suite::identity::adapter(),
            domain: crate::suite::identity::domain(),
        }
    }

    /// Identities of another envelope family.
    pub(crate) fn foreign() -> Self {
        const FOREIGN: &[u8] = b"hosted-foreign-envelope";
        Self {
            schema: crate::suite::identity::derive(campaign_core::IdKind::Projection, FOREIGN),
            adapter: crate::suite::identity::derive(campaign_core::IdKind::Adapter, FOREIGN),
            domain: crate::suite::identity::derive(campaign_core::IdKind::ControlDomain, FOREIGN),
        }
    }
}

/// One control history value with its host configuration.
#[derive(Clone)]
pub(crate) struct Hosted {
    pub(crate) graph: choregraph_history::HistoryGraph,
    pub(crate) branch: choregraph_history::BranchRef,
    pub(crate) payloads: Vec<campaign_choregraph::ControlPayloadEnvelope>,
    pub(crate) envelope: Envelope,
    pub(crate) maximum_events: u32,
}

impl Hosted {
    /// This history under other host envelope identities.
    pub(crate) fn under(&self, envelope: Envelope) -> Self {
        Self {
            envelope,
            ..self.clone()
        }
    }

    pub(crate) fn binding(
        &self,
    ) -> campaign_choregraph::AdapterResult<campaign_core::ProjectionBinding> {
        campaign_choregraph::derive_projection_binding(
            &self.graph,
            &self.branch,
            self.envelope.schema,
            self.envelope.adapter,
            self.envelope.domain,
        )
    }

    pub(crate) fn request(
        &self,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ControlProjectionRequest<'_>> {
        Ok(campaign_choregraph::ControlProjectionRequest {
            graph: &self.graph,
            branch: &self.branch,
            parent_choices: &NO_PARENT_CHOICES,
            maximum_events: self.maximum_events,
            expected_binding: self.binding()?,
            payloads: &self.payloads,
        })
    }

    /// Cold-start load.
    pub(crate) fn validate(
        &self,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        campaign_choregraph::validate_control_projection(&self.request()?)
    }

    /// Load as a fast-forward of a held projection.
    pub(crate) fn advance(
        &self,
        previous: &campaign_choregraph::ValidatedControlProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        campaign_choregraph::validate_control_advance(previous, &self.request()?)
    }

    pub(crate) fn publish(
        &self,
        previous: &campaign_choregraph::ValidatedControlProjection,
        control_event: campaign_core::CampaignControlEvent,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ControlPublication> {
        campaign_choregraph::plan_control_event_publication(
            campaign_choregraph::ControlPublicationRequest {
                current: self.request()?,
                previous,
                control_event,
            },
        )
    }

    /// Validates the derived cache of `cache` against this history, cold or held.
    pub(crate) fn validate_cache(
        &self,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        campaign_choregraph::validate_cache_against_history(
            &campaign_choregraph::CacheValidationRequest {
                graph: &self.graph,
                branch: &self.branch,
                parent_choices: &NO_PARENT_CHOICES,
                maximum_events: self.maximum_events,
                projection_schema_id: self.envelope.schema,
                adapter_id: self.envelope.adapter,
                control_domain_id: self.envelope.domain,
                payloads: &self.payloads,
                previous,
            },
            cache,
        )
    }

    /// The history after the host durably commits `publication`.
    pub(crate) fn committed(&self, publication: &campaign_choregraph::ControlPublication) -> Self {
        let mut next = self.clone();
        next.payloads
            .push(campaign_choregraph::ControlPayloadEnvelope {
                history_event_id: publication.history_event_id(),
                control_event: publication.control_event().clone(),
            });
        next.graph = publication.graph_after_admission().clone();
        next.branch = publication.branch_plan().after.clone();
        next
    }

    /// Loads, publishes `control_event` through the loaded projection, and commits it.
    pub(crate) fn append(
        self,
        control_event: campaign_core::CampaignControlEvent,
    ) -> Result<(Self, campaign_choregraph::ControlPublication), String> {
        let held = crate::suite::identity::observed(self.validate())?;
        let publication = crate::suite::identity::observed(self.publish(&held, control_event))?;
        Ok((self.committed(&publication), publication))
    }
}
