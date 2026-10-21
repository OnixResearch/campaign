//! The host bounds every headroom control is exercised under.

/// One host bound that limits a campaign's control lineage below its `max_control_events`.
#[derive(Clone, Debug)]
pub(super) struct Bound {
    /// The host limits the campaign is admitted under.
    pub(super) limits: crate::suite::configured::campaign::root::Limits,
    /// The typed refusal of an event beyond this bound.
    pub(super) exhausted: campaign_choregraph::AdapterError,
}

impl Bound {
    /// A freshly admitted campaign under this bound.
    pub(super) fn admitted(&self) -> Result<crate::suite::configured::Hosted, String> {
        crate::suite::configured::campaign::admitted_under(self.limits.clone())
    }

    /// Generation of the control branch after `events` control events.
    pub(super) fn generation_after(&self, events: u64) -> Result<u64, String> {
        self.limits
            .generation
            .checked_add(events)
            .ok_or_else(|| String::from("the bound admits the events"))
    }
}

/// Every host bound at exactly `events` control events: the Choregraph projection bound, the
/// control branch generations left after the admission's binding, and the graph event limit of a
/// graph that also holds the exploration root.
pub(super) fn bounds(events: u32) -> Result<[Bound; 3], String> {
    let generation = u64::MAX
        .checked_sub(u64::from(events))
        .ok_or_else(|| String::from("a generation below u64::MAX"))?;
    let graph_events = events
        .checked_add(1)
        .ok_or_else(|| String::from("a graph limit above the lineage"))?;
    assert!(generation < u64::MAX);
    assert!(graph_events > events);
    Ok([
        Bound {
            limits: crate::suite::configured::campaign::root::Limits::standard(
                crate::suite::constants::ROOT_GENERATION,
                events,
            ),
            exhausted: campaign_choregraph::AdapterError::ProjectionCapacityExhausted,
        },
        Bound {
            limits: crate::suite::configured::campaign::root::Limits::standard(
                generation,
                crate::suite::constants::MAX_EVENTS,
            ),
            exhausted: campaign_choregraph::AdapterError::GenerationCapacityExhausted,
        },
        Bound {
            limits: crate::suite::configured::campaign::root::Limits {
                graph_events,
                ..crate::suite::configured::campaign::root::Limits::standard(
                    crate::suite::constants::ROOT_GENERATION,
                    crate::suite::constants::MAX_EVENTS,
                )
            },
            exhausted: campaign_choregraph::AdapterError::GraphCapacityExhausted,
        },
    ])
}
