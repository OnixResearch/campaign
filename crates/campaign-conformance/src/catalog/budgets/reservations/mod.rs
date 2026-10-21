//! Costs in every declared resource dimension, above `u32::MAX` units, above the largest
//! integer `f64` represents exactly, and just below `u64::MAX`.

/// Units above `u32::MAX`.
const WIDE_UNITS: u64 = 1 << 33;

/// Units above `2^53` that `f64` cannot represent (`2^53 + 1`).
const INEXACT_UNITS: u64 = (1 << 53) | 1;

/// One unit family: the units of the cost at a small offset `d`, if representable.
type UnitFamily = fn(u64) -> Option<u64>;

/// Unit families of the generated costs, each offset by a small distinct amount per cost:
/// `2^33 + d` (above `u32::MAX`), `2^53 + 1 + d` (not representable in `f64` for even `d`, and
/// never through a 48-bit integer), and `u64::MAX - d` (only a full-width unsigned mapping keeps
/// them). A mapping through any narrower integer or through `f64` changes a declared and
/// reserved amount.
const UNIT_FAMILIES: [UnitFamily; 3] = [
    |offset_units| WIDE_UNITS.checked_add(offset_units),
    |offset_units| INEXACT_UNITS.checked_add(offset_units),
    |offset_units| u64::MAX.checked_sub(offset_units),
];

/// Budget of every declared dimension: fits every generated cost.
const WIDE_BUDGET: u64 = u64::MAX;

/// Handle of the costed operation of each scenario.
const COSTED: crate::vocabulary::OperationHandle = crate::vocabulary::OperationHandle(21);

/// Generated cost vectors over `dimensions` declared dimensions, for every unit family: one cost
/// in each dimension alone, then (with two or more dimensions) one cost in every dimension at
/// once. Every generated cost of a family has its own amount, so a mapping that moves units
/// between dimensions changes the declaration.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` when a dimension index or amount is not representable.
fn cost_vectors(
    dimensions: usize,
) -> crate::verdicts::ConformanceResult<
    alloc::vec::Vec<alloc::vec::Vec<crate::vocabulary::AbstractCost>>,
> {
    let mut vectors = alloc::vec::Vec::with_capacity(
        dimensions
            .saturating_add(1)
            .saturating_mul(UNIT_FAMILIES.len()),
    );
    for family in UNIT_FAMILIES {
        vectors.extend(family_vectors(dimensions, family)?);
    }
    debug_assert!(vectors.len() >= dimensions.saturating_mul(UNIT_FAMILIES.len()));
    Ok(vectors)
}

/// Cost vectors of one unit family; see `cost_vectors`.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` when a dimension index or amount is not representable.
fn family_vectors(
    dimensions: usize,
    family: UnitFamily,
) -> crate::verdicts::ConformanceResult<
    alloc::vec::Vec<alloc::vec::Vec<crate::vocabulary::AbstractCost>>,
> {
    let count =
        u32::try_from(dimensions).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let units = |offset_units: u32| {
        family(u64::from(offset_units)).ok_or(campaign_core::CampaignError::ArithmeticOverflow)
    };
    let mut vectors = alloc::vec::Vec::with_capacity(dimensions.saturating_add(1));
    for dimension in 0..count {
        vectors.push(alloc::vec![crate::vocabulary::AbstractCost {
            dimension,
            units: units(dimension)?,
        }]);
    }
    if count >= 2 {
        let mut every = alloc::vec::Vec::with_capacity(dimensions);
        for dimension in 0..count {
            every.push(crate::vocabulary::AbstractCost {
                dimension,
                units: units(
                    count
                        .checked_add(dimension)
                        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
                )?,
            });
        }
        vectors.push(every);
    }
    debug_assert!(vectors.len() >= dimensions);
    Ok(vectors)
}

/// The exact Campaign amounts one abstract cost vector stands for, in canonical dimension order.
///
/// # Errors
///
/// Returns `InvalidIdentities` for a dimension index the adapter never declared.
fn expected_amounts(
    dimension_ids: &[campaign_core::Id],
    costs: &[crate::vocabulary::AbstractCost],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::ResourceAmount>> {
    let mut amounts = alloc::vec::Vec::with_capacity(costs.len());
    for cost in costs {
        let index = usize::try_from(cost.dimension)
            .map_err(|_| crate::verdicts::ConformanceError::InvalidIdentities)?;
        amounts.push(campaign_core::ResourceAmount {
            dimension_id: dimension_ids
                .get(index)
                .copied()
                .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?,
            amount: cost.units,
        });
    }
    amounts.sort_by_key(|amount| amount.dimension_id);
    debug_assert_eq!(amounts.len(), costs.len());
    Ok(amounts)
}

/// Presents one operation per generated cost vector (each declared dimension alone, and every
/// dimension at once when the adapter declares several) in every unit family (`2^33 + d`,
/// `2^53 + 1 + d`, `u64::MAX - d`) under a `u64::MAX` budget, and requires the adapter's
/// candidate to declare exactly those amounts in exactly those dimensions and the executed
/// selection to reserve exactly them.
pub(super) fn declared_dimensions(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let dimensions = harness.adapter.identities()?.resource_dimension_ids.len();
    for costs in cost_vectors(dimensions)? {
        let mut session = crate::harness::Session::open(harness, WIDE_BUDGET, |_| {})?;
        let expected = expected_amounts(&session.identities.resource_dimension_ids, &costs)?;
        session.admit(&[crate::vocabulary::ScenarioOperation {
            handle: COSTED,
            parent: crate::harness::history::ROOT,
            costs,
        }])?;
        let candidate = session.candidate_id(COSTED)?;
        let mut declared = session
            .state
            .candidate(candidate)?
            .candidate
            .declared_costs
            .clone();
        declared.sort_by_key(|amount| amount.dimension_id);
        super::super::common::require(
            declared == expected,
            "the adapter declares every abstract cost in its own dimension with its exact units",
        )?;
        session.guide(&super::super::common::preference(
            COSTED,
            super::super::common::FIRST_GENERATION,
            super::super::common::LEVELS_A,
        ))?;
        session.execute(None)?;
        let mut reserved = session.state.reserved_resources.clone();
        reserved.sort_by_key(|amount| amount.dimension_id);
        super::super::common::require(
            reserved == expected,
            "a selection reserves exactly its declared cost in every dimension",
        )?;
        debug_assert!(
            expected
                .iter()
                .all(|amount| amount.amount > u64::from(u32::MAX))
        );
    }
    Ok(())
}
