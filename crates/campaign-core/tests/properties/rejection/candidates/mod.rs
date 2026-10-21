//! Corrupted candidate batches: over-bound, empty, unknown parent, crossed campaign or adapter.

/// Largest generated excess over the batch bound.
const MAX_EXCESS: u64 = 3;
/// First label of corrupted records; disjoint from scripted labels.
const LABEL_BASE: u64 = 1 << 32;

#[derive(Clone, Debug)]
pub(crate) enum Fault {
    OverBoundBatch(u64),
    EmptyBatch,
    UnknownParent,
    CrossedCandidateCampaign,
    CrossedCandidateAdapter,
}

pub(crate) fn fault() -> impl proptest::strategy::Strategy<Value = Fault> {
    proptest::prop_oneof![
        proptest::strategy::Strategy::prop_map(1..=MAX_EXCESS, Fault::OverBoundBatch),
        proptest::strategy::Just(Fault::EmptyBatch),
        proptest::strategy::Just(Fault::UnknownParent),
        proptest::strategy::Just(Fault::CrossedCandidateCampaign),
        proptest::strategy::Just(Fault::CrossedCandidateAdapter),
    ]
}

fn fresh(
    state: &campaign_core::FrontierState,
    label_index: u64,
) -> Result<campaign_core::CandidateRecord, String> {
    let root = state
        .admission
        .roots
        .first()
        .ok_or_else(|| String::from("a campaign has a root"))?;
    Ok(crate::suite::fixture::record(
        state,
        LABEL_BASE.saturating_add(label_index),
        campaign_core::KnownMoment {
            moment_id: root.moment_id,
            depth: campaign_core::frontier::ROOT_DEPTH,
        },
        vec![campaign_core::ResourceAmount {
            dimension_id: crate::suite::fixture::dimension(0),
            amount: campaign_core::UNIT_COST,
        }],
    ))
}

fn batch(
    state: &campaign_core::FrontierState,
    expected: campaign_core::CampaignError,
    records: Vec<campaign_core::CandidateRecord>,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let outcome = campaign_core::admit_candidates(
        state,
        records,
        crate::suite::fixture::rule(false),
        crate::suite::fixture::coalescing_policy(),
    );
    Ok(Some(crate::suite::rejection::Outcome {
        expected,
        actual: crate::suite::fixture::denied(outcome)?,
    }))
}

/// Applies one candidate-batch fault to the history's current state.
pub(crate) fn attempt(
    history: &crate::suite::interpreter::History,
    fault: &Fault,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let state = history.state()?;
    let crossed = campaign_core::CampaignError::CrossedIdentity;
    match fault {
        Fault::OverBoundBatch(excess) => {
            let count = state
                .admission
                .bounds
                .max_candidate_batch
                .saturating_add(*excess);
            let records: Vec<campaign_core::CandidateRecord> = (0..count)
                .map(|label_index| fresh(state, label_index))
                .collect::<Result<_, _>>()?;
            assert!(
                u64::try_from(records.len())
                    .is_ok_and(|len| len > state.admission.bounds.max_candidate_batch)
            );
            batch(state, campaign_core::CampaignError::BoundExceeded, records)
        }
        Fault::EmptyBatch => batch(
            state,
            campaign_core::CampaignError::MissingCandidate,
            Vec::new(),
        ),
        Fault::UnknownParent => {
            let mut record = fresh(state, 0)?;
            record.parent_moment_id =
                crate::suite::fixture::id(campaign_core::IdKind::ChildMoment, b"unknown-parent", 0);
            assert!(!state.has_known_moment(record.parent_moment_id));
            batch(
                state,
                campaign_core::CampaignError::MissingParentMoment,
                vec![record],
            )
        }
        Fault::CrossedCandidateCampaign => {
            let mut record = fresh(state, 0)?;
            record.campaign_id =
                crate::suite::fixture::id(campaign_core::IdKind::Campaign, b"other", 0);
            batch(state, crossed, vec![record])
        }
        Fault::CrossedCandidateAdapter => {
            let mut record = fresh(state, 0)?;
            record.source_adapter_id =
                crate::suite::fixture::id(campaign_core::IdKind::Adapter, b"other", 0);
            batch(state, crossed, vec![record])
        }
    }
}
