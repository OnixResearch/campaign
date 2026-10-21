//! Canonical re-derivation of a supplied control event, used to authenticate its content.

mod settlement;
mod work;

impl super::CampaignControlEvent {
    /// Returns this event's canonical re-derivation from its own content.
    ///
    /// Every content-derived identity the event carries is re-derived (campaign admission;
    /// candidates, admission policy and outcome list; guidance record, revision
    /// invalidation and outcome; selection decision, profile and ticket; result and child
    /// reservation and discovery identities; closure; retirement invalidation; prune plan and
    /// invalidation; stop), every canonically ordered list is in canonical order (admitted
    /// candidates, outcomes, result children, findings and derived records, prune targets,
    /// budget-stop dimensions), and the event identity is derived last.
    ///
    /// A supplied event authenticates its content only when it equals its canonical form, as
    /// replay requires.
    ///
    /// # Errors
    ///
    /// Returns the typed identity error of a malformed carried identity, `MalformedIdentity` for
    /// duplicate prune targets, and `CrossedIdentity` when a selection event carries a profile or
    /// ticket its decision does not name.
    pub fn canonical(&self) -> crate::error::CampaignResult<Self> {
        match self {
            Self::CampaignAdmitted { admission, .. } => {
                let admission = admission.clone().derive()?;
                Ok(Self::CampaignAdmitted {
                    event_id: crate::model::derive_campaign_event_id(&admission)?,
                    admission,
                })
            }
            Self::CandidatesAdmitted(batch) => {
                crate::model::events::content::work::candidates(batch).map(Self::CandidatesAdmitted)
            }
            Self::GuidanceAdmitted(event) => {
                crate::model::events::content::work::guidance(event).map(Self::GuidanceAdmitted)
            }
            Self::SelectionPublished(event) => {
                crate::model::events::content::work::selection(event).map(Self::SelectionPublished)
            }
            Self::ResultObserved(event) => {
                crate::model::events::content::settlement::result(event).map(Self::ResultObserved)
            }
            Self::UnobservedClosed(event) => {
                crate::model::events::content::settlement::closure(event)
                    .map(Self::UnobservedClosed)
            }
            Self::CandidateRetired(event) => {
                crate::model::events::content::settlement::retirement(event)
                    .map(Self::CandidateRetired)
            }
            Self::CandidatesPruned(event) => {
                crate::model::events::content::settlement::prune(event).map(Self::CandidatesPruned)
            }
            Self::StopAccepted(event) => {
                crate::model::events::content::settlement::stop(event).map(Self::StopAccepted)
            }
        }
    }

    /// Re-derives this event's identity from its own content: the event identity of its
    /// canonical re-derivation.
    ///
    /// An event whose supplied `event_id` differs from this identity does not authenticate its
    /// content.
    ///
    /// # Errors
    ///
    /// Returns every typed error of [`Self::canonical`].
    pub fn expected_event_id(&self) -> crate::error::CampaignResult<crate::identity::Id> {
        Ok(self.canonical()?.event_id())
    }
}
