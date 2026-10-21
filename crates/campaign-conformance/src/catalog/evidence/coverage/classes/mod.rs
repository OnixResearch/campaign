//! The declared state classes: every feature vector the Campaign rules below admit. Each
//! constraint is implied by a Campaign rule; the bounded reachability test shows that every state
//! reached from an admission within its bound lies in a declared class, and the recipe replay
//! test shows that every declared class is reached.

/// A declared implication between boolean features: when every premise flag holds, some
/// conclusion flag holds.
#[derive(Clone, Copy)]
struct Implication {
    premise: u16,
    conclusion: u16,
}

impl Implication {
    /// Whether `class` satisfies the implication.
    const fn holds(self, class: super::features::StateClass) -> bool {
        class.flags & self.premise != self.premise || class.has(self.conclusion)
    }
}

/// Every flag of a candidate status: retained, retired, or pruned.
const CANDIDATES: u16 =
    super::features::RETAINED | super::features::RETIRED | super::features::PRUNED;

/// The implications every declared class satisfies, whatever its status.
const IMPLICATIONS: [Implication; 9] = [
    Implication {
        premise: super::features::SELECTED,
        conclusion: super::features::GUIDED,
    },
    Implication {
        premise: super::features::OUTSTANDING,
        conclusion: super::features::SELECTED,
    },
    Implication {
        premise: super::features::RESULTS,
        conclusion: super::features::SELECTED,
    },
    Implication {
        premise: super::features::CLOSURES,
        conclusion: super::features::SELECTED,
    },
    Implication {
        premise: super::features::SELECTED,
        conclusion: super::features::OUTSTANDING
            | super::features::RESULTS
            | super::features::CLOSURES,
    },
    Implication {
        premise: super::features::FINDINGS,
        conclusion: super::features::RESULTS,
    },
    Implication {
        premise: super::features::DISCOVERIES,
        conclusion: super::features::RESULTS,
    },
    Implication {
        premise: super::features::GUIDED,
        conclusion: CANDIDATES,
    },
    Implication {
        premise: super::features::OUTSTANDING,
        conclusion: super::features::RETAINED | super::features::RETIRED,
    },
];

/// The implications a frontier stop adds: a retained candidate is guided, and a guided frontier
/// stop has a retired or pruned candidate.
const FRONTIER_IMPLICATIONS: [Implication; 2] = [
    Implication {
        premise: super::features::RETAINED,
        conclusion: super::features::GUIDED,
    },
    Implication {
        premise: super::features::GUIDED,
        conclusion: super::features::RETIRED | super::features::PRUNED,
    },
];

/// Whether `class` satisfies every implication of `implications`.
const fn satisfies(class: super::features::StateClass, implications: &[Implication]) -> bool {
    let mut remaining = implications;
    while let [implication, rest @ ..] = remaining {
        if !implication.holds(class) {
            return false;
        }
        remaining = rest;
    }
    true
}

/// Whether the Campaign rules admit a state with these features.
///
/// - A selection is planned only for an eligible candidate, which carries guidance
///   (`campaign.selection.valid`): selected implies guided.
/// - Outstanding selections, results, and closures belong to published selections
///   (`campaign.results`, `campaign.closures`): each implies selected; every published selection
///   is outstanding or settled by a result or a closure, so selected implies one of them.
/// - Findings and discovered child moments arrive in accepted results (`campaign.findings`,
///   `campaign.results.children`): each implies results.
/// - Guidance names an admitted candidate and candidates never leave the state
///   (`campaign.guidance`): guided implies a retained, retired, or pruned candidate.
/// - Pruning (declared or an explicit caller list) skips candidates with outstanding selections
///   (`campaign.pruning`): outstanding implies a retained or retired candidate.
/// - The planner computes the frontier, budget, selection-limit, and control-event-limit stops
///   only with nothing outstanding, and no selection follows a stop (`campaign.stop`): each
///   implies not outstanding.
/// - The selection bound is at least one (`campaign.stop.selection_limit`): the selection-limit
///   stop implies selected.
/// - Intake bounds every cost by the budget, so only reservations make a retained candidate's
///   cost exceed what is left (`campaign.stop.budget`): the budget stop implies selected and
///   retained.
/// - At the frontier stop every retained candidate is unguided while the guidance-record bound
///   (at least one) is used up (`campaign.stop.frontier`, `campaign.wait.guidance_room`): the
///   frontier stop with a retained candidate implies guided; a retained guided candidate is
///   eligible (or its cost exceeds the budget, a budget stop), and guidance is never removed, so
///   a guided frontier stop implies a retired or pruned candidate.
// r[impl campaign.conformance.state_coverage]
pub(in crate::catalog::evidence) const fn admits(class: super::features::StateClass) -> bool {
    debug_assert!(
        class.flags >> super::features::FLAGS.len() == 0,
        "a class sets only declared flags"
    );
    let status = class.status;
    let is_stop_consistent = if matches!(status, super::features::Status::Frontier) {
        satisfies(class, &FRONTIER_IMPLICATIONS)
    } else if matches!(status, super::features::Status::Budget) {
        class.has(super::features::SELECTED) && class.has(super::features::RETAINED)
    } else if matches!(status, super::features::Status::SelectionLimit) {
        class.has(super::features::SELECTED)
    } else {
        true
    };
    let is_settled = !status.is_planned() || !class.has(super::features::OUTSTANDING);
    satisfies(class, &IMPLICATIONS) && is_stop_consistent && is_settled
}

/// Every declared class, in code order.
pub(in crate::catalog::evidence) fn all() -> alloc::vec::Vec<super::features::StateClass> {
    let classes: alloc::vec::Vec<super::features::StateClass> = super::features::STATUSES
        .into_iter()
        .flat_map(|status| {
            (0..(1_u16 << super::features::FLAGS.len()))
                .map(move |flags| super::features::StateClass { status, flags })
        })
        .filter(|class| admits(*class))
        .collect();
    debug_assert!(classes.is_sorted_by_key(|class| class.code()));
    classes
}
