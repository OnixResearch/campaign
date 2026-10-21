//! Abstract scenario vocabulary owned by the suite.
//!
//! Scenarios speak only in handles, abstract costs, ordered integer score levels, explicit draws,
//! and outcomes. Adapters map this vocabulary to consumer Campaign records; decisions never depend
//! on an adapter's identity tie-breaks because every compared score level is unique.

/// Number of rank dimensions every standard abstract score carries; declared exploration and
/// retention orders name only these. Level-count scenarios present scores of other lengths, each
/// length in its own campaign.
pub const SCENARIO_RANK_DIMENSIONS: u32 = 2;

/// Float-view denominator: level `n` is presented to float consumers as `n / 4`.
const FLOAT_VIEW_DENOMINATOR: f64 = 4.0;

/// Abstract exploration moment handle (root or discovered child).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MomentHandle(pub u32);

/// Abstract operation (candidate) handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationHandle(pub u32);

/// One abstract moment: a root when `parent` is `None`, otherwise a child discovered below
/// `parent`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScenarioMoment {
    /// Moment handle.
    pub handle: MomentHandle,
    /// Parent moment of a discovered child.
    pub parent: Option<MomentHandle>,
}

/// Abstract cost in one suite resource dimension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbstractCost {
    /// Index into the adapter's declared resource dimensions.
    pub dimension: u32,
    /// Positive abstract units the operation reserves.
    pub units: u64,
}

/// One abstract operation below a known moment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioOperation {
    /// Operation handle.
    pub handle: OperationHandle,
    /// Moment the operation starts from.
    pub parent: MomentHandle,
    /// Abstract costs, one per used dimension.
    pub costs: alloc::vec::Vec<AbstractCost>,
}

/// Ordered abstract score: one level per rank dimension, higher is better.
///
/// Adapters must map every dimension with a strictly monotone conversion, so equal levels give
/// equal ranks and a higher level gives a higher rank.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractScore {
    /// Levels in declared rank-dimension order.
    pub levels: alloc::vec::Vec<u32>,
}

impl AbstractScore {
    /// Float view of the score for consumers whose domain score is floating point.
    #[must_use]
    pub fn float_view(&self) -> alloc::vec::Vec<f64> {
        self.levels
            .iter()
            .map(|level| f64::from(*level) / FLOAT_VIEW_DENOMINATOR)
            .collect()
    }
}

/// One abstract preference (guidance) for an admitted operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioPreference {
    /// Operation the preference ranks.
    pub operation: OperationHandle,
    /// Consumer guidance generation.
    pub generation: u64,
    /// Ordered abstract score.
    pub score: AbstractScore,
}

/// Explicit entropy draws supplied by the scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScenarioDraw {
    /// Explore-branch draw (ticket bytes `0..8`).
    pub explore_draw: u64,
    /// Explore-index draw (ticket bytes `8..16`).
    pub index_draw: u64,
}

/// Abstract consumer reason the adapter maps to a bounded `Reason` identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioReason {
    /// Consumer retirement of one candidate.
    Retirement,
    /// Declared retention pruning.
    Pruning,
    /// Explicit guidance revision.
    Revision,
    /// Unobserved closure of one published selection.
    Closure,
    /// Consumer-requested stop.
    ConsumerStop,
}

/// Observed abstract outcome of one executed selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioOutcome {
    /// Child moments the execution discovered (already admitted as exploration events).
    pub children: alloc::vec::Vec<MomentHandle>,
    /// Number of opaque finding references the consumer reports.
    pub findings: u32,
}
