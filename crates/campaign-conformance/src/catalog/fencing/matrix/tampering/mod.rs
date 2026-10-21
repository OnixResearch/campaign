//! Single-field tampering of a published decision under its own selection identity.
//!
//! Every field of `SelectionDecision` except `selection_id` is replaced, one at a time, by a
//! value the publication never carried: each identity by a foreign identity of the same kind,
//! each counter by its successor, the optional ticket identity by every other presence, and the
//! reserved costs by a changed amount, a changed dimension, an extra dimension, and no cost.

/// Frame name of every foreign identity a tampered field takes.
const FOREIGN: &str = "tampered-decision-field";

/// One tampered decision and the forbidden effect its execution names.
pub(super) type Tampered = (&'static str, campaign_core::SelectionDecision);

/// A foreign identity of `id`'s kind.
fn foreign(id: campaign_core::Id) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
    crate::harness::suite_id(id.kind, FOREIGN)
}

/// `selection` with one edit applied.
fn edited(
    selection: &campaign_core::SelectionDecision,
    forbidden: &'static str,
    edit: impl FnOnce(&mut campaign_core::SelectionDecision),
) -> Tampered {
    let mut tampered = selection.clone();
    edit(&mut tampered);
    (forbidden, tampered)
}

/// One identity field of a decision and the forbidden effect of executing it tampered.
type IdField = (
    &'static str,
    fn(&mut campaign_core::SelectionDecision) -> &mut campaign_core::Id,
);

/// Every identity field of a decision except its selection identity, fence fields last.
const ID_FIELDS: [IdField; 13] = [
    ("a decision tampered in its campaign executed", |d| {
        &mut d.campaign_id
    }),
    ("a decision tampered in its state executed", |d| {
        &mut d.state_before_id
    }),
    ("a decision tampered in its candidate executed", |d| {
        &mut d.candidate_id
    }),
    ("a decision tampered in its parent moment executed", |d| {
        &mut d.parent_moment_id
    }),
    ("a decision tampered in its operation executed", |d| {
        &mut d.operation_id
    }),
    ("a decision tampered in its guidance executed", |d| {
        &mut d.guidance_id
    }),
    ("a decision tampered in its profile executed", |d| {
        &mut d.profile_id
    }),
    ("a decision tampered in its fence event executed", |d| {
        &mut d.publication_plan.event_id
    }),
    ("a decision tampered in its fence campaign executed", |d| {
        &mut d.publication_plan.campaign_id
    }),
    ("a decision tampered in its fence branch executed", |d| {
        &mut d.publication_plan.control_branch_id
    }),
    ("a decision tampered in its fence head executed", |d| {
        &mut d.publication_plan.expected_head_id
    }),
    ("a decision tampered in its fence state executed", |d| {
        &mut d.publication_plan.state_before_id
    }),
    ("a decision tampered in its fence selection executed", |d| {
        &mut d.publication_plan.selection_id
    }),
];

/// One counter field of a decision and the forbidden effect of executing it tampered.
type CounterField = (
    &'static str,
    fn(&mut campaign_core::SelectionDecision) -> &mut u64,
);

/// Every counter field of a decision.
const COUNTER_FIELDS: [CounterField; 2] = [
    ("a decision tampered in its ordinal executed", |d| {
        &mut d.ordinal
    }),
    (
        "a decision tampered in its fence generation executed",
        |d| &mut d.publication_plan.expected_generation,
    ),
];

/// Every single-field tampering of `selection` that keeps its selection identity: identities,
/// counters, the ticket identity, and the reserved costs.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` at the last counter value, a framing error, and `Diverged` when a
/// tampered decision loses the selection identity or equals the published one.
pub(super) fn tampered_decisions(
    selection: &campaign_core::SelectionDecision,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Tampered>> {
    let mut decisions = alloc::vec::Vec::with_capacity(ID_FIELDS.len().saturating_add(8));
    for (forbidden, field) in ID_FIELDS {
        let mut tampered = selection.clone();
        let id = field(&mut tampered);
        *id = foreign(*id)?;
        decisions.push((forbidden, tampered));
    }
    for (forbidden, field) in COUNTER_FIELDS {
        let mut tampered = selection.clone();
        let counter = field(&mut tampered);
        *counter = successor(*counter)?;
        decisions.push((forbidden, tampered));
    }
    decisions.extend(ticket_edits(selection)?);
    decisions.extend(cost_edits(selection)?);
    let is_identity_kept = decisions.iter().all(|(_, tampered)| {
        tampered.selection_id == selection.selection_id && tampered != selection
    });
    super::super::super::common::require(
        is_identity_kept,
        "every tampered decision keeps the published selection identity and differs from it",
    )?;
    debug_assert!(decisions.len() > ID_FIELDS.len().saturating_add(COUNTER_FIELDS.len()));
    Ok(decisions)
}

/// The ticket identity replaced by every other presence: absent, or a foreign ticket.
fn ticket_edits(
    selection: &campaign_core::SelectionDecision,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Tampered>> {
    let mut decisions = alloc::vec::Vec::with_capacity(2);
    let foreign_ticket = crate::harness::suite_id(campaign_core::IdKind::EntropyTicket, FOREIGN)?;
    if selection.entropy_id.is_some() {
        decisions.push(edited(
            selection,
            "a decision stripped of its ticket executed",
            |d| d.entropy_id = None,
        ));
    }
    decisions.push(edited(
        selection,
        "a decision tampered in its ticket executed",
        |d| d.entropy_id = Some(foreign_ticket),
    ));
    debug_assert!(!decisions.is_empty());
    Ok(decisions)
}

/// The reserved costs with a changed amount, a changed dimension, an extra dimension, and none.
fn cost_edits(
    selection: &campaign_core::SelectionDecision,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Tampered>> {
    let dimension = crate::harness::suite_id(campaign_core::IdKind::ResourceDimension, FOREIGN)?;
    let first = selection.reserved_costs.first().cloned().ok_or(
        crate::verdicts::ConformanceError::Diverged {
            expected: "the published selection reserves a cost",
        },
    )?;
    let amount = successor(first.amount)?;
    let extra = campaign_core::ResourceAmount {
        dimension_id: dimension,
        amount: first.amount,
    };
    let decisions = alloc::vec![
        edited(
            selection,
            "a decision tampered in its reserved amount executed",
            |d| {
                if let Some(cost) = d.reserved_costs.first_mut() {
                    cost.amount = amount;
                }
            }
        ),
        edited(
            selection,
            "a decision tampered in its reserved dimension executed",
            |d| {
                if let Some(cost) = d.reserved_costs.first_mut() {
                    cost.dimension_id = dimension;
                }
            }
        ),
        edited(
            selection,
            "a decision reserving an extra dimension executed",
            |d| {
                d.reserved_costs.push(extra);
            }
        ),
        edited(selection, "a decision reserving nothing executed", |d| d
            .reserved_costs
            .clear()),
    ];
    debug_assert!(first.amount > 0);
    Ok(decisions)
}

/// The next counter value.
fn successor(value: u64) -> crate::verdicts::ConformanceResult<u64> {
    Ok(value
        .checked_add(1)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?)
}
