//! Bounded candidate admission at intake and replay.

type Edit = fn(&mut campaign_core::CandidateRecord, &campaign_core::FrontierState);

const ONE_DIMENSION: u64 = 1;
const ZERO_COST: u64 = 0;

fn other(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(kind, label)
}

const INVALID: [(&str, Edit, campaign_core::CampaignError); 10] = [
    (
        "unknown parent",
        |record, _| record.parent_moment_id = other(campaign_core::IdKind::ChildMoment, b"unknown"),
        campaign_core::CampaignError::MissingParentMoment,
    ),
    (
        "crossed campaign",
        |record, _| record.campaign_id = other(campaign_core::IdKind::Campaign, b"other-campaign"),
        campaign_core::CampaignError::CrossedIdentity,
    ),
    (
        "unsupported schema",
        |record, _| {
            record.operation_schema_id =
                other(campaign_core::IdKind::OperationSchema, b"unadmitted-schema");
        },
        campaign_core::CampaignError::UnsupportedSchema,
    ),
    (
        "stale adapter",
        |record, _| record.source_adapter_id = other(campaign_core::IdKind::Adapter, b"stale"),
        campaign_core::CampaignError::CrossedIdentity,
    ),
    (
        "excessive depth",
        |record, state| record.depth = state.admission.bounds.max_depth.saturating_add(1),
        campaign_core::CampaignError::CandidateOverDepth,
    ),
    (
        "zero cost",
        |record, _| record.declared_costs[0].amount = ZERO_COST,
        campaign_core::CampaignError::InvalidCostProfile,
    ),
    (
        "empty cost profile",
        |record, _| record.declared_costs.clear(),
        campaign_core::CampaignError::InvalidCostProfile,
    ),
    (
        "unknown cost dimension",
        |record, _| {
            record.declared_costs[0].dimension_id = other(
                campaign_core::IdKind::ResourceDimension,
                b"unknown-dimension",
            );
        },
        campaign_core::CampaignError::InvalidCostProfile,
    ),
    (
        "cost over budget",
        |record, state| {
            record.declared_costs[0].amount =
                state.admission.resource_budgets[0].amount.saturating_add(1);
        },
        campaign_core::CampaignError::BudgetExhausted,
    ),
    (
        "cost over dimension count",
        |record, _| {
            record.declared_costs.push(campaign_core::ResourceAmount {
                dimension_id: other(campaign_core::IdKind::ResourceDimension, b"extra-dimension"),
                amount: crate::suite::constants::COST_ONE,
            });
        },
        campaign_core::CampaignError::BoundExceeded,
    ),
];

pub(super) fn fixture() -> Result<
    (
        crate::suite::ingress::support::Events,
        campaign_core::FrontierState,
        campaign_core::Id,
    ),
    String,
> {
    let (events, state) = crate::suite::ingress::support::admitted_with(|bounds| {
        bounds.max_resource_dimensions = ONE_DIMENSION;
    })?;
    crate::suite::candidate::admit(
        events,
        &state,
        b"existing",
        crate::suite::constants::COST_ONE,
    )
}

fn fresh(state: &campaign_core::FrontierState) -> campaign_core::CandidateRecord {
    crate::suite::candidate::record(state, b"fresh", crate::suite::constants::COST_ONE)
}

#[test]
// r[verify campaign.candidate_admission.valid]
fn a_unique_bounded_candidate_enters_the_frontier() -> Result<(), String> {
    let (events, state, _existing) = fixture()?;
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![fresh(&state)],
        crate::suite::ingress::support::REJECT,
    ))?;
    let batch = crate::suite::ingress::support::batch_of(&event)?;
    assert_eq!(batch.state_before_id, state.state_id);
    assert_eq!(batch.candidates.len(), 1);
    let candidate_id = batch.candidates[0].candidate_id;
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    let entry = crate::suite::ingress::support::text(next.candidate(candidate_id))?;
    assert_eq!(entry.status, campaign_core::CandidateStatus::Retained);
    assert_eq!(entry.selection_count, campaign_core::NO_SELECTIONS);
    assert_eq!(
        next.candidates.len(),
        state.candidates.len().saturating_add(1)
    );
    assert_ne!(next.state_id, state.state_id);
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.invalid]
fn invalid_candidates_are_denied_at_intake_without_mutation() -> Result<(), String> {
    let (events, state, existing) = fixture()?;
    for (name, edit, expected) in INVALID {
        let mut record = fresh(&state);
        edit(&mut record, &state);
        assert_eq!(
            crate::suite::ingress::support::admit_batch(
                &state,
                vec![record],
                crate::suite::ingress::support::REJECT
            ),
            Err(expected.clone()),
            "{name}"
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let existing_record = crate::suite::ingress::support::text(state.candidate(existing))?
        .candidate
        .clone();
    assert_eq!(
        crate::suite::ingress::support::admit_batch(
            &state,
            vec![existing_record],
            crate::suite::ingress::support::REJECT
        ),
        Err(campaign_core::CampaignError::DuplicateIdentity)
    );
    assert_eq!(
        crate::suite::ingress::support::admit_batch(
            &state,
            vec![fresh(&state), fresh(&state)],
            crate::suite::ingress::support::REJECT
        ),
        Err(campaign_core::CampaignError::DuplicateIdentity)
    );
    let mut forged = crate::suite::ingress::support::text(fresh(&state).derive())?;
    forged.candidate_id = existing;
    assert_eq!(
        crate::suite::ingress::support::admit_batch(
            &state,
            vec![forged],
            crate::suite::ingress::support::REJECT
        ),
        Err(campaign_core::CampaignError::MalformedIdentity)
    );
    crate::suite::ingress::support::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.candidate_admission.invalid]
fn invalid_candidates_are_denied_at_replay_without_mutation() -> Result<(), String> {
    let (events, state, existing) = fixture()?;
    let valid = crate::suite::ingress::support::text(fresh(&state).derive())?;
    for (name, edit, expected) in INVALID {
        let mut record = fresh(&state);
        edit(&mut record, &state);
        let forged = match record.clone().derive() {
            Ok(derived) => derived,
            Err(_) => campaign_core::CandidateRecord {
                candidate_id: valid.candidate_id,
                ..record
            },
        };
        assert_eq!(
            crate::suite::ingress::support::replay_with(
                &events,
                crate::suite::ingress::support::forged_batch(
                    &state,
                    vec![forged],
                    crate::suite::ingress::support::REJECT
                )?
            ),
            Err(expected.clone()),
            "{name}"
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let existing_record = crate::suite::ingress::support::text(state.candidate(existing))?
        .candidate
        .clone();
    for duplicate in [vec![existing_record], vec![valid.clone(), valid]] {
        assert_eq!(
            crate::suite::ingress::support::replay_with(
                &events,
                crate::suite::ingress::support::forged_batch(
                    &state,
                    duplicate,
                    crate::suite::ingress::support::REJECT
                )?
            ),
            Err(campaign_core::CampaignError::DuplicateIdentity)
        );
    }
    crate::suite::ingress::support::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.candidate_admission.batch]
fn a_candidate_binds_its_operation_generation_profile_without_selection() -> Result<(), String> {
    let (events, state, _existing) = fixture()?;
    let single = fresh(&state);
    let grouped = campaign_core::CandidateRecord {
        operation_generation_profile_id: other(
            campaign_core::IdKind::Policy,
            b"bounded-action-group",
        ),
        ..fresh(&state)
    };
    let single_id = crate::suite::ingress::support::text(single.clone().derive())?.candidate_id;
    let grouped_id = crate::suite::ingress::support::text(grouped.clone().derive())?.candidate_id;
    assert_ne!(single_id, grouped_id);
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![single, grouped],
        crate::suite::ingress::support::REJECT,
    ))?;
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    let entry = crate::suite::ingress::support::text(next.candidate(grouped_id))?;
    assert_eq!(
        entry.candidate.operation_generation_profile_id,
        other(campaign_core::IdKind::Policy, b"bounded-action-group")
    );
    assert!(next.outstanding.is_empty());
    assert!(next.discoveries.is_empty());
    assert_eq!(next.known_moments, state.known_moments);
    Ok(())
}
