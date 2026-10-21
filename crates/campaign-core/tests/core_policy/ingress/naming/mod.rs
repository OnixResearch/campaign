//! Campaign identity: equal, changed, and missing admission facts.

type Edit = fn(&mut campaign_core::CampaignAdmission);
type EditTable<'a> = (&'a [Edit], Edit);

fn xid(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(kind, label)
}

fn root(
    input: &campaign_core::CampaignAdmission,
    label: &'static [u8],
) -> campaign_core::RootMoment {
    campaign_core::RootMoment {
        moment_id: xid(campaign_core::IdKind::RootMoment, label),
        projection_id: input.projection.projection_id,
        adapter_id: input.projection.adapter_id,
    }
}

/// Default input widened to three roots, two budget dimensions, and two operation schemas.
fn wide_input() -> Result<campaign_core::CampaignAdmission, String> {
    let mut input = crate::suite::admission::default_input()?;
    let second = root(&input, b"root-two");
    let third = root(&input, b"root-three");
    input.roots.extend([second, third]);
    input.resource_budgets.push(campaign_core::ResourceAmount {
        dimension_id: xid(campaign_core::IdKind::ResourceDimension, b"second-resource"),
        amount: crate::suite::constants::BUDGET_TOTAL,
    });
    input.operation_schema_ids.push(xid(
        campaign_core::IdKind::OperationSchema,
        b"second-schema",
    ));
    Ok(input)
}

/// Re-derives the projection after a projection fact changed and rebinds dependent facts.
fn rebind_projection(input: &mut campaign_core::CampaignAdmission) {
    let Ok(projection) = input.projection.clone().derive() else {
        return;
    };
    input.adapter_id = projection.adapter_id;
    input.control_domain_id = projection.control_domain_id;
    for root in &mut input.roots {
        root.projection_id = projection.projection_id;
        root.adapter_id = projection.adapter_id;
    }
    input.projection = projection;
}

fn rederive_bounds(input: &mut campaign_core::CampaignAdmission) {
    if let Ok(bounds) = input.bounds.clone().derive() {
        input.bounds = bounds;
    }
}

const PROJECTION_EDITS: [Edit; 7] = [
    |input| input.projection.graph_id = xid(campaign_core::IdKind::Graph, b"other-graph"),
    |input| {
        input.projection.control_branch_id =
            xid(campaign_core::IdKind::ControlBranch, b"other-branch");
    },
    |input| input.projection.generation = crate::suite::constants::GENERATION_NEXT,
    |input| input.projection.head_id = xid(campaign_core::IdKind::Head, b"other-head"),
    |input| {
        input.projection.projection_schema_id = xid(
            campaign_core::IdKind::Projection,
            b"other-projection-schema",
        );
    },
    |input| input.projection.adapter_id = xid(campaign_core::IdKind::Adapter, b"other-adapter"),
    |input| {
        input.projection.control_domain_id =
            xid(campaign_core::IdKind::ControlDomain, b"other-domain");
    },
];

const PROFILE_EDITS: [Edit; 4] = [
    |input| input.policy_id = xid(campaign_core::IdKind::Policy, b"other-policy"),
    |input| {
        input.guidance_profile_id = xid(
            campaign_core::IdKind::GuidanceProfile,
            b"other-guidance-profile",
        );
    },
    |input| {
        input.entropy_profile_id = xid(
            campaign_core::IdKind::EntropyProfile,
            b"other-entropy-profile",
        );
    },
    |input| {
        input.budget_profile_id = xid(
            campaign_core::IdKind::BudgetProfile,
            b"other-budget-profile",
        );
    },
];

fn bump(value: &mut u64) {
    *value = value.saturating_add(1);
}

const BOUND_EDITS: [Edit; 17] = [
    |input| bump(&mut input.bounds.max_roots),
    |input| bump(&mut input.bounds.max_candidates),
    |input| bump(&mut input.bounds.max_candidate_batch),
    |input| bump(&mut input.bounds.max_retained_candidates),
    |input| bump(&mut input.bounds.max_selections),
    |input| bump(&mut input.bounds.max_guidance_records),
    |input| bump(&mut input.bounds.max_outstanding),
    |input| bump(&mut input.bounds.max_depth),
    |input| bump(&mut input.bounds.max_child_moments),
    |input| bump(&mut input.bounds.max_derived_records),
    |input| bump(&mut input.bounds.max_rank_dimensions),
    |input| bump(&mut input.bounds.max_rank_magnitude),
    |input| bump(&mut input.bounds.max_findings),
    |input| bump(&mut input.bounds.max_control_events),
    |input| bump(&mut input.bounds.max_resource_dimensions),
    |input| bump(&mut input.bounds.max_operation_schemas),
    |input| bump(&mut input.bounds.max_prune_batch),
];

const SET_EDITS: [Edit; 6] = [
    |input| bump(&mut input.resource_budgets[0].amount),
    |input| {
        input.resource_budgets[0].dimension_id =
            xid(campaign_core::IdKind::ResourceDimension, b"other-resource");
    },
    |input| input.roots[0].moment_id = xid(campaign_core::IdKind::RootMoment, b"other-root"),
    |input| {
        let extra = root(input, b"extra-root");
        input.roots.push(extra);
    },
    |input| {
        input.operation_schema_ids[0] =
            xid(campaign_core::IdKind::OperationSchema, b"other-schema");
    },
    |input| {
        input
            .operation_schema_ids
            .push(xid(campaign_core::IdKind::OperationSchema, b"extra-schema"));
    },
];

#[test]
// r[verify campaign.identity.equal]
fn equal_inputs_in_any_order_give_equal_identity_and_admission() -> Result<(), String> {
    let base = wide_input()?;
    let mut permuted = base.clone();
    permuted.roots.reverse();
    permuted.roots.rotate_left(1);
    permuted.resource_budgets.reverse();
    permuted.operation_schema_ids.reverse();
    assert_ne!(permuted.roots, base.roots);
    let left = crate::suite::ingress::support::text(campaign_core::admit_campaign(base.clone()))?;
    let right = crate::suite::ingress::support::text(campaign_core::admit_campaign(permuted))?;
    assert_eq!(left, right);
    let again = crate::suite::ingress::support::text(base.derive())?;
    let campaign_core::CampaignControlEvent::CampaignAdmitted { admission, .. } = &left else {
        return Err(format!("expected a campaign admission, got {left:?}"));
    };
    assert_eq!(admission.campaign_id, again.campaign_id);
    assert_eq!(admission.campaign_id.kind, campaign_core::IdKind::Campaign);
    Ok(())
}

#[test]
// r[verify campaign.identity.changed]
fn every_bound_admission_fact_changes_the_campaign_identity() -> Result<(), String> {
    let base = crate::suite::ingress::support::text(wide_input()?.derive())?;
    let mut seen = vec![base.campaign_id];
    let tables: [EditTable<'_>; 4] = [
        (&PROJECTION_EDITS, rebind_projection),
        (&PROFILE_EDITS, |_| {}),
        (&BOUND_EDITS, rederive_bounds),
        (&SET_EDITS, |_| {}),
    ];
    for (edits, repair) in tables {
        for edit in edits {
            let mut changed = wide_input()?;
            edit(&mut changed);
            repair(&mut changed);
            let changed = crate::suite::ingress::support::text(changed.derive())?;
            assert!(
                !seen.contains(&changed.campaign_id),
                "fact edit did not change the campaign identity: {changed:?}"
            );
            seen.push(changed.campaign_id);
        }
    }
    let total = [
        PROJECTION_EDITS.len(),
        PROFILE_EDITS.len(),
        BOUND_EDITS.len(),
        SET_EDITS.len(),
        1,
    ]
    .iter()
    .sum::<usize>();
    assert_eq!(seen.len(), total);
    Ok(())
}

#[test]
fn a_changed_schema_version_is_not_admitted() -> Result<(), String> {
    let mut input = wide_input()?;
    input.schema_version = campaign_core::CAMPAIGN_SCHEMA_VERSION.saturating_add(1);
    assert_eq!(
        campaign_core::admit_campaign(input),
        Err(campaign_core::CampaignError::UnsupportedVersion)
    );
    Ok(())
}

const MISSING_EDITS: [Edit; 14] = [
    |input| input.policy_id = campaign_core::Id::absent(),
    |input| input.guidance_profile_id = campaign_core::Id::absent(),
    |input| input.entropy_profile_id = campaign_core::Id::absent(),
    |input| input.budget_profile_id = campaign_core::Id::absent(),
    |input| input.adapter_id = campaign_core::Id::absent(),
    |input| input.control_domain_id = campaign_core::Id::absent(),
    |input| input.projection.projection_id = campaign_core::Id::absent(),
    |input| input.projection.graph_id = campaign_core::Id::absent(),
    |input| input.bounds.bounds_id = campaign_core::Id::absent(),
    |input| input.roots[0].moment_id = campaign_core::Id::absent(),
    |input| input.roots[0].projection_id = campaign_core::Id::absent(),
    |input| input.roots[0].adapter_id = campaign_core::Id::absent(),
    |input| input.resource_budgets[0].dimension_id = campaign_core::Id::absent(),
    |input| input.operation_schema_ids[0] = campaign_core::Id::absent(),
];

#[test]
// r[verify campaign.identity.missing]
fn a_missing_identity_fails_campaign_admission() -> Result<(), String> {
    for edit in MISSING_EDITS {
        let mut input = wide_input()?;
        edit(&mut input);
        assert_eq!(
            campaign_core::admit_campaign(input),
            Err(campaign_core::CampaignError::MissingIdentity)
        );
    }
    assert!(matches!(
        campaign_core::admit_campaign(wide_input()?),
        Ok(campaign_core::CampaignControlEvent::CampaignAdmitted { .. })
    ));
    Ok(())
}
