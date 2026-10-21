//! A campaign admission bound onto an exploration graph under explicit host limits, before and
//! after its control root is admitted.

/// The host limits one configured campaign is admitted under.
#[derive(Clone, Debug)]
pub(crate) struct Limits {
    /// Generation of the admission's projection binding.
    pub(crate) generation: u64,
    /// The graph's own event limit (`HistoryLimits::max_events`).
    pub(crate) graph_events: u32,
    /// The graph's projection limit (`HistoryLimits::max_projection_events`).
    pub(crate) projection_events: u32,
    /// The host projection bound (`maximum_events`) the history is served under.
    pub(crate) maximum_events: u32,
}

impl Limits {
    /// A graph of the suite's standard event limit whose projection limit and host bound are both
    /// `maximum_events`.
    pub(crate) const fn standard(generation: u64, maximum_events: u32) -> Self {
        Self {
            generation,
            graph_events: crate::suite::constants::MAX_EVENTS,
            projection_events: maximum_events,
            maximum_events,
        }
    }
}

/// An exploration graph under `limits` holding the suite's exploration root.
pub(crate) fn exploration(limits: &Limits) -> Result<choregraph_history::Admission, String> {
    let history_limits = crate::suite::identity::observed(choregraph_history::HistoryLimits::new(
        limits.graph_events,
        crate::suite::constants::HISTORY_MAX_PARENTS,
        crate::suite::constants::MAX_METADATA,
        crate::suite::constants::MAX_LABEL_BYTES,
        limits.projection_events,
    ))?;
    let empty =
        crate::suite::identity::observed(choregraph_history::HistoryGraph::new(history_limits))?;
    let exploration = crate::suite::identity::observed(
        empty.admit(choregraph_history::EventDraft::v1(
            choregraph_history::EventClass::Application,
            choregraph_history::Digest::from_bytes(
                crate::suite::identity::derive(
                    campaign_core::IdKind::EventPayload,
                    b"repair-exploration-root",
                )
                .digest,
            ),
            Vec::new(),
            choregraph_history::Digest::from_bytes(crate::suite::identity::schema().digest),
        )),
    )?;
    assert_eq!(exploration.graph.len(), 1);
    assert_eq!(exploration.graph.limits().max_events, limits.graph_events);
    Ok(exploration)
}

/// The suite's admission event bound onto an exploration graph, not yet admitted as a root.
pub(crate) struct Unadmitted {
    /// The exploration graph the control root is admitted to.
    pub(crate) graph: choregraph_history::HistoryGraph,
    /// The control branch value the admission binds, headed at the exploration root.
    pub(crate) before: choregraph_history::BranchRef,
    /// The `CampaignAdmitted` event.
    pub(crate) event: campaign_core::CampaignControlEvent,
    /// The limits the admission is bound under.
    pub(crate) limits: Limits,
}

impl Unadmitted {
    /// Binds the suite's admission at `limits.generation` onto a fresh exploration graph.
    pub(crate) fn bound(limits: Limits) -> Result<Self, String> {
        let canonical = crate::suite::admission::initialized()?;
        let campaign_core::CampaignControlEvent::CampaignAdmitted { admission, .. } =
            canonical.payloads[0].control_event.clone()
        else {
            return Err(String::from("the suite's root is its admission"));
        };
        let exploration = exploration(&limits)?;
        let before = choregraph_history::BranchRef {
            name: canonical.branch.name.clone(),
            head: exploration.event,
            generation: limits.generation,
        };
        let envelope = crate::suite::configured::Envelope::canonical();
        let projection =
            crate::suite::identity::observed(campaign_choregraph::derive_projection_binding(
                &exploration.graph,
                &before,
                envelope.schema,
                envelope.adapter,
                envelope.domain,
            ))?;
        let mut rebound = admission;
        rebound.campaign_id = campaign_core::Id::absent();
        rebound.roots[0].projection_id = projection.projection_id;
        rebound.projection = projection;
        let event = crate::suite::identity::observed(campaign_core::admit_campaign(rebound))?;
        Ok(Self {
            graph: exploration.graph,
            before,
            event,
            limits,
        })
    }

    /// Admits the control root through `admit_control_root` under the canonical envelope.
    pub(crate) fn root(
        &self,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ControlRootAdmission> {
        let envelope = crate::suite::configured::Envelope::canonical();
        campaign_choregraph::admit_control_root(
            &self.graph,
            self.event.clone(),
            envelope.schema,
            envelope.adapter,
            envelope.domain,
        )
    }

    /// The history holding the admitted control root, served under `limits.maximum_events`.
    pub(crate) fn hosted(self) -> Result<crate::suite::configured::Hosted, String> {
        let root = crate::suite::identity::observed(self.root())?;
        let generation = self
            .limits
            .generation
            .checked_add(crate::suite::constants::BRANCH_EVENT_INCREMENT)
            .ok_or_else(|| String::from("the admission generation leaves no control root"))?;
        assert_eq!(root.envelope.control_event, self.event);
        assert!(root.graph_after_admission.contains(self.before.head));
        Ok(crate::suite::configured::Hosted {
            graph: root.graph_after_admission,
            branch: choregraph_history::BranchRef {
                name: self.before.name,
                head: root.envelope.history_event_id,
                generation,
            },
            payloads: vec![root.envelope],
            envelope: crate::suite::configured::Envelope::canonical(),
            maximum_events: self.limits.maximum_events,
        })
    }
}
