//! Record mapping and shell delegation of the Animus-shaped adapter.

/// Converts suite levels to ranks unchanged.
fn ranks(score: &crate::vocabulary::AbstractScore) -> alloc::vec::Vec<i64> {
    score.levels.iter().map(|level| i64::from(*level)).collect()
}

fn guidance_record(
    preference: &crate::vocabulary::ScenarioPreference,
    campaign_id: campaign_core::Id,
    candidate_id: campaign_core::Id,
) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
    let identities = super::identities()?;
    Ok(campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id,
        candidate_id,
        generation: preference.generation,
        ranks: ranks(&preference.score),
        source_facts_id: super::derived(
            campaign_core::IdKind::SourceFacts,
            "ranking-facts",
            |writer| writer.id(candidate_id).u64(preference.generation),
        )?,
        ordering_profile_id: identities.guidance_profile_id,
        adapter_id: identities.adapter_id,
    }
    .derive()?)
}

impl crate::port::RecordMapping for super::OpaqueCandidateAdapter {
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
                "candidate-edit",
                |writer| writer.u32(operation.handle.0).id(context.parent_moment_id),
            )?,
            operation_schema_id: identities
                .operation_schema_ids
                .first()
                .copied()
                .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?,
            source_adapter_id: identities.adapter_id,
            depth: context.depth,
            cost_profile_id: super::named(campaign_core::IdKind::CostProfile, "validation-runs")?,
            declared_costs: crate::fixtures::shell::declared_costs(
                operation,
                &identities.resource_dimension_ids,
            )?,
            operation_generation_profile_id: super::named(
                campaign_core::IdKind::Policy,
                "candidate-generation",
            )?,
        })
    }

    fn guidance(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::GuidanceContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord> {
        guidance_record(preference, context.campaign_id, context.candidate_id)
    }

    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &crate::port::RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest> {
        Ok(campaign_core::GuidanceRevisionRequest {
            candidate_id: context.candidate_id,
            successor: guidance_record(preference, context.campaign_id, context.candidate_id)?,
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
        let finding_count = usize::try_from(outcome.findings)
            .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
        let mut findings = alloc::vec::Vec::with_capacity(finding_count);
        for index in 0..outcome.findings {
            findings.push(campaign_core::FindingRef {
                finding_id: super::derived(campaign_core::IdKind::Finding, "finding", |writer| {
                    writer.id(selection.selection_id).u32(index)
                })?,
                priority_facts_id: super::named(
                    campaign_core::IdKind::SourceFacts,
                    "finding-facts",
                )?,
            });
        }
        Ok(campaign_core::ResultObservation {
            result_id: campaign_core::Id::absent(),
            campaign_id: context.campaign_id,
            selection_id: selection.selection_id,
            parent_moment_id: selection.parent_moment_id,
            operation_id: selection.operation_id,
            adapter_id: super::identities()?.adapter_id,
            payload_id: super::derived(
                campaign_core::IdKind::EventPayload,
                "execution-record",
                |writer| writer.id(selection.selection_id),
            )?,
            children: context.children.to_vec(),
            findings,
            derived_records: alloc::vec![super::OpaqueCandidateAdapter::validator_reference(
                selection.selection_id
            )?],
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

impl crate::port::ConsumerShell for super::OpaqueCandidateAdapter {
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
