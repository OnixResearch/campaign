//! The three host bounds on one campaign's control lineage and the events it still needs.

/// Size of a control history after one publication: its control lineage (projection) and the
/// Choregraph graph that holds it together with every other writer's events.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Extent {
    /// Control-lineage events.
    pub(crate) lineage: usize,
    /// Choregraph graph events, control and other events alike.
    pub(crate) graph: u32,
}

/// Events a control history still needs: its extent, one settlement per outstanding selection,
/// and the stop of an open campaign.
#[derive(Clone, Copy, Debug)]
pub(super) struct Demand {
    pub(super) extent: Extent,
    pub(super) outstanding: usize,
    pub(super) open: bool,
}

impl Demand {
    /// The settlements and the stop still to be published, saturating at `usize::MAX` (a demand
    /// no bound admits).
    pub(super) fn reserve(self) -> usize {
        let reserve = self.outstanding.saturating_add(usize::from(self.open));
        debug_assert!(reserve >= usize::from(self.open));
        reserve
    }

    /// Control-lineage events including the reserve.
    fn lineage_events(self, reserve: u64) -> crate::contracts::AdapterResult<u64> {
        u64::try_from(self.extent.lineage)
            .ok()
            .and_then(|lineage| lineage.checked_add(reserve))
            .ok_or_else(|| campaign_core::CampaignError::ArithmeticOverflow.into())
    }

    /// Choregraph graph events including the reserve.
    fn graph_events(self, reserve: u64) -> crate::contracts::AdapterResult<u64> {
        u64::from(self.extent.graph)
            .checked_add(reserve)
            .ok_or_else(|| campaign_core::CampaignError::ArithmeticOverflow.into())
    }
}

/// The host bounds on one campaign's control lineage: the projection bound, the generation of the
/// admission's binding (from which the branch advances one generation per event), and the
/// Choregraph graph's own event limit.
#[derive(Clone, Copy, Debug)]
pub(super) struct HostBounds {
    /// The host projection bound (`maximum_events`).
    pub(super) maximum_events: u32,
    /// Generation of the admission's projection binding.
    pub(super) admitted_generation: u64,
    /// The graph's own event limit (`HistoryLimits::max_events`).
    pub(super) graph_max_events: u32,
}

impl HostBounds {
    /// Requires `demand` to fit the projection bound, the branch generations left, and the graph
    /// event limit, in that order.
    // r[impl campaign.publication_fence.capacity]
    pub(super) fn check(self, demand: Demand) -> crate::contracts::AdapterResult<()> {
        let reserve = u64::try_from(demand.reserve())
            .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
        let lineage = demand.lineage_events(reserve)?;
        if lineage > u64::from(self.maximum_events) {
            return Err(crate::contracts::AdapterError::ProjectionCapacityExhausted);
        }
        if self.admitted_generation.checked_add(lineage).is_none() {
            return Err(crate::contracts::AdapterError::GenerationCapacityExhausted);
        }
        if demand.graph_events(reserve)? > u64::from(self.graph_max_events) {
            return Err(crate::contracts::AdapterError::GraphCapacityExhausted);
        }
        Ok(())
    }

    /// Whether `demand` fits; a capacity refusal answers false, any other error is returned.
    pub(super) fn fits(self, demand: Demand) -> crate::contracts::AdapterResult<bool> {
        match self.check(demand) {
            Ok(()) => Ok(true),
            Err(
                crate::contracts::AdapterError::ProjectionCapacityExhausted
                | crate::contracts::AdapterError::GenerationCapacityExhausted
                | crate::contracts::AdapterError::GraphCapacityExhausted,
            ) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
