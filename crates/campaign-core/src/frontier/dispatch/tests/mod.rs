const BUDGET_AMOUNT: u64 = 2;
const EXISTING_AMOUNT: u64 = 1;
const REQUEST_AMOUNT: u64 = 1;

fn amount(label: &[u8], value: u64) -> crate::model::ResourceAmount {
    crate::model::ResourceAmount {
        dimension_id: crate::identity::derive_id(
            crate::identity::IdKind::ResourceDimension,
            label,
            &[],
        ),
        amount: value,
    }
}

#[test]
fn retains_prior_totals_and_adds_new_dimensions() -> crate::error::CampaignResult<()> {
    let existing = amount(b"existing-budget", EXISTING_AMOUNT);
    let new = amount(b"new-budget", REQUEST_AMOUNT);
    let budgets = [
        amount(b"existing-budget", BUDGET_AMOUNT),
        amount(b"new-budget", BUDGET_AMOUNT),
    ];
    let reserved = super::reserve_costs(
        alloc::vec![existing.clone()],
        &budgets,
        &[existing, new.clone()],
    )?;
    assert_eq!(reserved, alloc::vec![budgets[0].clone(), new]);
    let unchanged = super::reserve_costs(reserved.clone(), &budgets, &[])?;
    assert_eq!(unchanged, reserved);
    let empty = super::reserve_costs(alloc::vec::Vec::new(), &[], &[])?;
    assert!(empty.is_empty());
    Ok(())
}

/// A full reservation is exhausted, never an overflow: the fit test compares with the remainder.
#[test]
// r[verify campaign.budgets.overflow]
fn rejects_missing_and_exhausted_budgets_without_overflow() {
    let cost = amount(b"bounded-budget", REQUEST_AMOUNT);
    assert_eq!(
        super::reserve_costs(alloc::vec::Vec::new(), &[], core::slice::from_ref(&cost)),
        Err(crate::error::CampaignError::BudgetExhausted)
    );
    assert_eq!(
        super::reserve_costs(
            alloc::vec![amount(b"bounded-budget", BUDGET_AMOUNT)],
            &[amount(b"bounded-budget", BUDGET_AMOUNT)],
            core::slice::from_ref(&cost)
        ),
        Err(crate::error::CampaignError::BudgetExhausted)
    );
    assert_eq!(
        super::reserve_costs(
            alloc::vec![amount(b"bounded-budget", u64::MAX)],
            &[amount(b"bounded-budget", u64::MAX)],
            &[cost]
        ),
        Err(crate::error::CampaignError::BudgetExhausted)
    );
}

const FIRST_ORDINAL: u64 = 1;

fn id(kind: crate::identity::IdKind, label: &[u8]) -> crate::identity::Id {
    crate::identity::derive_id(kind, label, &[])
}

fn candidate_with(status: crate::model::CandidateStatus) -> crate::model::CandidateState {
    crate::model::CandidateState {
        candidate: crate::model::CandidateRecord {
            candidate_id: id(crate::identity::IdKind::Candidate, b"dispatch-candidate"),
            campaign_id: id(crate::identity::IdKind::Campaign, b"dispatch-campaign"),
            parent_moment_id: id(crate::identity::IdKind::RootMoment, b"dispatch-root"),
            operation_profile_id: id(crate::identity::IdKind::OperationProfile, b"dispatch-op"),
            operation_schema_id: id(crate::identity::IdKind::OperationSchema, b"dispatch-schema"),
            source_adapter_id: id(crate::identity::IdKind::Adapter, b"dispatch-adapter"),
            depth: FIRST_ORDINAL,
            cost_profile_id: id(crate::identity::IdKind::CostProfile, b"dispatch-cost"),
            declared_costs: alloc::vec::Vec::new(),
            operation_generation_profile_id: id(crate::identity::IdKind::Policy, b"dispatch-gen"),
        },
        status,
        selection_count: 0,
        latest_guidance: None,
    }
}

#[test]
fn replay_selection_requires_a_retained_candidate() {
    assert_eq!(
        super::ensure_candidate_selectable(&candidate_with(
            crate::model::CandidateStatus::Retained
        )),
        Ok(())
    );
    assert_eq!(
        super::ensure_candidate_selectable(&candidate_with(
            crate::model::CandidateStatus::Retired {
                reason_id: id(crate::identity::IdKind::Reason, b"dispatch-retired"),
            }
        )),
        Err(crate::error::CampaignError::CandidateRetired)
    );
    assert_eq!(
        super::ensure_candidate_selectable(&candidate_with(
            crate::model::CandidateStatus::Pruned {
                prune_plan_id: id(crate::identity::IdKind::PrunePlan, b"dispatch-pruned"),
            }
        )),
        Err(crate::error::CampaignError::CandidatePruned)
    );
}

#[test]
// r[verify campaign.budgets.overflow]
fn replay_ordinal_is_the_checked_successor_of_the_selection_count() {
    assert_eq!(super::next_ordinal(0), Ok(FIRST_ORDINAL));
    assert_eq!(
        super::next_ordinal(u64::MAX),
        Err(crate::error::CampaignError::ArithmeticOverflow)
    );
}
