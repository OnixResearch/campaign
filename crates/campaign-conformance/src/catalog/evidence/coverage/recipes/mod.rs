//! Canonical recipes: for every declared class, the admission configuration and the abstract
//! control-event steps that reach it through the adapter, holding every event sub-kind the class
//! is compatible with (`kinds::compatible`). Steps name suite operation handles and choose among
//! candidates only in suite operation order, never by candidate identity.
//!
//! Every recipe first plans the frontier stop of its bare admission (a ranked and an exploratory
//! plan receipt of a planner stop it never records). A recipe with candidates admits one batch of
//! three operations and a batch of a fourth with a coalesced duplicate; a guided recipe discovers
//! two operations, strictly improves the first, updates it at equal ranks (coalesced), and revises
//! it after that update; a selecting recipe publishes a ranked and an exploratory selection among
//! its selections, settles one per compatible result sub-kind (plain, child, finding, both) and
//! closure, and leaves the rest outstanding; then it prunes an explicit list (the last retained
//! candidate, or every one when the class neither retains nor retires any), retires by consumer
//! and by policy, retires every remaining candidate when the class retains none, and records its
//! stop. A frontier stop retires the guided candidates (or
//! prunes them, when it retires none) before it prunes.

pub(in crate::catalog::evidence) mod replay;
#[cfg(test)]
mod tests;

/// How a recipe settles one of its selections.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) enum Settlement {
    /// A result with neither child moments nor findings.
    Plain,
    /// A result reporting a new child moment.
    Child,
    /// A result reporting a finding.
    Finding,
    /// A result reporting a new child moment and a finding.
    Both,
    /// An unobserved closure.
    Closure,
    /// Left outstanding.
    Outstanding,
}

/// One abstract step: each publishes one control event (a settled selection two, the selection
/// and its settlement; `RetireRemaining` one per retained candidate).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) enum Step {
    /// Admits the first three recipe operations, every one new.
    Batch,
    /// Admits the fourth recipe operation and names the first again (coalesced).
    CoalescedBatch,
    /// The first guidance of the first or the second recipe operation.
    Discover { is_second: bool },
    /// A strict improvement of the first operation's guidance.
    Improve,
    /// An equal-rank update of the first operation's guidance at its next generation.
    CoalescedGuidance,
    /// An explicit revision of the first operation's guidance after its coalesced update.
    Revise,
    /// Plans and publishes a selection, then settles it (or leaves it outstanding).
    Select {
        is_exploratory: bool,
        settlement: Settlement,
    },
    /// Prunes an explicit caller list of retained candidates without outstanding selections.
    Prune(Pruned),
    /// Retires the first retained operation in suite order, by consumer or by policy.
    Retire { is_policy: bool },
    /// Retires every remaining retained operation by consumer (one event each).
    RetireRemaining,
    /// Records the class's stop (a planner stop as planned).
    Stop(super::features::Status),
}

impl Step {
    /// Control events the step publishes before the stop of a recipe with nothing outstanding:
    /// two for a settled selection, none for the stop and for `RetireRemaining` (counted apart),
    /// one otherwise.
    const fn events_before_stop(self) -> u64 {
        match self {
            Self::Stop(_) | Self::RetireRemaining => 0,
            Self::Select {
                settlement: Settlement::Outstanding,
                ..
            } => 1,
            Self::Select { .. } => 2,
            _ => 1,
        }
    }
}

/// The candidates an explicit prune names, chosen in suite operation order: the last retained
/// candidate without outstanding work, every guided retained one, or every retained one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) enum Pruned {
    Last,
    Guided,
    All,
}

/// Admission configuration of a recipe: the budget and the bounds it narrows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::catalog::evidence) struct Config {
    pub(in crate::catalog::evidence) budget_units: u64,
    pub(in crate::catalog::evidence) max_selections: Option<u64>,
    pub(in crate::catalog::evidence) max_control_events: Option<u64>,
    pub(in crate::catalog::evidence) max_guidance_records: Option<u64>,
}

/// One canonical recipe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) struct Recipe {
    pub(in crate::catalog::evidence) config: Config,
    pub(in crate::catalog::evidence) steps: alloc::vec::Vec<Step>,
}

/// Operations a recipe with candidates admits.
const OPERATIONS: u64 = 4;

/// Guidance records the guidance steps count (two discoveries, an improvement, a revision; the
/// coalesced update counts none).
const COUNTED_GUIDANCE: u64 = 4;

/// Most steps of one recipe: two batches, five guidance steps, six selections, two retirements,
/// a prune, the remaining retirements, and a stop.
const MAX_STEPS: usize = 18;

/// The selections a class settles or leaves outstanding: one per compatible settlement sub-kind,
/// at least two, the outstanding ones last.
fn settlements(class: super::features::StateClass) -> alloc::vec::Vec<Settlement> {
    debug_assert!(class.has(super::features::SELECTED));
    let mut chosen: alloc::vec::Vec<Settlement> = [
        (class.has(super::features::RESULTS), Settlement::Plain),
        (class.has(super::features::DISCOVERIES), Settlement::Child),
        (class.has(super::features::FINDINGS), Settlement::Finding),
        (
            class.has(super::features::FINDINGS) && class.has(super::features::DISCOVERIES),
            Settlement::Both,
        ),
        (class.has(super::features::CLOSURES), Settlement::Closure),
        (
            class.has(super::features::OUTSTANDING),
            Settlement::Outstanding,
        ),
    ]
    .into_iter()
    .filter_map(|(is_compatible, settlement)| is_compatible.then_some(settlement))
    .collect();
    debug_assert!(
        !chosen.is_empty(),
        "a selecting class settles or leaves outstanding some selection"
    );
    if let [only] = chosen.as_slice() {
        chosen.push(*only);
    }
    chosen
}

/// The canonical steps of `class`.
fn steps(class: super::features::StateClass) -> alloc::vec::Vec<Step> {
    let has_candidates =
        class.has(super::features::RETAINED | super::features::RETIRED | super::features::PRUNED);
    let is_frontier = class.status == super::features::Status::Frontier;
    let is_retired = class.has(super::features::RETIRED);
    let mut steps = alloc::vec::Vec::with_capacity(MAX_STEPS);
    if has_candidates {
        steps.extend([Step::Batch, Step::CoalescedBatch]);
    }
    if class.has(super::features::GUIDED) {
        steps.extend([
            Step::Discover { is_second: false },
            Step::Discover { is_second: true },
            Step::Improve,
            Step::CoalescedGuidance,
            Step::Revise,
        ]);
    }
    if class.has(super::features::SELECTED) {
        steps.extend(
            settlements(class)
                .into_iter()
                .enumerate()
                .map(|(position, settlement)| Step::Select {
                    is_exploratory: position == 1,
                    settlement,
                }),
        );
    }
    let retirements = [
        Step::Retire { is_policy: false },
        Step::Retire { is_policy: true },
    ];
    let prune = class.has(super::features::PRUNED).then(|| {
        Step::Prune(if !class.has(super::features::RETAINED) && !is_retired {
            Pruned::All
        } else if is_frontier && !is_retired {
            Pruned::Guided
        } else {
            Pruned::Last
        })
    });
    if is_frontier {
        if is_retired {
            steps.extend(retirements);
        }
        steps.extend(prune);
    } else {
        steps.extend(prune);
        if is_retired {
            steps.extend(retirements);
        }
    }
    if is_retired && !class.has(super::features::RETAINED) {
        steps.push(Step::RetireRemaining);
    }
    if class.status != super::features::Status::Running {
        steps.push(Step::Stop(class.status));
    }
    debug_assert!(steps.len() <= MAX_STEPS);
    debug_assert_eq!(steps.first() == Some(&Step::Batch), has_candidates);
    steps
}

/// The canonical recipe of `class`.
// r[impl campaign.conformance.state_coverage]
pub(super) fn recipe(class: super::features::StateClass) -> Recipe {
    let steps = steps(class);
    let selection_count: u64 = steps
        .iter()
        .map(|step| u64::from(matches!(step, Step::Select { .. })))
        .sum();
    let status = class.status;
    debug_assert_eq!(selection_count > 0, class.has(super::features::SELECTED));
    debug_assert!(
        status == super::features::Status::Running || steps.last() == Some(&Step::Stop(status)),
        "a stopped class's recipe ends with its stop"
    );
    let config = Config {
        budget_units: if status == super::features::Status::Budget {
            selection_count
        } else {
            super::super::super::common::AMPLE_BUDGET
        },
        max_selections: (status == super::features::Status::SelectionLimit)
            .then_some(selection_count),
        max_control_events: (status == super::features::Status::ControlEventLimit)
            .then(|| control_events_before_stop(class, &steps).saturating_add(2)),
        max_guidance_records: (status == super::features::Status::Frontier
            && class.has(super::features::RETAINED))
        .then_some(COUNTED_GUIDANCE),
    };
    Recipe { config, steps }
}

/// Control events a recipe with nothing outstanding publishes before its stop, the admission
/// included: one per step, two per settled selection, and one per candidate `RetireRemaining`
/// retires (every operation less the two retired ones and the one a prune of the last removed).
fn control_events_before_stop(class: super::features::StateClass, steps: &[Step]) -> u64 {
    let stepped: u64 = steps.iter().map(|step| step.events_before_stop()).sum();
    let remaining = if steps.contains(&Step::RetireRemaining) {
        let pruned = u64::from(class.has(super::features::PRUNED));
        OPERATIONS.saturating_sub(pruned).saturating_sub(2)
    } else {
        0
    };
    stepped.saturating_add(1).saturating_add(remaining)
}
