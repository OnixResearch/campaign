//! The covering rule of the held-lineage matrix. Every holder kind meets a refused rewind, a
//! refused fork of every head kind a base of that kind admits (`admits_head`), a refused fork at
//! each depth from zero to `NAMED_DEPTHS` and at the deepest depth its base admits, a refused
//! history of every catalog class (the forged-fence class needs a selection, so it is presented
//! through every holder whose campaign is running, guided, and below its outstanding bound), and
//! its lineage on an older graph. Every held length meets a refusal and every length but the last
//! (whose graph is the newest) its lineage on an older graph; every length after the root meets a
//! refused fork at depth zero and at its deepest depth and a refused history of every catalog
//! class its head admits; every stopped holder meets a post-stop settlement it loads; and the root
//! meets another campaign's root.

/// Named fork depths every holder kind meets, from zero.
const NAMED_DEPTHS: usize = 3;

/// Whether Campaign refuses one presentation.
fn is_refused(presented: &super::Presented) -> bool {
    !matches!(presented.answer, super::super::Answer::Loads)
}

/// Whether a fork over a base in `state` admits a head of `kind`: after a stop only settlements
/// of outstanding selections; a selection needs room for one more outstanding selection under
/// the profile's bound; a settlement settles an outstanding selection or a selection it
/// publishes first.
fn admits_head(
    kind: super::super::super::lineages::forks::heads::HeadKind,
    state: &campaign_core::FrontierState,
) -> Option<bool> {
    let is_running = state.stopped.is_none();
    let outstanding = u64::try_from(state.outstanding.len()).ok()?;
    let has_selection_room = outstanding < state.admission.bounds.max_outstanding;
    debug_assert!(super::super::super::lineages::forks::heads::HEAD_KINDS.contains(&kind));
    debug_assert!(has_selection_room || outstanding > 0);
    if kind.settles() {
        return Some(outstanding > 0 || (is_running && has_selection_room));
    }
    if kind == super::super::super::lineages::forks::heads::HeadKind::Selection {
        return Some(is_running && has_selection_room);
    }
    Some(is_running)
}

/// The held matrix under the covering rule: its holders and presentations.
pub(super) struct Matrix<'m> {
    holders: &'m [super::Holder],
    presented: &'m [super::Presented],
}

impl Matrix<'_> {
    /// Whether one presentation of a shape `shape` accepts is refused through a holder `holder`
    /// accepts.
    fn is_refused_through(
        &self,
        holder: &dyn Fn(usize, &super::Holder) -> bool,
        shape: &dyn Fn(super::ShapeKind) -> bool,
    ) -> bool {
        self.presented.iter().any(|presentation| {
            is_refused(presentation)
                && shape(presentation.shape)
                && self
                    .holders
                    .get(presentation.holder)
                    .is_some_and(|held| holder(presentation.holder, held))
        })
    }

    /// Whether some holder of `kind` has a base that admits a head of kind `head`.
    pub(super) fn is_head_admitted(
        &self,
        kind: super::HolderKind,
        head: super::super::super::lineages::forks::heads::HeadKind,
    ) -> bool {
        self.holders.iter().any(|held| {
            held.kind == kind
                && held
                    .base
                    .as_ref()
                    .is_some_and(|base| admits_head(head, &base.state).unwrap_or(false))
        })
    }

    /// Whether the holders of `kind` meet every covered shape.
    pub(super) fn covers_kind(&self, kind: super::HolderKind) -> bool {
        let of_kind = |_: usize, held: &super::Holder| held.kind == kind;
        let has_rewind = self.is_refused_through(&of_kind, &|shape| {
            matches!(shape, super::ShapeKind::Rewind(_))
        });
        let has_heads = super::super::super::lineages::forks::heads::HEAD_KINDS
            .iter()
            .all(|head| {
                !self.is_head_admitted(kind, *head)
                    || self.is_refused_through(&of_kind, &|shape| {
                        matches!(shape, super::ShapeKind::Fork { head: forked, .. } if forked == *head)
                    })
            });
        let has_depths = (0..=NAMED_DEPTHS).all(|depth| {
            self.is_refused_through(&of_kind, &|shape| {
                matches!(shape, super::ShapeKind::Fork { depth: forked, .. } if forked == depth)
            })
        }) && self.is_refused_through(&of_kind, &is_deepest);
        let has_classes = super::super::classes::CLASSES.iter().all(|class| {
            (class.needs_selection() && kind == super::HolderKind::Stopped)
                || self
                    .is_refused_through(&of_kind, &|shape| shape == super::ShapeKind::Class(*class))
        });
        let has_stale_graph =
            self.is_refused_through(&of_kind, &|shape| shape == super::ShapeKind::StaleGraph);
        debug_assert!(super::HOLDER_KINDS.contains(&kind));
        debug_assert!(!self.holders.is_empty());
        has_rewind && has_heads && has_depths && has_classes && has_stale_graph
    }

    /// Whether the holder at `position` meets its covered shapes.
    pub(super) fn covers_length(&self, position: usize, holder: &super::Holder) -> bool {
        let at = |at: usize, _: &super::Holder| at == position;
        let is_last = position.saturating_add(1) == self.holders.len();
        let has_refusal = self.is_refused_through(&at, &|_| true)
            && (is_last
                || self.is_refused_through(&at, &|shape| shape == super::ShapeKind::StaleGraph));
        debug_assert!(position < self.holders.len());
        debug_assert!(holder.history.payloads.len() == position.saturating_add(1));
        if holder.base.is_none() {
            return has_refusal
                && self.is_refused_through(&at, &|shape| shape == super::ShapeKind::ForeignRoot);
        }
        let has_shallow = self.is_refused_through(&at, &|shape| {
            matches!(shape, super::ShapeKind::Fork { depth: 0, .. })
        });
        let has_deepest = self.is_refused_through(&at, &is_deepest);
        let has_classes = super::super::classes::CLASSES.iter().all(|class| {
            (class.needs_selection() && !admits_selection(holder))
                || self.is_refused_through(&at, &|shape| shape == super::ShapeKind::Class(*class))
        });
        has_refusal && has_shallow && has_deepest && has_classes
    }

    /// Whether every stopped holder meets a post-stop settlement it loads.
    pub(super) fn advances_every_stop(&self) -> bool {
        let mut stopped = self
            .holders
            .iter()
            .enumerate()
            .filter(|(_, holder)| holder.kind == super::HolderKind::Stopped)
            .peekable();
        let has_stop = stopped.peek().is_some();
        has_stop
            && stopped.all(|(position, _)| {
                self.presented.iter().any(|presentation| {
                    presentation.holder == position
                        && presentation.shape == super::ShapeKind::Advance
                        && !is_refused(presentation)
                })
            })
    }
}

/// Whether a shape is a fork at the deepest depth its base admits for its head kind.
const fn is_deepest(shape: super::ShapeKind) -> bool {
    matches!(
        shape,
        super::ShapeKind::Fork {
            is_deepest: true,
            ..
        }
    )
}

/// Whether the forged-fence class is constructible over a holder: its campaign runs, holds
/// guidance, and has room for one more outstanding selection, so its planner selects.
pub(super) fn admits_selection(holder: &super::Holder) -> bool {
    let state = holder.view.state();
    let is_admitted = admits_head(
        super::super::super::lineages::forks::heads::HeadKind::Selection,
        state,
    );
    state.guidance_record_count > 0 && is_admitted == Some(true)
}

/// Requires the covering rule.
///
/// # Errors
///
/// Returns `Diverged` for a matrix that misses a covered dimension.
// r[impl campaign.conformance.bounds]
pub(super) fn require(
    holders: &[super::Holder],
    presented: &[super::Presented],
) -> crate::verdicts::ConformanceResult<()> {
    let matrix = Matrix { holders, presented };
    let is_every_kind_covered = super::HOLDER_KINDS
        .iter()
        .all(|kind| matrix.covers_kind(*kind));
    let is_every_length_covered = holders
        .iter()
        .enumerate()
        .all(|(position, holder)| matrix.covers_length(position, holder));
    super::super::super::super::common::require(
        is_every_kind_covered && is_every_length_covered && matrix.advances_every_stop(),
        "the held matrix presents every holder kind with a refused rewind, fork of every head \
         kind and depth, and catalog class; every held length with a refusal, its shallowest \
         and deepest forks, and every class its head admits; and every stopped holder with a \
         post-stop settlement it loads",
    )?;
    debug_assert!(presented.iter().any(is_refused));
    Ok(())
}
