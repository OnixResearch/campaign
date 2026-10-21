//! `campaign.configuration.valid` runtime consumption: loaded values reach core identities.

fn projection() -> Result<campaign_core::ProjectionBinding, String> {
    campaign_core::ProjectionBinding {
        projection_id: campaign_core::Id::absent(),
        graph_id: super::id(campaign_core::IdKind::Graph, b"graph"),
        control_branch_id: super::id(campaign_core::IdKind::ControlBranch, b"branch"),
        control_branch_name_id: super::id(campaign_core::IdKind::ControlBranch, b"branch-name"),
        generation: 0,
        head_id: super::id(campaign_core::IdKind::Head, b"head"),
        projection_schema_id: super::id(campaign_core::IdKind::Projection, b"projection-schema"),
        adapter_id: super::id(campaign_core::IdKind::Adapter, b"adapter"),
        control_domain_id: super::id(campaign_core::IdKind::ControlDomain, b"control-domain"),
    }
    .derive()
    .map_err(|error| format!("{error:?}"))
}

/// Admits a campaign whose only profile-dependent input is `bounds`.
pub(super) fn admitted(
    bounds: campaign_core::ExplorationBounds,
) -> Result<campaign_core::CampaignAdmission, String> {
    let projection = projection()?;
    assert_eq!(
        projection.projection_id.kind,
        campaign_core::IdKind::Projection
    );
    assert!(!bounds.bounds_id.is_absent());
    campaign_core::CampaignAdmission {
        campaign_id: campaign_core::Id::absent(),
        schema_version: campaign_core::CAMPAIGN_SCHEMA_VERSION,
        projection: projection.clone(),
        policy_id: super::id(campaign_core::IdKind::Policy, b"policy"),
        guidance_profile_id: super::id(campaign_core::IdKind::GuidanceProfile, b"guidance"),
        entropy_profile_id: super::id(campaign_core::IdKind::EntropyProfile, b"entropy"),
        budget_profile_id: super::id(campaign_core::IdKind::BudgetProfile, b"budget"),
        adapter_id: projection.adapter_id,
        control_domain_id: projection.control_domain_id,
        bounds,
        resource_budgets: vec![campaign_core::ResourceAmount {
            dimension_id: super::id(campaign_core::IdKind::ResourceDimension, b"resource"),
            amount: 1,
        }],
        roots: vec![campaign_core::RootMoment {
            moment_id: super::id(campaign_core::IdKind::RootMoment, b"root"),
            projection_id: projection.projection_id,
            adapter_id: projection.adapter_id,
        }],
        operation_schema_ids: vec![super::id(
            campaign_core::IdKind::OperationSchema,
            b"operation-schema",
        )],
    }
    .derive()
    .map_err(|error| format!("{error:?}"))
}

pub(super) fn runtime(entropy_source_id: Option<campaign_core::Id>) -> crate::SelectionRuntime {
    crate::SelectionRuntime {
        production_policy_id: super::id(campaign_core::IdKind::Policy, b"policy"),
        scheduling_policy_id: super::id(campaign_core::IdKind::SchedulingProfile, b"schedule"),
        coalescing_policy_id: super::id(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
        entropy_source_id,
    }
}

fn entropy_source() -> campaign_core::Id {
    super::id(campaign_core::IdKind::EntropyProfile, b"entropy-source")
}

#[test]
// r[verify campaign.configuration.valid]
fn a_bound_change_changes_the_bounds_and_campaign_identity() -> super::TestResult {
    let default = crate::default_profile().map_err(|error| error.to_string())?;
    let reloaded = super::load(&super::default_document()?).map_err(|error| error.to_string())?;
    let raised_depth = default
        .bounds
        .max_depth
        .checked_add(1)
        .ok_or("maximum depth overflow")?;
    let changed = super::load(&super::with(
        "/bounds/maximum_depth",
        serde_json::json!(raised_depth),
    )?)
    .map_err(|error| error.to_string())?;
    assert_eq!(reloaded.bounds.bounds_id, default.bounds.bounds_id);
    assert_eq!(changed.bounds.max_depth, raised_depth);
    assert_ne!(changed.bounds.bounds_id, default.bounds.bounds_id);

    let default_campaign = admitted(default.bounds)?;
    let reloaded_campaign = admitted(reloaded.bounds)?;
    let changed_campaign = admitted(changed.bounds)?;
    assert_eq!(default_campaign.validate(), Ok(()));
    assert_eq!(reloaded_campaign.campaign_id, default_campaign.campaign_id);
    assert_ne!(changed_campaign.campaign_id, default_campaign.campaign_id);
    assert_eq!(
        changed_campaign.campaign_id.kind,
        campaign_core::IdKind::Campaign
    );
    Ok(())
}

#[test]
fn the_selection_profile_binds_declared_values_and_runtime_identities() -> super::TestResult {
    let profile = crate::default_profile().map_err(|error| error.to_string())?;
    let built = profile
        .selection_profile(runtime(Some(entropy_source())))
        .map_err(|error| error.to_string())?;
    let expected = campaign_core::SelectionProfile {
        profile_id: campaign_core::Id::absent(),
        scheduling_policy_id: super::id(campaign_core::IdKind::SchedulingProfile, b"schedule"),
        admission_policy_id: profile
            .admission
            .admission_policy_id()
            .map_err(|error| format!("{error:?}"))?,
        coalescing_policy_id: super::id(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
        mode: campaign_core::SelectionMode::Exploratory {
            threshold_parts_per_million: 100_000,
            entropy_source_id: entropy_source(),
            exploration_order: vec![0],
        },
        decay: campaign_core::SelectionCountDecay::None,
        stopping: campaign_core::StoppingRule {
            finding_limit: None,
        },
        implementation: campaign_core::PolicyImplementation::ProductionCompiled {
            policy_id: super::id(campaign_core::IdKind::Policy, b"policy"),
        },
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(built, expected);
    assert_eq!(built.validate(), Ok(()));

    // A declared selection value is part of the derived profile identity.
    let raised = super::load(&super::with(
        "/selection/exploratory_threshold_parts_per_million",
        serde_json::json!(100_001),
    )?)
    .map_err(|error| error.to_string())?
    .selection_profile(runtime(Some(entropy_source())))
    .map_err(|error| error.to_string())?;
    assert_ne!(raised.profile_id, built.profile_id);
    Ok(())
}

#[test]
fn the_selection_profile_rejects_mismatched_runtime_identities() -> super::TestResult {
    let exploratory = crate::default_profile().map_err(|error| error.to_string())?;
    assert_eq!(
        exploratory.selection_profile(runtime(None)),
        Err(crate::ProfileError::MissingEntropySource)
    );
    let mut ranked_document = super::default_document()?;
    *ranked_document
        .pointer_mut("/selection")
        .ok_or("default selection is absent")? = serde_json::json!({
        "mode": "ranked",
        "decay": { "rule": "none" },
        "tie_break": "candidate_identity",
    });
    let ranked = super::load(&ranked_document).map_err(|error| error.to_string())?;
    assert_eq!(
        ranked.selection_profile(runtime(Some(entropy_source()))),
        Err(crate::ProfileError::UnexpectedEntropySource)
    );
    let ranked_profile = ranked
        .selection_profile(runtime(None))
        .map_err(|error| error.to_string())?;
    assert_eq!(ranked_profile.mode, campaign_core::SelectionMode::Ranked);

    let mut crossed = runtime(Some(entropy_source()));
    crossed.scheduling_policy_id = super::id(campaign_core::IdKind::Policy, b"schedule");
    assert_eq!(
        exploratory.selection_profile(crossed),
        Err(crate::ProfileError::Campaign(
            campaign_core::CampaignError::CrossedIdentity
        ))
    );
    let mut absent = runtime(Some(entropy_source()));
    absent.production_policy_id = campaign_core::Id::absent();
    assert_eq!(
        exploratory.selection_profile(absent),
        Err(crate::ProfileError::Campaign(
            campaign_core::CampaignError::MissingIdentity
        ))
    );
    let wrong_entropy = runtime(Some(super::id(campaign_core::IdKind::Policy, b"entropy")));
    assert_eq!(
        exploratory.selection_profile(wrong_entropy),
        Err(crate::ProfileError::Campaign(
            campaign_core::CampaignError::CrossedIdentity
        ))
    );
    Ok(())
}
