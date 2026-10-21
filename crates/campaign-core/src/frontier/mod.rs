mod admission;
mod dispatch;
mod encoding;
mod replay;
pub(crate) mod reserve;
mod retention;
pub(crate) mod settlement;
pub(crate) mod validation;
pub use replay::{apply_control_event, reconstruct};
/// Depth of every admitted root moment.
pub const ROOT_DEPTH: u64 = 0;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// One moment known to the frontier with its depth below a root.
pub struct KnownMoment {
    /// Root or child moment identity.
    pub moment_id: crate::identity::Id,
    /// Roots have depth zero; a child has the depth of the candidate whose selection produced it.
    pub depth: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// One settled selection and the result or closure identity that settled it.
pub struct Settlement {
    /// Settled selection identity.
    pub selection_id: crate::identity::Id,
    /// Accepted result identity or unobserved-closure identity.
    pub settlement_id: crate::identity::Id,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// One accepted exploration payload and the single child moment or result it belongs to.
pub struct PayloadBinding {
    /// `EventPayload` identity of a child exploration event or of a result.
    pub payload_id: crate::identity::Id,
    /// Child moment or result identity the payload is bound to.
    pub owner_id: crate::identity::Id,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.fd.identity]
pub struct StableDiscovery {
    /// Public Campaign core API value.
    pub child_moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub discovery_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub event_payload_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub first_result_id: crate::identity::Id,
}
#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.frontier]
pub struct FrontierState {
    /// Public Campaign core API value.
    pub admission: crate::model::CampaignAdmission,
    /// Public Campaign core API value.
    pub candidates: alloc::vec::Vec<crate::model::CandidateState>,
    /// Public Campaign core API value.
    pub outstanding: alloc::vec::Vec<crate::model::OutstandingSelection>,
    /// Public Campaign core API value.
    pub accepted_results: alloc::vec::Vec<crate::identity::Id>,
    /// Public Campaign core API value.
    pub discoveries: alloc::vec::Vec<crate::frontier::StableDiscovery>,
    /// Public Campaign core API value.
    pub known_moments: alloc::vec::Vec<crate::frontier::KnownMoment>,
    /// Settled selections in canonical selection order.
    pub settled: alloc::vec::Vec<crate::frontier::Settlement>,
    /// Child reservation identities bound by accepted results.
    pub reservations: alloc::vec::Vec<crate::identity::Id>,
    /// Every accepted child and result payload with the one owner it is bound to.
    pub payload_bindings: alloc::vec::Vec<crate::frontier::PayloadBinding>,
    /// Public Campaign core API value.
    pub findings: alloc::vec::Vec<crate::model::FindingRef>,
    /// Public Campaign core API value.
    pub stopped: Option<crate::model::StopDecision>,
    /// Public Campaign core API value.
    pub event_ids: alloc::vec::Vec<crate::identity::Id>,
    /// Public Campaign core API value.
    pub reserved_resources: alloc::vec::Vec<crate::model::ResourceAmount>,
    /// Public Campaign core API value.
    pub selection_count: u64,
    /// Public Campaign core API value.
    pub guidance_record_count: u64,
    /// Public Campaign core API value.
    pub control_event_count: u64,
    /// Public Campaign core API value.
    pub state_id: crate::identity::Id,
}
impl crate::frontier::FrontierState {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[cfg_attr(
        dylint_lib = "octet",
        allow(
            tigerstyle::mutating_input_in_pure,
            reason = "POLICY owns this stable mutable API wrapper; identity computation consumes private cloned state and publishes only a complete result"
        )
    )]
    pub fn recompute_identity(&mut self) -> crate::error::CampaignResult<()> {
        *self = self.clone().with_identity()?;
        Ok(())
    }

    pub(crate) fn with_identity(self) -> crate::error::CampaignResult<Self> {
        let mut state = self.sorted();
        state.state_id = crate::frontier::encoding::derive_state_id(&state)?;
        debug_assert_eq!(state.state_id.kind, crate::identity::IdKind::State);
        Ok(state)
    }
    /// Returns the one stable discovery of a child moment, shared by every path that found it.
    ///
    /// This and the other keyed lookups (`payload_owner`, `candidate`, `known_moment_depth`) binary
    /// search the canonical (sorted) order every core state carries; after editing public fields,
    /// call `sort_all` first.
    #[must_use]
    // r[impl campaign.fd.identity]
    pub fn discovery(
        &self,
        child_moment_id: crate::identity::Id,
    ) -> Option<&crate::frontier::StableDiscovery> {
        crate::identity::collection::sorted_position(
            &self.discoveries,
            child_moment_id,
            |discovery| discovery.child_moment_id,
        )
        .map(|index| &self.discoveries[index])
    }
    /// Returns the child moment or result an accepted payload is bound to.
    #[must_use]
    pub fn payload_owner(&self, payload_id: crate::identity::Id) -> Option<crate::identity::Id> {
        crate::identity::collection::sorted_position(
            &self.payload_bindings,
            payload_id,
            |binding| binding.payload_id,
        )
        .map(|index| self.payload_bindings[index].owner_id)
    }
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn candidate(
        &self,
        candidate_id: crate::identity::Id,
    ) -> crate::error::CampaignResult<&crate::model::CandidateState> {
        crate::identity::collection::sorted_position(&self.candidates, candidate_id, |candidate| {
            candidate.candidate.candidate_id
        })
        .map(|index| &self.candidates[index])
        .ok_or(crate::error::CampaignError::MissingCandidate)
    }
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[must_use]
    pub fn has_known_moment(&self, moment_id: crate::identity::Id) -> bool {
        self.known_moment_depth(moment_id).is_some()
    }
    /// Returns the depth of a known root or child moment.
    #[must_use]
    pub fn known_moment_depth(&self, moment_id: crate::identity::Id) -> Option<u64> {
        crate::identity::collection::sorted_position(&self.known_moments, moment_id, |moment| {
            moment.moment_id
        })
        .map(|index| self.known_moments[index].depth)
    }
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[must_use]
    pub fn has_outstanding_for_candidate(&self, candidate_id: crate::identity::Id) -> bool {
        self.outstanding
            .iter()
            .any(|selection| selection.candidate_id == candidate_id)
    }
    /// Candidate identities of every outstanding selection in canonical order (a candidate with
    /// several outstanding selections repeats), built once so a loop over candidates tests each
    /// for outstanding work by binary search instead of scanning `outstanding`.
    #[must_use]
    pub(crate) fn outstanding_candidate_ids(&self) -> alloc::vec::Vec<crate::identity::Id> {
        let mut candidate_ids: alloc::vec::Vec<crate::identity::Id> = self
            .outstanding
            .iter()
            .map(|selection| selection.candidate_id)
            .collect();
        candidate_ids.sort_unstable();
        debug_assert_eq!(candidate_ids.len(), self.outstanding.len());
        candidate_ids
    }
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[cfg_attr(
        dylint_lib = "octet",
        allow(
            tigerstyle::mutating_input_in_pure,
            reason = "POLICY owns this stable mutable API wrapper; canonical sorting consumes private cloned state before replacement"
        )
    )]
    pub fn sort_all(&mut self) {
        *self = self.clone().sorted();
    }

    fn sorted(mut self) -> Self {
        self.admission.roots.sort_by_key(|root| root.moment_id);
        self.admission
            .resource_budgets
            .sort_by_key(|budget| budget.dimension_id);
        self.admission.operation_schema_ids.sort();
        self.candidates
            .sort_by_key(|candidate| candidate.candidate.candidate_id);
        self.outstanding
            .sort_by_key(|selection| selection.selection_id);
        self.accepted_results.sort();
        self.discoveries
            .sort_by_key(|discovery| discovery.child_moment_id);
        self.known_moments.sort_by_key(|moment| moment.moment_id);
        self.settled
            .sort_by_key(|settlement| settlement.selection_id);
        self.reservations.sort();
        self.payload_bindings
            .sort_by_key(|binding| binding.payload_id);
        self.findings.sort_by_key(|finding| finding.finding_id);
        self.event_ids.sort();
        self.reserved_resources
            .sort_by_key(|resource| resource.dimension_id);
        debug_assert!(self.event_ids.is_sorted());
        debug_assert!(
            self.known_moments
                .is_sorted_by_key(|moment| moment.moment_id)
        );
        debug_assert!(self.reservations.is_sorted());
        self
    }
}
