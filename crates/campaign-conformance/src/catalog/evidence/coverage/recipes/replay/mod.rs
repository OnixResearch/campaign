//! Replays a canonical recipe through the adapter: every record is mapped by the adapter and every
//! control event published over real Choregraph history, and the reached state must lie in the
//! recipe's class.

mod steps;

/// Operations of a recipe, in suite operation order: the three standard operations and a fourth
/// (handle four is the reject rule's same-batch refusal operation).
pub(in crate::catalog::evidence) const OPERATIONS: [crate::vocabulary::OperationHandle; 4] = [
    super::super::super::super::common::OP_A,
    super::super::super::super::common::OP_B,
    super::super::super::super::common::OP_C,
    crate::vocabulary::OperationHandle(5),
];

/// What a replay leaves beside its session: the plan receipts of the plans it made (each
/// projected when planned, with its receipt kind), the history before its head as captured
/// before the head's publication (none for a bare admission), the latest state before any stop,
/// and the state of every shorter prefix of the lineage (`prefixes[length - 1]`).
pub(in crate::catalog::evidence) struct Replayed {
    pub(in crate::catalog::evidence) plans: alloc::vec::Vec<(u32, campaign_core::CampaignReceipt)>,
    pub(in crate::catalog::evidence) before:
        Option<crate::catalog::evidence::lineages::forks::Base>,
    pub(in crate::catalog::evidence) unstopped: campaign_core::FrontierState,
    pub(in crate::catalog::evidence) prefixes: alloc::vec::Vec<campaign_core::FrontierState>,
}

/// Opens a campaign under the recipe's configuration, plans the frontier stop of its bare
/// admission (keeping both plan receipts), and replays its steps; the reached state must lie in
/// `class`.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a step, and `Diverged` when a step finds
/// no candidate to act on or the reached state lies in another class.
// r[impl campaign.conformance.state_coverage]
// r[impl campaign.conformance.bounds]
pub(in crate::catalog::evidence) fn build<'s, 'a>(
    harness: &'s mut crate::harness::Driver<'a>,
    class: super::super::features::StateClass,
) -> crate::verdicts::ConformanceResult<(crate::harness::Session<'s, 'a>, Replayed)> {
    let recipe = super::recipe(class);
    let config = recipe.config;
    let mut session = crate::harness::Session::open(harness, config.budget_units, |bounds| {
        if let Some(limit) = config.max_selections {
            bounds.max_selections = limit;
        }
        if let Some(limit) = config.max_control_events {
            bounds.max_control_events = limit;
        }
        if let Some(limit) = config.max_guidance_records {
            bounds.max_guidance_records = limit;
        }
    })?;
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Coalesce;
    let mut trail = steps::Trail {
        plans: alloc::vec::Vec::new(),
        before: None,
        unstopped: session.state.clone(),
        prefixes: alloc::vec::Vec::new(),
        children: 0,
    };
    trail.admission_stop(&mut session)?;
    for step in &recipe.steps {
        if matches!(step, super::Step::Stop(_)) {
            trail.unstopped = session.state.clone();
        }
        trail.step(&mut session, *step)?;
    }
    if session.state.stopped.is_none() {
        trail.unstopped = session.state.clone();
    }
    debug_assert!(
        trail.unstopped.stopped.is_none(),
        "the latest state before any stop is running"
    );
    debug_assert_eq!(
        trail.before.is_some(),
        !trail.prefixes.is_empty(),
        "every publication captures its prefix and the history before it"
    );
    super::super::super::super::common::require(
        super::super::features::of(&session.state) == class
            && trail.prefixes.len().saturating_add(1) == session.payloads.len(),
        "the canonical recipe reaches its declared class",
    )?;
    Ok((
        session,
        Replayed {
            plans: trail.plans,
            before: trail.before,
            unstopped: trail.unstopped,
            prefixes: trail.prefixes,
        },
    ))
}
