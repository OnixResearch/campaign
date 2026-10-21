//! Unchanged-state rejection: randomly corrupted requests (crossed identities, stale states and
//! projections, over-bound batches, invalid guidance, stale publications) return the exact typed
//! error, and the input state, its identity, its history, and its next valid plan are unchanged.

mod candidates;
mod guidance;
mod planning;

/// Expected and returned denial of one corrupted call.
pub(crate) struct Outcome {
    pub(crate) expected: campaign_core::CampaignError,
    pub(crate) actual: campaign_core::CampaignError,
}

#[derive(Clone, Debug)]
enum Fault {
    Request(planning::RequestFault),
    Entropy(planning::EntropyFault),
    Stale(planning::StateFault),
    Candidates(candidates::Fault),
    Guidance(guidance::Fault),
}

fn fault() -> impl proptest::strategy::Strategy<Value = Fault> {
    proptest::prop_oneof![
        proptest::strategy::Strategy::prop_map(planning::request_fault(), Fault::Request),
        proptest::strategy::Strategy::prop_map(planning::entropy_fault(), Fault::Entropy),
        proptest::strategy::Strategy::prop_map(planning::state_fault(), Fault::Stale),
        proptest::strategy::Strategy::prop_map(candidates::fault(), Fault::Candidates),
        proptest::strategy::Strategy::prop_map(guidance::fault(), Fault::Guidance),
    ]
}

fn denial_leaves_state_unchanged(
    script: &crate::suite::strategies::Script,
    fault: &Fault,
) -> Result<bool, String> {
    let history = crate::suite::interpreter::run(script)?;
    let before = history.state()?.clone();
    let baseline = crate::suite::fixture::observed(crate::suite::requests::plan(
        &before,
        planning::ranked(&before)?,
    ))?;
    let outcome = match fault {
        Fault::Request(item) => planning::attempt_request(&history, item)?,
        Fault::Entropy(item) => planning::attempt_entropy(&history, &script.exploration, item)?,
        Fault::Stale(item) => planning::attempt_state(&history, item)?,
        Fault::Candidates(item) => candidates::attempt(&history, item)?,
        Fault::Guidance(item) => guidance::attempt(&history, item)?,
    };
    let Some(outcome) = outcome else {
        return Ok(false);
    };
    let is_new_work = matches!(fault, Fault::Candidates(_) | Fault::Guidance(_));
    let expected = if is_new_work && !crate::suite::oracle::has_new_work_headroom(&before)? {
        campaign_core::CampaignError::ControlEventLimitReached
    } else {
        outcome.expected
    };
    assert_eq!(outcome.actual, expected, "{fault:?}");
    let state = history.state()?;
    assert_eq!(state.state_id, before.state_id);
    assert_eq!(state, &before);
    let replayed = crate::suite::fixture::observed(campaign_core::reconstruct(&history.events))?;
    assert_eq!(replayed, before);
    let replanned = crate::suite::fixture::observed(crate::suite::requests::plan(
        state,
        planning::ranked(state)?,
    ))?;
    assert_eq!(replanned, baseline);
    Ok(true)
}

#[test]
// r[verify campaign.entropy.invalid]
// r[verify campaign.guidance.invalid]
fn corrupted_requests_are_denied_without_mutation() -> Result<(), String> {
    let applied = core::cell::Cell::new(0_u64);
    crate::suite::config::check(
        crate::suite::config::HISTORY_CASES,
        &(crate::suite::strategies::script(), fault()),
        |(script, fault)| {
            if denial_leaves_state_unchanged(&script, &fault)? {
                applied.set(applied.get().saturating_add(1));
            }
            Ok(())
        },
    )?;
    assert!(applied.get() > 0, "no generated corruption applied");
    Ok(())
}
