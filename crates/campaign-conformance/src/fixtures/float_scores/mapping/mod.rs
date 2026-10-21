//! Port implementation of the float-scoring adapter.

/// Consumer reason labels.
const REASON_LABELS: [(crate::vocabulary::ScenarioReason, &str); 5] = [
    (crate::vocabulary::ScenarioReason::Retirement, "retire"),
    (crate::vocabulary::ScenarioReason::Pruning, "prune"),
    (crate::vocabulary::ScenarioReason::Revision, "rescore"),
    (crate::vocabulary::ScenarioReason::Closure, "unobserved"),
    (crate::vocabulary::ScenarioReason::ConsumerStop, "halt"),
];

/// Choregraph parent role of a scored step.
const STEP_PARENT_ROLE: &str = "step-parent";

// r[impl campaign.guidance.float]
fn scored(
    preference: &crate::vocabulary::ScenarioPreference,
    campaign_id: campaign_core::Id,
    candidate_id: campaign_core::Id,
) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
    let identities = super::identities()?;
    let mut ranks = alloc::vec::Vec::with_capacity(preference.score.levels.len());
    for score in preference.score.float_view() {
        ranks.push(super::fixed_point_rank(score)?);
    }
    Ok(campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id,
        candidate_id,
        generation: preference.generation,
        ranks,
        source_facts_id: super::derived(campaign_core::IdKind::SourceFacts, "scores", |writer| {
            writer.id(candidate_id).u64(preference.generation)
        })?,
        ordering_profile_id: identities.guidance_profile_id,
        adapter_id: identities.adapter_id,
    })
}

/// One opaque finding reference per reported finding of a step.
fn step_findings(
    outcome: &crate::vocabulary::ScenarioOutcome,
    selection_id: campaign_core::Id,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::FindingRef>> {
    let count = usize::try_from(outcome.findings)
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let mut findings = alloc::vec::Vec::with_capacity(count);
    for index in 0..outcome.findings {
        findings.push(campaign_core::FindingRef {
            finding_id: super::derived(campaign_core::IdKind::Finding, "step-finding", |writer| {
                writer.id(selection_id).u32(index)
            })?,
            priority_facts_id: super::derived(
                campaign_core::IdKind::SourceFacts,
                "step-finding-facts",
                |writer| writer.id(selection_id).u32(index),
            )?,
        });
    }
    debug_assert_eq!(findings.len(), count);
    Ok(findings)
}

impl crate::port::MomentMapping for super::FixedPointAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        super::identities()
    }

    fn exploration_draft(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        parent_event: Option<campaign_choregraph::EventId>,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::EventDraft> {
        let parents = parent_event.map_or_else(alloc::vec::Vec::new, |event| {
            alloc::vec![campaign_choregraph::ParentLink {
                role: alloc::string::String::from(STEP_PARENT_ROLE),
                event,
            }]
        });
        let payload = super::derived(campaign_core::IdKind::EventPayload, "state", |writer| {
            writer.u32(moment.handle.0)
        })?;
        Ok(campaign_choregraph::EventDraft::v1(
            campaign_choregraph::EventClass::Application,
            campaign_choregraph::digest_from_id(payload),
            parents,
            campaign_choregraph::Digest::derive(super::DOMAIN, b"scored-state"),
        ))
    }

    fn moment_identity(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        event: campaign_choregraph::EventId,
    ) -> crate::verdicts::ConformanceResult<crate::port::MomentIdentity> {
        if moment.parent.is_none() {
            return Ok(crate::port::MomentIdentity::Root {
                moment_id: campaign_choregraph::id_from_digest(
                    campaign_core::IdKind::RootMoment,
                    event.digest(),
                ),
            });
        }
        Ok(crate::port::MomentIdentity::Child {
            moment_id: campaign_choregraph::id_from_digest(
                campaign_core::IdKind::ChildMoment,
                event.digest(),
            ),
            event_payload_id: super::derived(
                campaign_core::IdKind::EventPayload,
                "state",
                |writer| writer.u32(moment.handle.0),
            )?,
        })
    }

    fn reason(
        &mut self,
        reason: crate::vocabulary::ScenarioReason,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        let label = REASON_LABELS
            .iter()
            .find(|(known, _)| *known == reason)
            .map(|(_, label)| *label)
            .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?;
        super::named(campaign_core::IdKind::Reason, label)
    }
}

impl crate::port::RecordMapping for super::FixedPointAdapter {
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &crate::port::CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord> {
        let identities = super::identities()?;
        Ok(campaign_core::CandidateRecord {
            candidate_id: campaign_core::Id::absent(),
            campaign_id: context.campaign_id,
            parent_moment_id: context.parent_moment_id,
            operation_profile_id: super::derived(
                campaign_core::IdKind::OperationProfile,
                "step",
                |writer| writer.u32(operation.handle.0),
            )?,
            operation_schema_id: identities
                .operation_schema_ids
                .first()
                .copied()
                .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?,
            source_adapter_id: identities.adapter_id,
            depth: context.depth,
            cost_profile_id: super::named(campaign_core::IdKind::CostProfile, "steps")?,
            declared_costs: crate::fixtures::shell::declared_costs(
                operation,
                &identities.resource_dimension_ids,
            )?,
            operation_generation_profile_id: super::named(
                campaign_core::IdKind::Policy,
                "stepper",
            )?,
        })
    }

    fn guidance(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::GuidanceContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
        scored(preference, context.campaign_id, context.candidate_id)
    }

    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        Ok(campaign_core::GuidanceRevisionRequest {
            candidate_id: context.candidate_id,
            successor: scored(preference, context.campaign_id, context.candidate_id)?,
            invalidated_guidance_id: context.invalidated_guidance_id,
            reason_id: crate::port::MomentMapping::reason(
                self,
                crate::vocabulary::ScenarioReason::Revision,
            )?,
        })
    }

    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &crate::port::EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        crate::fixtures::shell::entropy_ticket(
            super::identities()?.entropy_source_id,
            draw,
            context,
        )
    }

    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &crate::port::ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation> {
        let selection = context.selection;
        let findings = step_findings(outcome, selection.selection_id)?;
        Ok(campaign_core::ResultObservation {
            result_id: campaign_core::Id::absent(),
            campaign_id: context.campaign_id,
            selection_id: selection.selection_id,
            parent_moment_id: selection.parent_moment_id,
            operation_id: selection.operation_id,
            adapter_id: super::identities()?.adapter_id,
            payload_id: super::derived(
                campaign_core::IdKind::EventPayload,
                "step-result",
                |writer| writer.id(selection.selection_id),
            )?,
            children: context.children.to_vec(),
            findings,
            derived_records: alloc::vec::Vec::new(),
        })
    }

    fn closure(
        &mut self,
        context: &crate::port::ClosureContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure> {
        let reason_id =
            crate::port::MomentMapping::reason(self, crate::vocabulary::ScenarioReason::Closure)?;
        crate::fixtures::shell::closure(context, reason_id)
    }
}

impl crate::port::ConsumerShell for super::FixedPointAdapter {
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
        crate::fixtures::shell::load_history(request, previous)
    }

    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState> {
        crate::fixtures::shell::validate_cache(request, cache)
    }

    fn execution_gate(
        &mut self,
        request: &crate::port::ExecutionGateRequest<'_>,
    ) -> crate::port::ExecutionAuthority {
        crate::fixtures::shell::execution_gate(request)
    }

    fn promotion_claims(
        &mut self,
        _receipt: &campaign_core::CampaignReceipt,
        _signals: &crate::port::AuthoritySignals,
    ) -> campaign_core::ReceiptClaims {
        crate::fixtures::shell::promotion_claims()
    }

    fn role_identities(
        &mut self,
        candidate: &campaign_core::CandidateRecord,
        work_item: &campaign_core::SelectionDecision,
        result: &campaign_core::ResultObservation,
    ) -> campaign_core::RoleCollapseFixture {
        crate::fixtures::shell::role_identities(candidate, work_item, result)
    }
}
