//! Candidate batches, guidance, and explicit revisions mapped through the adapter.

/// Suite coalescing policy name.
const COALESCING_POLICY: &str = "coalescing-policy";

/// Suite coalescing policy identity.
///
/// # Errors
///
/// Returns a typed Campaign error when framing fails.
pub(crate) fn coalescing_policy_id() -> crate::verdicts::ConformanceResult<campaign_core::Id> {
    super::suite_id(campaign_core::IdKind::CoalescingPolicy, COALESCING_POLICY)
}

impl super::Session<'_, '_> {
    /// Maps abstract operations to derived candidate records and remembers their handles.
    ///
    /// # Errors
    ///
    /// Returns the adapter or Campaign error of one mapping.
    pub(crate) fn candidates(
        &mut self,
        operations: &[crate::vocabulary::ScenarioOperation],
    ) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::CandidateRecord>> {
        let mut records = alloc::vec::Vec::with_capacity(operations.len());
        for operation in operations {
            let parent = self.moment(operation.parent)?;
            let parent_moment_id = parent.identity.moment_id();
            let parent_depth = self.state.known_moment_depth(parent_moment_id).ok_or(
                crate::verdicts::ConformanceError::Diverged {
                    expected: "operation parent is a known moment",
                },
            )?;
            let context = crate::port::CandidateContext {
                campaign_id: self.state.admission.campaign_id,
                parent_moment_id,
                depth: parent_depth
                    .checked_add(1)
                    .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
            };
            let record = self
                .harness
                .adapter
                .candidate(operation, &context)?
                .derive()?;
            if self.candidate_id(operation.handle).is_err() {
                self.operations
                    .push((operation.handle, record.candidate_id));
            }
            records.push(record);
        }
        debug_assert_eq!(records.len(), operations.len());
        Ok(records)
    }

    /// Builds the candidate batch event of the session's admission rule without publishing it.
    ///
    /// # Errors
    ///
    /// Returns the adapter or Campaign denial of the batch.
    pub(crate) fn batch(
        &mut self,
        operations: &[crate::vocabulary::ScenarioOperation],
    ) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
        let records = self.candidates(operations)?;
        Ok(campaign_core::admit_candidates(
            &self.state,
            records,
            self.rule,
            coalescing_policy_id()?,
        )?)
    }

    /// Admits and publishes one candidate batch; returns the published event.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the batch.
    pub(crate) fn admit(
        &mut self,
        operations: &[crate::vocabulary::ScenarioOperation],
    ) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
        Ok(self.admit_published(operations)?.control_event().clone())
    }

    /// Admits and publishes one candidate batch; returns its publication.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the batch.
    pub(crate) fn admit_published(
        &mut self,
        operations: &[crate::vocabulary::ScenarioOperation],
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let event = self.batch(operations)?;
        let publication = self.publish(event)?;
        let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) =
            publication.control_event()
        else {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "candidate admission publishes a candidate batch",
            });
        };
        let mut admitted = alloc::vec::Vec::with_capacity(batch.outcomes.len());
        let mut coalesced = alloc::vec::Vec::with_capacity(batch.outcomes.len());
        for outcome in &batch.outcomes {
            let handle = self.operation_of(outcome.candidate_id)?;
            match outcome.outcome {
                campaign_core::AdmissionDecision::Admitted => admitted.push(handle),
                campaign_core::AdmissionDecision::CoalescedDuplicate { .. } => {
                    coalesced.push(handle);
                }
            }
        }
        self.record(
            crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::ADMITTED, "")
                .with_operations(admitted),
        );
        if !coalesced.is_empty() {
            self.record(
                crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::COALESCED, "")
                    .with_operations(coalesced),
            );
        }
        Ok(publication)
    }

    /// Maps one abstract preference to a guidance record.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the conversion.
    pub(crate) fn guidance_record(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
        let context = crate::port::GuidanceContext {
            campaign_id: self.state.admission.campaign_id,
            candidate_id: self.candidate_id(preference.operation)?,
        };
        self.harness.adapter.guidance(preference, &context)
    }

    /// Admits and publishes one preference through relaxation-only guidance admission.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the guidance.
    pub(crate) fn guide(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
        Ok(self.guide_published(preference)?.control_event().clone())
    }

    /// Admits and publishes one preference through relaxation-only guidance admission; returns
    /// its publication.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the guidance.
    pub(crate) fn guide_published(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let record = self.guidance_record(preference)?;
        let event = campaign_core::admit_guidance(&self.state, record)?;
        self.publish_guidance(preference.operation, event)
    }

    /// Builds the explicit revision request of one preference.
    ///
    /// # Errors
    ///
    /// Returns `MissingGuidance` without a current record and the adapter error of the mapping.
    pub(crate) fn revision_request(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        let candidate_id = self.candidate_id(preference.operation)?;
        let invalidated_guidance_id = self
            .state
            .candidate(candidate_id)?
            .latest_guidance
            .as_ref()
            .map(|guidance| guidance.guidance_id)
            .ok_or(campaign_core::CampaignError::MissingGuidance)?;
        let context = crate::port::RevisionContext {
            campaign_id: self.state.admission.campaign_id,
            candidate_id,
            invalidated_guidance_id,
        };
        self.harness.adapter.revision(preference, &context)
    }

    /// Admits and publishes one explicit revision.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the revision.
    pub(crate) fn revise(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
        let request = self.revision_request(preference)?;
        let event = campaign_core::revise_guidance(&self.state, request)?;
        Ok(self
            .publish_guidance(preference.operation, event)?
            .control_event()
            .clone())
    }

    fn publish_guidance(
        &mut self,
        operation: crate::vocabulary::OperationHandle,
        event: campaign_core::CampaignControlEvent,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ControlPublication> {
        let publication = self.publish(event)?;
        let campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) =
            publication.control_event()
        else {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "guidance admission publishes a guidance event",
            });
        };
        let label = super::labels::dynamics(&guidance.dynamics);
        self.record(
            crate::ledger::TranscriptEntry::labelled(crate::ledger::kinds::GUIDED, label)
                .with_operations(alloc::vec![operation]),
        );
        Ok(publication)
    }
}
