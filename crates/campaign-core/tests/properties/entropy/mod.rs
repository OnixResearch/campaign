//! Explicit-entropy determinism over generated histories: equal state, profile, and ticket give
//! equal decisions; a selection stays inside the eligible set; the exploit or explore branch
//! follows `explore_draw % ENTROPY_SCALE < threshold`.

/// Branch counts observed across all generated cases.
#[derive(Default)]
struct Branches {
    exploit: core::cell::Cell<u64>,
    explore: core::cell::Cell<u64>,
}

impl Branches {
    fn record(&self, is_explored: bool) {
        let counter = if is_explored {
            &self.explore
        } else {
            &self.exploit
        };
        counter.set(counter.get().saturating_add(1));
    }
}

/// Exploratory profile facts and ticket of one request.
struct Exploratory<'a> {
    threshold: u64,
    order: &'a [u32],
    ticket: &'a campaign_core::EntropyTicket,
}

fn exploratory(request: &campaign_core::SelectionRequest) -> Option<Exploratory<'_>> {
    let campaign_core::SelectionMode::Exploratory {
        threshold_parts_per_million,
        exploration_order,
        ..
    } = &request.profile.mode
    else {
        return None;
    };
    let ticket = request.entropy.as_ref()?;
    debug_assert_eq!(ticket.profile_id, request.profile.profile_id);
    debug_assert_eq!(ticket.state_id, request.state_id);
    Some(Exploratory {
        threshold: *threshold_parts_per_million,
        order: exploration_order,
        ticket,
    })
}

/// The chosen candidate sits where the branch the explore draw selects puts it.
fn branch_is_consistent(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    facts: &Exploratory<'_>,
    branches: &Branches,
) -> Result<(), String> {
    let is_explored = crate::suite::oracle::explores(facts.ticket, facts.threshold);
    branches.record(is_explored);
    if is_explored {
        let explored_order = crate::suite::oracle::exploration_order(state, facts.order);
        let position = explored_order
            .iter()
            .position(|item| *item == candidate_id)
            .and_then(|index| u64::try_from(index).ok());
        let count = u64::try_from(explored_order.len()).map_err(|error| format!("{error}"))?;
        assert!(count > 0);
        assert_eq!(position, facts.ticket.index_draw().checked_rem(count));
    } else {
        let ranked = crate::suite::oracle::ranked_order(state)?;
        assert_eq!(ranked.first(), Some(&candidate_id));
    }
    Ok(())
}

fn planning_is_deterministic(
    planning: &crate::suite::interpreter::Planning,
    branches: &Branches,
) -> Result<(), String> {
    let state = &planning.state;
    let again = crate::suite::fixture::observed(crate::suite::requests::plan(
        state,
        planning.request.clone(),
    ))?;
    assert_eq!(again, planning.plan);
    assert_eq!(planning.plan.state_id, state.state_id);
    let campaign_core::NextDecision::Select(selection) = &planning.plan.decision else {
        return Ok(());
    };
    let eligible: Vec<campaign_core::Id> = crate::suite::oracle::eligible(state)
        .iter()
        .map(|candidate| candidate.candidate.candidate_id)
        .collect();
    assert!(eligible.contains(&selection.candidate_id));
    assert_eq!(
        usize::try_from(planning.plan.work.eligible_candidates).ok(),
        Some(eligible.len())
    );
    assert_eq!(
        selection.entropy_id,
        planning
            .request
            .entropy
            .as_ref()
            .map(|ticket| ticket.entropy_id)
    );
    let expected = crate::suite::oracle::expected_choice(state, &planning.request)?;
    assert_eq!(Some(selection.candidate_id), expected);
    let Some(facts) = exploratory(&planning.request) else {
        return Ok(());
    };
    branch_is_consistent(state, selection.candidate_id, &facts, branches)
}

#[test]
// r[verify campaign.entropy.valid]
fn equal_explicit_inputs_choose_equally_within_the_eligible_set() -> Result<(), String> {
    let branches = Branches::default();
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &crate::suite::strategies::script(),
        |script| {
            let history = crate::suite::interpreter::run(&script)?;
            for planning in &history.plannings {
                planning_is_deterministic(planning, &branches)?;
            }
            Ok(())
        },
    )?;
    assert!(
        branches.exploit.get() > 0,
        "no generated case took the exploit branch"
    );
    assert!(
        branches.explore.get() > 0,
        "no generated case took the explore branch"
    );
    Ok(())
}
