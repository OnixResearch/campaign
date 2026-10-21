//! Closeout-audit loophole adapters kept as negative controls: each loophole that once passed
//! every fixture now fails exactly the named fixtures that close it, in debug and release builds.

mod receipts;
mod wrapper;

/// Fixture name and declared forbidden effect of one negative fixture.
type Declared = (&'static str, &'static str);

/// Divergence every fixture reports when the adapter loads a projection of unpresented history.
const FORGED_BINDING: crate::verdicts::ConformanceError =
    crate::verdicts::ConformanceError::Diverged {
        expected: "the loaded projection binds the session's own graph and branch",
    };

/// Divergence of the fork fixtures when a result drops a reserved child.
const DROPPED_CHILD: crate::verdicts::ConformanceError =
    crate::verdicts::ConformanceError::Diverged {
        expected: "the forked result committed both reserved children",
    };

/// Fixtures that never load history through the adapter.
const NON_LOADING: [&str; 12] = [
    "identities.stable",
    "admission.batch",
    "admission.duplicate_rejected",
    "admission.duplicate_coalesced",
    "admission.hidden_admission",
    "guidance.coalesced_duplicate",
    "guidance.monotone_levels",
    "retirement.invalid",
    "pruning.declared_order",
    "pruning.authority_claims",
    "caches.fresh",
    "caches.moved_binding",
];

const FOREIGN_SELECTION: Declared = (
    "fencing.foreign_selection",
    "an unpublished selection executed under another selection's durable publication",
);
const STALE_STATE_SELECTION: Declared = (
    "fencing.stale_state_selection",
    "a stale-state selection executed under the winner's durable publication",
);
const RANKED_GATE_MATRIX: Declared = (
    "fencing.gate_matrix_ranked",
    "an unpublished selection executed under another selection's durable publication",
);
const EXPLORATORY_GATE_MATRIX: Declared = (
    "fencing.gate_matrix_exploratory",
    "an unpublished selection executed under another selection's durable publication",
);
/// Forbidden effect of the first receipt every claims fixture presents: the admission's event
/// receipt against the admitted state.
const ADMISSION_RECEIPT: &str = "a campaign admission event receipt carried a promotion claim";
const EVENT_RECEIPT_CLAIM: Declared = ("receipts.non_selection_overclaim", ADMISSION_RECEIPT);
const FABRICATED_ROLES: Declared = (
    "roles.collapsed_identity",
    "the adapter reported role identities other than its stored candidate, work item, and result",
);
const MOVED_GRAPH_CACHE: Declared = (
    "caches.moved_binding",
    "a cache bound to a superseded graph was used as current state",
);
const PLANNER_EVENT_CLAIM: Declared = ("receipts.planner_overclaim", ADMISSION_RECEIPT);
const RECORDED_EVENT_CLAIM: Declared = ("receipts.recorded_stop_overclaim", ADMISSION_RECEIPT);

/// Fork fixtures that detect a dropped reserved child, with their polarity.
const DROPPED_CHILD_FIXTURES: [(&str, crate::verdicts::FixturePolarity); 3] = [
    (
        "reconstruction.incremental_equals_rebuild",
        crate::verdicts::FixturePolarity::Positive,
    ),
    (
        "reconstruction.conflicting_history",
        crate::verdicts::FixturePolarity::Negative,
    ),
    (
        "results.fork_children",
        crate::verdicts::FixturePolarity::Positive,
    ),
];

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn run(
    loopholes: &[wrapper::Loophole],
) -> crate::verdicts::ConformanceResult<crate::verdicts::ConformanceReport> {
    let profile = super::default_profile()?;
    let mut adapter = wrapper::LoopholeAdapter::new(loopholes);
    Ok(crate::catalog::run_suite(&mut adapter, &profile))
}

/// The negative fixtures the authority loophole adapter lets through, in suite order.
const AUTHORITY_LET_THROUGH: [Declared; 8] = [
    FOREIGN_SELECTION,
    STALE_STATE_SELECTION,
    RANKED_GATE_MATRIX,
    EXPLORATORY_GATE_MATRIX,
    EVENT_RECEIPT_CLAIM,
    PLANNER_EVENT_CLAIM,
    RECORDED_EVENT_CLAIM,
    FABRICATED_ROLES,
];

/// Negative fixtures whose declared forbidden input or effect went through.
fn let_through(declared: &[Declared]) -> Failures {
    declared
        .iter()
        .map(|(name, forbidden)| {
            (
                *name,
                crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
            )
        })
        .collect()
}

/// Fixtures that failed with one divergence under their own polarity.
fn diverged<'a>(
    fixtures: impl Iterator<Item = (&'static str, crate::verdicts::FixturePolarity)> + 'a,
    error: &'a crate::verdicts::ConformanceError,
) -> impl Iterator<Item = (&'static str, crate::verdicts::FixtureVerdict)> + 'a {
    fixtures.map(move |(name, polarity)| {
        let verdict = if polarity == crate::verdicts::FixturePolarity::Positive {
            crate::verdicts::FixtureVerdict::PositiveFixtureFailed(error.clone())
        } else {
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(error.clone())
        };
        (name, verdict)
    })
}

/// Every fixture that loads history, failed with the forged-binding divergence.
fn forged_binding_failures(report: &crate::verdicts::ConformanceReport) -> Failures {
    let loading = report
        .outcomes
        .iter()
        .filter(|outcome| !NON_LOADING.contains(&outcome.name))
        .map(|outcome| (outcome.name, outcome.polarity));
    diverged(loading, &FORGED_BINDING).collect()
}

/// Settled transcript entries that name exactly these accepted children.
fn settled_with(
    report: &crate::verdicts::ConformanceReport,
    moments: &[crate::vocabulary::MomentHandle],
) -> usize {
    report
        .transcript
        .entries
        .iter()
        .filter(|entry| entry.kind == "settled" && entry.moments == moments)
        .count()
}

#[test]
fn the_authority_loophole_adapter_fails_the_gate_claim_and_role_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let report = run(wrapper::AUTHORITY)?;
    assert_eq!(
        super::failures(&report),
        crate::tests::state_coverage_let_through(
            let_through(&AUTHORITY_LET_THROUGH),
            ADMISSION_RECEIPT
        )
    );
    Ok(())
}

#[test]
fn an_own_execution_gate_fails_the_foreign_publication_and_gate_matrix_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let report = run(&[wrapper::Loophole::OwnGate])?;
    assert_eq!(
        super::failures(&report),
        let_through(&[
            FOREIGN_SELECTION,
            STALE_STATE_SELECTION,
            RANKED_GATE_MATRIX,
            EXPLORATORY_GATE_MATRIX,
        ])
    );
    Ok(())
}

#[test]
fn fabricated_distinct_role_identities_fail_the_collapsed_identity_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let report = run(&[wrapper::Loophole::FabricatedRoles])?;
    assert_eq!(super::failures(&report), let_through(&[FABRICATED_ROLES]));
    Ok(())
}

#[test]
fn the_shallow_adapter_fails_every_loading_fixture_and_the_moved_cache_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let report = run(wrapper::SHALLOW)?;
    let mut expected = forged_binding_failures(&report);
    let after_stale = expected
        .iter()
        .position(|(name, _)| *name == "caches.stale")
        .map_or(expected.len(), |stale| stale.saturating_add(1));
    expected.splice(after_stale..after_stale, let_through(&[MOVED_GRAPH_CACHE]));
    assert_eq!(super::failures(&report), expected);
    Ok(())
}

#[test]
fn dropped_children_fail_the_fork_fixtures_and_change_the_transcript()
-> crate::verdicts::ConformanceResult<()> {
    let report = run(&[wrapper::Loophole::DroppedChildren])?;
    let mut want: Failures = diverged(DROPPED_CHILD_FIXTURES.into_iter(), &DROPPED_CHILD).collect();
    let uncounted = crate::verdicts::ConformanceError::Diverged {
        expected: "the result reports and the state records every discovered child",
    };
    let counted = [(
        "results.child_counts",
        crate::verdicts::FixturePolarity::Positive,
    )];
    want.extend(diverged(counted.into_iter(), &uncounted));
    assert_eq!(super::failures(&report), want);
    let healthy = run(&[])?;
    let first_child = crate::vocabulary::MomentHandle(11);
    let both_children = [first_child, crate::vocabulary::MomentHandle(12)];
    assert!(settled_with(&healthy, &both_children) > 0);
    assert_eq!(settled_with(&report, &both_children), 0);
    assert!(settled_with(&report, &[first_child]) > settled_with(&healthy, &[first_child]));
    Ok(())
}

#[test]
fn a_state_only_cache_check_fails_the_moved_binding_and_cache_matrix_fixtures()
-> crate::verdicts::ConformanceResult<()> {
    let report = run(&[wrapper::Loophole::StateOnlyCache])?;
    assert_eq!(
        super::failures(&report),
        crate::tests::state_coverage_let_through(
            let_through(&[
                MOVED_GRAPH_CACHE,
                (
                    "caches.validation_matrix",
                    "the current state under a superseded binding was used as current state",
                ),
            ]),
            "a cache of a held state's lineage on an older graph was used as current state"
        )
    );
    Ok(())
}

#[test]
fn a_forged_loaded_binding_fails_every_loading_fixture() -> crate::verdicts::ConformanceResult<()> {
    let report = run(&[wrapper::Loophole::ForgedBinding])?;
    let expected = forged_binding_failures(&report);
    assert_eq!(
        expected.len(),
        report.outcomes.len().saturating_sub(NON_LOADING.len())
    );
    assert_eq!(super::failures(&report), expected);
    Ok(())
}
