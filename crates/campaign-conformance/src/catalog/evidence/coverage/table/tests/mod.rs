//! Freshness of the pinned covering table.

/// Combinations every declared class carries with its compatible receipt kinds.
fn required(classes: &[super::super::features::StateClass]) -> super::super::combos::Carried {
    let presented: alloc::vec::Vec<(super::super::features::StateClass, u32)> = classes
        .iter()
        .map(|class| (*class, super::super::kinds::compatible(*class)))
        .collect();
    super::super::combos::Carried::of(&presented)
}

#[test]
// r[verify campaign.conformance.state_coverage]
fn the_pinned_selection_is_the_greedy_covering_of_the_declared_classes() {
    let recomputed = super::greedy::covering(&super::super::classes::all());
    assert_eq!(recomputed, super::TABLE.to_vec());
}

#[test]
// r[verify campaign.conformance.state_coverage]
fn the_pinned_selection_carries_every_declared_combination()
-> crate::verdicts::ConformanceResult<()> {
    let all = super::super::classes::all();
    let selected = super::selected()?;
    assert_eq!(required(&selected).missing(&required(&all)), (0, 0));
    Ok(())
}
