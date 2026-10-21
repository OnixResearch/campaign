//! Generated campaign shapes and control scripts. Every value is interpreted through public APIs.

/// Largest generated root set.
pub(crate) const MAX_ROOTS: u64 = 3;
/// Largest generated resource-dimension count.
pub(crate) const MAX_DIMENSIONS: usize = 3;
/// Generated per-dimension budget range.
pub(crate) const MIN_BUDGET: u64 = 2;
pub(crate) const MAX_BUDGET: u64 = 12;
/// Largest generated per-dimension candidate cost; zero leaves the dimension undeclared.
pub(crate) const MAX_COST: u64 = 3;
/// Largest generated guidance rank dimension count.
pub(crate) const MAX_RANK_DIMENSIONS: usize = 3;
/// Largest candidate draft count in one scripted batch.
pub(crate) const MAX_DRAFTS: usize = 3;
/// Largest first-rank lift of one scripted guidance update; zero resubmits the latest record.
pub(crate) const MAX_LIFT: u8 = 4;
/// Scripted step count range.
pub(crate) const MIN_STEPS: usize = 8;
pub(crate) const MAX_STEPS: usize = 40;
/// Generated bound ranges, sized so scripts reach every bound.
const OUTSTANDING_RANGE: core::ops::RangeInclusive<u64> = 1..=3;
const CANDIDATE_RANGE: core::ops::RangeInclusive<u64> = 3..=10;
const BATCH_RANGE: core::ops::RangeInclusive<u64> = 1..=4;
const SELECTION_RANGE: core::ops::RangeInclusive<u64> = 2..=10;
const GUIDANCE_RANGE: core::ops::RangeInclusive<u64> = 3..=12;
const DEPTH_RANGE: core::ops::RangeInclusive<u64> = 1..=3;
const CONTROL_EVENT_RANGE: core::ops::RangeInclusive<u64> = 10..=48;
/// Weights of scripted step kinds.
const ADMIT_WEIGHT: u32 = 3;
const GUIDE_WEIGHT: u32 = 4;
const SELECT_WEIGHT: u32 = 5;
const SETTLE_WEIGHT: u32 = 3;

/// Permutation key count; covers every generated collection.
pub(crate) const KEY_COUNT: usize = 16;

/// One scripted candidate: a pick among known moments and one cost per budget dimension.
#[derive(Clone, Debug)]
pub(crate) struct Draft {
    pub(crate) parent_pick: u8,
    pub(crate) costs: Vec<u64>,
}

/// One scripted control step.
#[derive(Clone, Debug)]
pub(crate) enum Step {
    /// Admit fresh drafts; `resubmit` also resubmits one admitted candidate under `Coalesce`.
    Admit {
        drafts: Vec<Draft>,
        resubmit: Option<u8>,
    },
    /// First guidance, or a relaxation of the latest record.
    Guide(GuideStep),
    /// Plan ranked (`None`) or exploratory with explicit draws, then publish any selection.
    Select {
        draws: Option<crate::suite::requests::Draws>,
    },
    /// Settle one outstanding selection with an exact one-child result.
    Settle { pick: u8 },
}

/// First guidance with `ranks`, or a relaxation lifting the latest first rank by `lift`
/// (zero resubmits the latest record).
#[derive(Clone, Debug)]
pub(crate) struct GuideStep {
    pub(crate) pick: u8,
    pub(crate) ranks: Vec<i64>,
    pub(crate) lift: u8,
}

/// Exploratory profile facts shared by every exploratory step of one script.
#[derive(Clone, Debug)]
pub(crate) struct Exploration {
    pub(crate) threshold: u64,
    pub(crate) order: Vec<u32>,
}

/// One generated campaign and its scripted control steps.
#[derive(Clone, Debug)]
pub(crate) struct Script {
    pub(crate) limits: crate::suite::fixture::Limits,
    pub(crate) roots: u64,
    pub(crate) budgets: Vec<u64>,
    pub(crate) exploration: Exploration,
    pub(crate) steps: Vec<Step>,
}

pub(crate) fn limits() -> impl proptest::strategy::Strategy<Value = crate::suite::fixture::Limits> {
    proptest::strategy::Strategy::prop_map(
        (
            OUTSTANDING_RANGE,
            CANDIDATE_RANGE,
            BATCH_RANGE,
            SELECTION_RANGE,
            GUIDANCE_RANGE,
            DEPTH_RANGE,
            CONTROL_EVENT_RANGE,
            1..=MAX_RANK_DIMENSIONS,
        ),
        |values| crate::suite::fixture::Limits {
            max_outstanding: values.0,
            max_candidates: values.1,
            max_candidate_batch: values.2,
            max_selections: values.3,
            max_guidance_records: values.4,
            max_depth: values.5,
            max_control_events: values.6,
            rank_dimensions: values.7,
        },
    )
}

/// Smallest generated rank.
const RANK_FLOOR: i64 = crate::suite::fixture::RANK_LIMIT.saturating_neg();

pub(crate) fn ranks(rank_count: usize) -> impl proptest::strategy::Strategy<Value = Vec<i64>> {
    proptest::collection::vec(RANK_FLOOR..=crate::suite::fixture::RANK_LIMIT, rank_count)
}

/// Midpoint of the entropy scale; a generated boundary for thresholds and explore draws.
const ENTROPY_HALF: u64 = campaign_core::ENTROPY_SCALE / 2;
/// Largest scale multiple added to boundary explore draws without overflow.
const MAX_SCALE_MULTIPLE: u64 = (u64::MAX / campaign_core::ENTROPY_SCALE).saturating_sub(1);
/// Largest residue of an explore draw.
const ENTROPY_LAST: u64 = campaign_core::ENTROPY_SCALE.saturating_sub(1);
/// Residues adjacent to the midpoint.
const ENTROPY_BELOW_HALF: u64 = ENTROPY_HALF.saturating_sub(1);
const ENTROPY_ABOVE_HALF: u64 = ENTROPY_HALF.saturating_add(1);

/// Thresholds weighted toward the branch boundaries.
pub(crate) fn threshold() -> impl proptest::strategy::Strategy<Value = u64> {
    proptest::prop_oneof![
        proptest::strategy::Just(0),
        proptest::strategy::Just(1),
        proptest::strategy::Just(ENTROPY_HALF),
        proptest::strategy::Just(ENTROPY_LAST),
        proptest::strategy::Just(campaign_core::ENTROPY_SCALE),
        0..=campaign_core::ENTROPY_SCALE,
    ]
}

/// Explore draws: arbitrary, or a scale multiple plus a residue at a generated threshold.
pub(crate) fn explore_draw() -> impl proptest::strategy::Strategy<Value = u64> {
    let residue = proptest::prop_oneof![
        proptest::strategy::Just(0),
        proptest::strategy::Just(1),
        proptest::strategy::Just(ENTROPY_BELOW_HALF),
        proptest::strategy::Just(ENTROPY_HALF),
        proptest::strategy::Just(ENTROPY_ABOVE_HALF),
        proptest::strategy::Just(ENTROPY_LAST),
    ];
    proptest::prop_oneof![
        proptest::num::u64::ANY,
        proptest::strategy::Strategy::prop_map(
            (0..=MAX_SCALE_MULTIPLE, residue),
            |(multiple, residue)| {
                debug_assert!(residue < campaign_core::ENTROPY_SCALE);
                debug_assert!(multiple <= MAX_SCALE_MULTIPLE);
                multiple
                    .saturating_mul(campaign_core::ENTROPY_SCALE)
                    .saturating_add(residue)
            },
        ),
    ]
}

pub(crate) fn draws() -> impl proptest::strategy::Strategy<Value = crate::suite::requests::Draws> {
    proptest::strategy::Strategy::prop_map(
        (explore_draw(), proptest::num::u64::ANY),
        |(explore, index)| crate::suite::requests::Draws { explore, index },
    )
}

/// A non-empty, unique, shuffled selection of declared rank dimensions.
pub(crate) fn exploration_order(
    rank_count: usize,
) -> impl proptest::strategy::Strategy<Value = Vec<u32>> {
    let declared: Vec<u32> = (0_u32..).take(rank_count).collect();
    let width = declared.len();
    proptest::strategy::Strategy::prop_shuffle(proptest::sample::subsequence(declared, 1..=width))
}

fn exploration(rank_count: usize) -> impl proptest::strategy::Strategy<Value = Exploration> {
    proptest::strategy::Strategy::prop_map(
        (threshold(), exploration_order(rank_count)),
        |(threshold, order)| Exploration { threshold, order },
    )
}

fn draft(dimensions: usize) -> impl proptest::strategy::Strategy<Value = Draft> {
    proptest::strategy::Strategy::prop_map(
        (
            proptest::num::u8::ANY,
            proptest::collection::vec(0..=MAX_COST, dimensions),
        ),
        |(parent_pick, costs)| Draft { parent_pick, costs },
    )
}

/// Widths every scripted step of one campaign shares.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StepShape {
    dimensions: usize,
    rank_count: usize,
}

pub(crate) fn step(shape: StepShape) -> impl proptest::strategy::Strategy<Value = Step> {
    let StepShape {
        dimensions,
        rank_count,
    } = shape;
    debug_assert!(dimensions > 0);
    debug_assert!(rank_count > 0);
    proptest::prop_oneof![
        ADMIT_WEIGHT => proptest::strategy::Strategy::prop_map(
            (
                proptest::collection::vec(draft(dimensions), 1..=MAX_DRAFTS),
                proptest::option::of(proptest::num::u8::ANY),
            ),
            |(drafts, resubmit)| Step::Admit { drafts, resubmit },
        ),
        GUIDE_WEIGHT => proptest::strategy::Strategy::prop_map(
            (proptest::num::u8::ANY, ranks(rank_count), 0..=MAX_LIFT),
            |(pick, ranks, lift)| Step::Guide(GuideStep { pick, ranks, lift }),
        ),
        SELECT_WEIGHT => proptest::strategy::Strategy::prop_map(
            proptest::option::of(draws()),
            |draws| Step::Select { draws },
        ),
        SETTLE_WEIGHT => proptest::strategy::Strategy::prop_map(
            proptest::num::u8::ANY,
            |pick| Step::Settle { pick },
        ),
    ]
}

/// Campaign shape: bounds, root count, and one budget per resource dimension.
type Shape = (crate::suite::fixture::Limits, u64, Vec<u64>);

fn shape() -> impl proptest::strategy::Strategy<Value = Shape> {
    (
        limits(),
        1..=MAX_ROOTS,
        proptest::collection::vec(MIN_BUDGET..=MAX_BUDGET, 1..=MAX_DIMENSIONS),
    )
}

pub(crate) fn script() -> impl proptest::strategy::Strategy<Value = Script> {
    let scripted =
        proptest::strategy::Strategy::prop_flat_map(shape(), |(limits, roots, budgets)| {
            let dimensions = budgets.len();
            let rank_count = limits.rank_dimensions;
            debug_assert!((1..=MAX_DIMENSIONS).contains(&dimensions));
            debug_assert!((1..=MAX_RANK_DIMENSIONS).contains(&rank_count));
            (
                proptest::strategy::Just((limits, roots, budgets)),
                exploration(rank_count),
                proptest::collection::vec(
                    step(StepShape {
                        dimensions,
                        rank_count,
                    }),
                    MIN_STEPS..=MAX_STEPS,
                ),
            )
        });
    proptest::strategy::Strategy::prop_map(
        scripted,
        |((limits, roots, budgets), exploration, steps)| {
            debug_assert!(!exploration.order.is_empty());
            debug_assert!(steps.len() >= MIN_STEPS);
            Script {
                limits,
                roots,
                budgets,
                exploration,
                steps,
            }
        },
    )
}

/// Random sort keys that drive one permutation.
pub(crate) fn keys() -> impl proptest::strategy::Strategy<Value = Vec<u32>> {
    proptest::collection::vec(proptest::num::u32::ANY, KEY_COUNT)
}

/// `items` reordered by `keys` (position order breaks equal keys).
pub(crate) fn permute<T: Clone>(items: &[T], keys: &[u32]) -> Vec<T> {
    let mut keyed: Vec<(u32, usize, &T)> = items
        .iter()
        .enumerate()
        .map(|(position, item)| (keys.get(position).copied().unwrap_or(0), position, item))
        .collect();
    keyed.sort_by_key(|(key, position, _)| (*key, *position));
    keyed.into_iter().map(|(_, _, item)| item.clone()).collect()
}
