//! Bounded reachability over campaign-core: from the admission of each suite admission
//! configuration, breadth-first over every control event a consumer can publish (`steps`), every
//! state reached within `DEPTH` control events after the admission lies in a declared class.
//!
//! Instance bounds: at most three candidates (the standard operations A, B, C below the root, in
//! suite order, unit cost), the Animus-shaped adapter's records, the default profile's selection
//! and retention orders, and the configurations of `CONFIGURATIONS` (the narrowed bounds the
//! recipes use). States are deduplicated by their identity with path-derived identities erased
//! (`erased`); a shallower search deduplicated by exact state identity (`state_id`) reaches the same
//! classes.

mod erased;
mod steps;

/// Control events after the admission the search explores.
const DEPTH: usize = 8;

/// Depth of the cross-check deduplicated by exact state identity.
const EXACT_DEPTH: usize = 3;

/// One admission configuration: budget units and the bounds it narrows.
type Configuration = (u64, fn(&mut campaign_core::ExplorationBounds));

/// The admission configurations the search starts from: ample, small budgets, one selection, a
/// small control-event bound, one guidance record, and one retained candidate.
const CONFIGURATIONS: [Configuration; 7] = [
    (64, |_| {}),
    (1, |_| {}),
    (2, |bounds| bounds.max_retained_candidates = 1),
    (64, |bounds| bounds.max_selections = 1),
    (64, |bounds| bounds.max_control_events = 6),
    (64, |bounds| bounds.max_guidance_records = 1),
    (64, |bounds| bounds.max_retained_candidates = 1),
];

/// Records the Animus-shaped adapter maps for every search operation below the root.
///
/// # Errors
///
/// Returns the adapter error of a record mapping and the Campaign error of its derivation.
fn records(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<
    alloc::vec::Vec<(
        crate::vocabulary::OperationHandle,
        campaign_core::CandidateRecord,
    )>,
> {
    let root = session
        .moment(crate::harness::history::ROOT)?
        .identity
        .moment_id();
    let mut records = alloc::vec::Vec::with_capacity(steps::OPERATIONS.len());
    for operation in steps::OPERATIONS {
        let context = crate::port::CandidateContext {
            campaign_id: session.state.admission.campaign_id,
            parent_moment_id: root,
            depth: 1,
        };
        let scenario = crate::catalog::common::operation(
            operation,
            root_handle(),
            crate::catalog::common::UNIT_COST,
        );
        let record = session
            .harness
            .adapter
            .candidate(&scenario, &context)?
            .derive()?;
        records.push((operation, record));
    }
    debug_assert!(
        records
            .iter()
            .all(|(_, record)| record.parent_moment_id == root),
        "every record names the root as its parent"
    );
    debug_assert!(
        records
            .iter()
            .all(|(_, record)| record.campaign_id == session.state.admission.campaign_id),
        "every record belongs to the searched campaign"
    );
    Ok(records)
}

/// The successors of `node` not seen before under `dedup`, with the decisions of their
/// outstanding selections; every class reached must be declared, and joins `reached`.
///
/// # Errors
///
/// Returns the adapter error of a record mapping.
fn expand<K: Ord>(
    context: &mut steps::Context<'_, '_, '_>,
    node: &steps::Node,
    dedup: fn(&campaign_core::FrontierState) -> K,
    (seen, reached): (
        &mut alloc::collections::BTreeSet<K>,
        &mut alloc::collections::BTreeSet<u16>,
    ),
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<steps::Node>> {
    let successors = context.successors(node)?;
    let mut expanded = alloc::vec::Vec::with_capacity(successors.len());
    for (event, decision) in successors {
        let Ok(state) = campaign_core::apply_control_event(node.state.clone(), &event) else {
            continue;
        };
        if !seen.insert(dedup(&state)) {
            continue;
        }
        let class = super::features::of(&state);
        assert!(
            super::classes::admits(class),
            "undeclared class {class:?} reached"
        );
        reached.insert(class.code());
        let mut decisions: alloc::vec::Vec<campaign_core::SelectionDecision> = node
            .decisions
            .iter()
            .chain(decision.as_ref())
            .filter(|known| {
                state
                    .outstanding
                    .iter()
                    .any(|held| held.selection_id == known.selection_id)
            })
            .cloned()
            .collect();
        decisions.sort_by_key(|known| known.selection_id);
        expanded.push(steps::Node { state, decisions });
    }
    debug_assert!(
        expanded
            .iter()
            .all(|next| next.decisions.len() <= next.state.outstanding.len()),
        "a node keeps only the decisions of its outstanding selections"
    );
    Ok(expanded)
}

/// Classes reached within `depth` events under `configuration`, deduplicated by `dedup`, and the
/// states explored; every reached class must be declared.
fn search<K: Ord>(
    configuration: Configuration,
    depth: usize,
    dedup: fn(&campaign_core::FrontierState) -> K,
) -> crate::verdicts::ConformanceResult<(alloc::collections::BTreeSet<u16>, usize)> {
    debug_assert!(depth <= DEPTH, "the search explores at most `DEPTH` events");
    let profile = crate::tests::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let (budget, configure) = configuration;
    let mut session = crate::harness::Session::open(&mut harness, budget, configure)?;
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Coalesce;
    let records = records(&mut session)?;
    let profiles = [
        session.selection_profile(false)?,
        session.selection_profile(true)?,
    ];
    let binding = session.binding()?;
    let start = steps::Node {
        state: session.state.clone(),
        decisions: alloc::vec::Vec::new(),
    };
    let mut context = steps::Context {
        session: &mut session,
        profiles,
        records,
        binding,
    };
    let mut seen = alloc::collections::BTreeSet::new();
    seen.insert(dedup(&start.state));
    let mut reached = alloc::collections::BTreeSet::new();
    reached.insert(super::features::of(&start.state).code());
    let mut frontier = alloc::vec![start];
    for _ in 0..depth {
        let mut next = alloc::vec::Vec::with_capacity(frontier.len());
        for node in &frontier {
            next.extend(expand(
                &mut context,
                node,
                dedup,
                (&mut seen, &mut reached),
            )?);
        }
        frontier = next;
    }
    debug_assert!(
        reached.len() <= seen.len(),
        "every reached class is the class of a seen state"
    );
    Ok((reached, seen.len()))
}

/// Handle of the root moment every search operation starts from.
const fn root_handle() -> crate::vocabulary::MomentHandle {
    crate::harness::history::ROOT
}

#[test]
// r[verify campaign.conformance.state_coverage]
fn every_state_reached_within_the_bound_lies_in_a_declared_class()
-> crate::verdicts::ConformanceResult<()> {
    let mut reached = alloc::collections::BTreeSet::new();
    let mut explored = 0_usize;
    for configuration in CONFIGURATIONS {
        let (classes, states) = search(configuration, DEPTH, erased::of)?;
        explored = explored.saturating_add(states);
        reached.extend(classes);
    }
    let declared: alloc::collections::BTreeSet<u16> = super::classes::all()
        .iter()
        .map(|class| class.code())
        .collect();
    assert!(reached.is_subset(&declared));
    assert!(
        explored >= reached.len(),
        "every reached class is the class of an explored state"
    );
    Ok(())
}

#[test]
// r[verify campaign.conformance.state_coverage]
fn the_erased_key_reaches_the_classes_exact_state_identity_reaches()
-> crate::verdicts::ConformanceResult<()> {
    for configuration in CONFIGURATIONS {
        let (exact, _) = search(configuration, EXACT_DEPTH, |state| state.state_id)?;
        let (erased, _) = search(configuration, EXACT_DEPTH, erased::of)?;
        assert_eq!(exact, erased);
    }
    Ok(())
}
