//! Identity stability: equal inputs derive equal identities; permuted roots, budgets, schemas,
//! and candidate batches normalize to equal campaign identities and batch events.

/// Largest generated set of roots, budget dimensions, or operation schemas.
const MAX_LISTED: u64 = 4;
/// Largest generated fresh candidate count in one batch.
const MAX_BATCH: usize = 6;
/// Largest generated number of resubmitted duplicates in one coalescing batch.
const MAX_DUPLICATES: usize = 3;

#[derive(Clone, Debug)]
struct AdmissionFacts {
    roots: u64,
    budgets: Vec<u64>,
    schemas: u64,
    keys: Vec<u32>,
    changed: u8,
}

fn admission_facts() -> impl proptest::strategy::Strategy<Value = AdmissionFacts> {
    proptest::strategy::Strategy::prop_map(
        (
            1..=MAX_LISTED,
            proptest::collection::vec(1..=crate::suite::strategies::MAX_BUDGET, 1..=4),
            1..=MAX_LISTED,
            crate::suite::strategies::keys(),
            proptest::num::u8::ANY,
        ),
        |(roots, budgets, schemas, keys, changed)| AdmissionFacts {
            roots,
            budgets,
            schemas,
            keys,
            changed,
        },
    )
}

fn wide_limits() -> crate::suite::fixture::Limits {
    crate::suite::fixture::Limits {
        max_outstanding: crate::suite::fixture::BOUND_WIDE,
        max_candidates: crate::suite::fixture::BOUND_WIDE,
        max_candidate_batch: crate::suite::fixture::BOUND_WIDE,
        max_selections: crate::suite::fixture::BOUND_WIDE,
        max_guidance_records: crate::suite::fixture::BOUND_WIDE,
        max_depth: crate::suite::fixture::BOUND_WIDE,
        max_control_events: crate::suite::fixture::BOUND_WIDE,
        rank_dimensions: crate::suite::strategies::MAX_RANK_DIMENSIONS,
    }
}

/// Admission input listing roots, budgets, and schemas; `keys` permutes every list.
fn listed(
    facts: &AdmissionFacts,
    keys: &[u32],
) -> Result<campaign_core::CampaignAdmission, String> {
    let projection = crate::suite::fixture::projection()?;
    let roots: Vec<campaign_core::RootMoment> = (0..facts.roots)
        .map(|index| crate::suite::fixture::root(&projection, index))
        .collect();
    let budgets: Vec<campaign_core::ResourceAmount> = (0_u64..)
        .zip(&facts.budgets)
        .map(|(index, amount)| campaign_core::ResourceAmount {
            dimension_id: crate::suite::fixture::dimension(index),
            amount: *amount,
        })
        .collect();
    let schemas: Vec<campaign_core::Id> = (0..facts.schemas)
        .map(crate::suite::fixture::schema)
        .collect();
    assert_eq!(
        roots.len(),
        usize::try_from(facts.roots).map_err(|error| format!("{error}"))?
    );
    assert_eq!(budgets.len(), facts.budgets.len());
    crate::suite::fixture::admission(
        crate::suite::fixture::bounds(&wide_limits())?,
        crate::suite::strategies::permute(&roots, keys),
        crate::suite::strategies::permute(&budgets, keys),
        crate::suite::strategies::permute(&schemas, keys),
    )
}

fn admission_derivation_is_stable(facts: &AdmissionFacts) -> Result<(), String> {
    let canonical = listed(facts, &[])?;
    let permuted = listed(facts, &facts.keys)?;
    let first = crate::suite::fixture::observed(canonical.clone().derive())?;
    let second = crate::suite::fixture::observed(canonical.clone().derive())?;
    assert_eq!(first, second);
    assert_eq!(first.campaign_id.kind, campaign_core::IdKind::Campaign);
    let reordered = crate::suite::fixture::observed(permuted.clone().derive())?;
    assert_eq!(reordered.campaign_id, first.campaign_id);
    assert_eq!(reordered, first);
    let (canonical_event, canonical_state) = crate::suite::fixture::campaign(canonical)?;
    let (permuted_event, permuted_state) = crate::suite::fixture::campaign(permuted)?;
    assert_eq!(permuted_event, canonical_event);
    assert_eq!(permuted_state, canonical_state);
    let mut changed = listed(facts, &facts.keys)?;
    let index = crate::suite::fixture::position(facts.changed, changed.resource_budgets.len())?;
    let budget = changed
        .resource_budgets
        .get_mut(index)
        .ok_or_else(|| String::from("a generated campaign has a budget"))?;
    budget.amount = budget.amount.saturating_add(1);
    let changed = crate::suite::fixture::observed(changed.derive())?;
    assert_ne!(changed.campaign_id, first.campaign_id);
    Ok(())
}

#[test]
// r[verify campaign.identity.changed]
// r[verify campaign.identity.equal]
fn permuted_admission_inputs_derive_one_campaign_id() -> Result<(), String> {
    crate::suite::config::check(
        crate::suite::config::INPUT_CASES,
        &admission_facts(),
        |facts| admission_derivation_is_stable(&facts),
    )
}

#[derive(Clone, Debug)]
struct BatchFacts {
    drafts: Vec<crate::suite::strategies::Draft>,
    duplicates: Vec<u8>,
    keys: Vec<u32>,
}

fn batch_facts() -> impl proptest::strategy::Strategy<Value = BatchFacts> {
    proptest::strategy::Strategy::prop_map(
        (
            proptest::collection::vec(
                proptest::strategy::Strategy::prop_map(
                    (
                        proptest::num::u8::ANY,
                        proptest::collection::vec(0..=crate::suite::strategies::MAX_COST, 2),
                    ),
                    |(parent_pick, costs)| crate::suite::strategies::Draft { parent_pick, costs },
                ),
                1..=MAX_BATCH,
            ),
            proptest::collection::vec(proptest::num::u8::ANY, 1..=MAX_DUPLICATES),
            crate::suite::strategies::keys(),
        ),
        |(drafts, duplicates, keys)| BatchFacts {
            drafts,
            duplicates,
            keys,
        },
    )
}

fn batch_state() -> Result<campaign_core::FrontierState, String> {
    let facts = AdmissionFacts {
        roots: MAX_LISTED,
        budgets: vec![crate::suite::strategies::MAX_BUDGET; 2],
        schemas: 1,
        keys: Vec::new(),
        changed: 0,
    };
    let (_event, state) = crate::suite::fixture::campaign(listed(&facts, &[])?)?;
    Ok(state)
}

fn records(
    state: &campaign_core::FrontierState,
    drafts: &[crate::suite::strategies::Draft],
) -> Result<Vec<campaign_core::CandidateRecord>, String> {
    let roots = &state.known_moments;
    let mut out = Vec::with_capacity(drafts.len());
    for (label, draft) in (0_u64..).zip(drafts) {
        let parent = roots
            .get(crate::suite::fixture::position(
                draft.parent_pick,
                roots.len(),
            )?)
            .copied()
            .ok_or_else(|| String::from("a campaign has a root"))?;
        let costs = crate::suite::fixture::costs(&draft.costs);
        let record = crate::suite::fixture::record(state, label, parent, costs);
        let first = crate::suite::fixture::observed(record.clone().derive())?;
        let second = crate::suite::fixture::observed(record.derive())?;
        assert_eq!(first, second);
        assert_eq!(first.candidate_id.kind, campaign_core::IdKind::Candidate);
        out.push(first);
    }
    Ok(out)
}

fn admit(
    state: &campaign_core::FrontierState,
    records: Vec<campaign_core::CandidateRecord>,
    coalesce: bool,
) -> Result<campaign_core::CampaignControlEvent, String> {
    crate::suite::fixture::observed(campaign_core::admit_candidates(
        state,
        records,
        crate::suite::fixture::rule(coalesce),
        crate::suite::fixture::coalescing_policy(),
    ))
}

fn batch_derivation_is_stable(facts: &BatchFacts) -> Result<(), String> {
    let state = batch_state()?;
    let canonical = records(&state, &facts.drafts)?;
    let event = admit(&state, canonical.clone(), false)?;
    let permuted = admit(
        &state,
        crate::suite::strategies::permute(&canonical, &facts.keys),
        false,
    )?;
    assert_eq!(permuted, event);
    let admitted = campaign_core::CampaignControlEvent::CampaignAdmitted {
        event_id: crate::suite::fixture::observed(campaign_core::derive_campaign_event_id(
            &state.admission,
        ))?,
        admission: state.admission.clone(),
    };
    let canonical_history = [admitted.clone(), event];
    let permuted_history = [admitted, permuted];
    let first = crate::suite::fixture::observed(campaign_core::reconstruct(&canonical_history))?;
    let second = crate::suite::fixture::observed(campaign_core::reconstruct(&permuted_history))?;
    assert_eq!(second, first);
    assert_eq!(first.candidates.len(), canonical.len());
    let mut submitted = canonical.clone();
    for pick in &facts.duplicates {
        let duplicate = canonical
            .get(crate::suite::fixture::position(*pick, canonical.len())?)
            .cloned()
            .ok_or_else(|| String::from("a batch has a record"))?;
        submitted.push(duplicate);
    }
    let rejected = campaign_core::admit_candidates(
        &state,
        crate::suite::strategies::permute(&submitted, &facts.keys),
        crate::suite::fixture::rule(false),
        crate::suite::fixture::coalescing_policy(),
    );
    assert_eq!(
        crate::suite::fixture::denied(rejected)?,
        campaign_core::CampaignError::DuplicateIdentity
    );
    let coalesced = admit(&state, submitted.clone(), true)?;
    let reordered = admit(
        &state,
        crate::suite::strategies::permute(&submitted, &facts.keys),
        true,
    )?;
    assert_eq!(reordered, coalesced);
    let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) = &coalesced else {
        return Err(format!("candidate admission produced {coalesced:?}"));
    };
    assert_eq!(batch.outcomes.len(), submitted.len());
    assert!(
        batch
            .candidates
            .is_sorted_by_key(|record| record.candidate_id)
    );
    assert!(
        batch
            .outcomes
            .is_sorted_by_key(|outcome| outcome.candidate_id)
    );
    assert!(batch.candidates.len() < submitted.len());
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.order]
fn permuted_candidate_batches_admit_one_canonical_event() -> Result<(), String> {
    crate::suite::config::check(crate::suite::config::INPUT_CASES, &batch_facts(), |facts| {
        batch_derivation_is_stable(&facts)
    })
}
