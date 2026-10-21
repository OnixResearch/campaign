//! Selection publication events carrying the policy inputs of the request that planned them, and
//! the execution gate compared with Campaign's own gate answer.

/// Expectation of every gate question Campaign grants.
pub(super) const GRANTED_PUBLICATION: &str = "the gate grants a durably published selection";

impl crate::harness::Session<'_, '_> {
    /// The control event that publishes one selection with the profile and ticket of the request
    /// the session built for it.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when the session never built a request naming the selection's profile
    /// and ticket.
    pub(crate) fn selection_event(
        &self,
        selection: &campaign_core::SelectionDecision,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
        let (profile, entropy) = self
            .policy_inputs
            .iter()
            .rev()
            .find(|(profile, entropy)| {
                profile.profile_id == selection.profile_id
                    && entropy.as_ref().map(|ticket| ticket.entropy_id) == selection.entropy_id
            })
            .cloned()
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "the session built the request of the published selection",
            })?;
        Ok(campaign_core::CampaignControlEvent::SelectionPublished(
            campaign_core::SelectionEvent::new(selection.clone(), profile, entropy)?,
        ))
    }

    /// Asks the adapter's execution gate, records its answer, and requires it to equal Campaign's
    /// own gate answer for the same selection, publication, and evidence.
    ///
    /// # Errors
    ///
    /// Returns `ForbiddenAccepted` naming `forbidden` when the adapter grants what Campaign
    /// refuses, and `Diverged` when it refuses what Campaign grants.
    pub(crate) fn gate(
        &mut self,
        selection: &campaign_core::SelectionDecision,
        publication: Option<&campaign_choregraph::ControlPublication>,
        evidence: Option<&campaign_choregraph::DurablePublicationEvidence>,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<crate::port::ExecutionAuthority> {
        let authority = self
            .harness
            .adapter
            .execution_gate(&crate::port::ExecutionGateRequest {
                selection,
                publication,
                evidence,
            });
        let label = if authority == crate::port::ExecutionAuthority::Granted {
            "granted"
        } else {
            "refused"
        };
        self.record(crate::ledger::TranscriptEntry::labelled(
            crate::ledger::kinds::GATED,
            label,
        ));
        let campaign = crate::harness::oracle::campaign_gate(selection, publication, evidence);
        match (authority, campaign) {
            (shell, campaign) if shell == campaign => Ok(authority),
            (crate::port::ExecutionAuthority::Granted, _) => {
                Err(crate::verdicts::ConformanceError::ForbiddenAccepted { forbidden })
            }
            (crate::port::ExecutionAuthority::Refused, _) => {
                Err(crate::verdicts::ConformanceError::Diverged {
                    expected: GRANTED_PUBLICATION,
                })
            }
        }
    }
}
