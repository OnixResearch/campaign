//! `campaign.conformance.negative`: every broken wrapper fails exactly the named fixtures that
//! declare its breakage, each through its declared verdict, and the healthy wrapper passes every
//! fixture (`campaign.conformance.positive`).

mod wrapper;

/// Fixture name and declared forbidden effect of one negative fixture.
type Declared = (&'static str, &'static str);

const STALE_HISTORY: [Declared; 3] = [
    (
        "caches.stale",
        "a cache of a stale head was used as current state",
    ),
    (
        "caches.moved_binding",
        "a cache bound to a superseded graph was used as current state",
    ),
    (
        "caches.validation_matrix",
        "a cache of a stale state was used as current state",
    ),
];

const EXECUTION_BEFORE_PUBLICATION: [Declared; 7] = [
    (
        "fencing.unpublished_execution",
        "a planned but unpublished selection was executed",
    ),
    (
        "fencing.non_durable_execution",
        "a selection executed without durable publication",
    ),
    (
        "fencing.stale_branch",
        "stale branch evidence authorized execution",
    ),
    (
        "fencing.foreign_selection",
        "an unpublished selection executed under another selection's durable publication",
    ),
    (
        "fencing.stale_state_selection",
        "a stale-state selection executed under the winner's durable publication",
    ),
    (
        "fencing.gate_matrix_ranked",
        "a selection executed without a publication or evidence",
    ),
    (
        "fencing.gate_matrix_exploratory",
        "a selection executed without a publication or evidence",
    ),
];

const CROSSED_IDENTITIES: [Declared; 1] = [(
    "results.crossed_selection",
    "a result settled a selection other than the executed one",
)];

/// Positive budget fixtures a budget miscount diverges from, with their declared expectations.
const BUDGET_DIVERGENCES: [Declared; 2] = [
    (
        "budgets.reservation",
        "a selection reserves exactly its abstract cost",
    ),
    (
        "budgets.declared_dimensions",
        "the adapter declares every abstract cost in its own dimension with its exact units",
    ),
];

const BUDGET_MISCOUNT: [Declared; 1] = [(
    "budgets.overdraw",
    "a selection overdrew the abstract budget",
)];

const HISTORY_REWRITE: [Declared; 4] = [
    (
        "history.rewritten_payload",
        "a rewritten control payload was loaded as history",
    ),
    (
        "history.altered_graph",
        "history altered after binding was loaded as current",
    ),
    (
        "history.ambiguous_lineage",
        "an ambiguous control lineage was loaded as history",
    ),
    (
        "history.load_matrix",
        "a rewound control branch was loaded as current history",
    ),
];

const RECEIPT_OVERCLAIM: [Declared; 4] = [
    (
        "receipts.promotion_overclaim",
        "rank, selection, exit status, or a check reference became promotion authority",
    ),
    (
        "receipts.non_selection_overclaim",
        "a campaign admission event receipt carried a promotion claim",
    ),
    (
        "receipts.planner_overclaim",
        "a campaign admission event receipt carried a promotion claim",
    ),
    (
        "receipts.recorded_stop_overclaim",
        "a campaign admission event receipt carried a promotion claim",
    ),
];

const ROLE_COLLAPSE: [Declared; 1] = [(
    "roles.collapsed_identity",
    "the adapter stored candidate, work item, and result as one record",
)];

/// Runs the suite against one wrapper and returns every fixture that did not pass, with its
/// verdict.
fn failures(
    breakage: wrapper::Breakage,
) -> crate::verdicts::ConformanceResult<
    alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>,
> {
    let profile = super::default_profile()?;
    let mut adapter = wrapper::FaultyAdapter::new(breakage);
    Ok(super::failures(&crate::catalog::run_suite(
        &mut adapter,
        &profile,
    )))
}

/// Negative fixtures whose declared forbidden input or effect went through.
fn let_through(
    declared: &[Declared],
) -> alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)> {
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

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(wrapper::Breakage::Healthy)?, let_through(&[]));
    Ok(())
}

#[test]
// r[verify campaign.conformance.negative]
fn stale_history_fails_the_cache_fixtures() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(wrapper::Breakage::StaleHistory)?,
        crate::tests::state_coverage_let_through(
            let_through(&STALE_HISTORY),
            "a cache of a held state's lineage on an older graph was used as current state"
        )
    );
    Ok(())
}

#[test]
// r[verify campaign.conformance.negative]
fn execution_before_publication_fails_the_fence_fixtures() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(
        failures(wrapper::Breakage::ExecutionBeforePublication)?,
        let_through(&EXECUTION_BEFORE_PUBLICATION)
    );
    Ok(())
}

/// Crossed identities settle another selection in the older-first order and, in the newer-first
/// order, name the other selection with children reserved for the executed one, which Campaign
/// refuses there and at the state coverage's child-reporting fork heads.
#[test]
// r[verify campaign.conformance.bounds]
// r[verify campaign.conformance.negative]
fn crossed_identities_fail_the_crossed_selection_fixtures() -> crate::verdicts::ConformanceResult<()>
{
    let mut expected = let_through(&CROSSED_IDENTITIES);
    let refused = crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
        crate::verdicts::ConformanceError::Campaign(
            campaign_core::CampaignError::InvalidChildReservation,
        ),
    );
    expected.push(("results.out_of_order_settlement", refused.clone()));
    expected.push(("history.state_coverage", refused));
    assert_eq!(failures(wrapper::Breakage::CrossedIdentities)?, expected);
    Ok(())
}

#[test]
fn a_budget_miscount_fails_the_budget_fixtures() -> crate::verdicts::ConformanceResult<()> {
    let mut expected: alloc::vec::Vec<_> = BUDGET_DIVERGENCES
        .iter()
        .map(|(name, expected)| {
            (
                *name,
                crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
                    crate::verdicts::ConformanceError::Diverged { expected },
                ),
            )
        })
        .collect();
    expected.extend(let_through(&BUDGET_MISCOUNT));
    assert_eq!(failures(wrapper::Breakage::BudgetMiscount)?, expected);
    Ok(())
}

#[test]
fn a_history_rewrite_fails_the_history_fixtures() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(wrapper::Breakage::HistoryRewrite)?,
        crate::tests::state_coverage_let_through(
            let_through(&HISTORY_REWRITE),
            "a held state's lineage on an older graph was loaded as current"
        )
    );
    Ok(())
}

#[test]
// r[verify campaign.conformance.negative]
fn a_receipt_overclaim_fails_the_promotion_fixtures() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(wrapper::Breakage::ReceiptOverclaim)?,
        crate::tests::state_coverage_let_through(
            let_through(&RECEIPT_OVERCLAIM),
            "a campaign admission event receipt carried a promotion claim"
        )
    );
    Ok(())
}

#[test]
// r[verify campaign.fd.roles.collapsed]
fn a_role_collapse_fails_the_collapsed_identity_fixture() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(
        failures(wrapper::Breakage::RoleCollapse)?,
        let_through(&ROLE_COLLAPSE)
    );
    Ok(())
}
