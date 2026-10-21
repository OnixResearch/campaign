//! Corrupted guidance records: over-dimension, over-magnitude, crossed profile, unknown
//! candidate, stale generation, and non-improving relaxation.

/// Largest generated excess over the rank magnitude bound.
const MAX_EXCESS: i64 = 3;

#[derive(Clone, Debug)]
pub(crate) enum Fault {
    OverDimensionGuidance,
    OverMagnitudeGuidance(i64),
    CrossedGuidanceProfile,
    UnknownGuidanceCandidate,
    StaleGuidanceGeneration(u8),
    NonImprovingGuidance(u8),
}

pub(crate) fn fault() -> impl proptest::strategy::Strategy<Value = Fault> {
    proptest::prop_oneof![
        proptest::strategy::Just(Fault::OverDimensionGuidance),
        proptest::strategy::Strategy::prop_map(1..=MAX_EXCESS, Fault::OverMagnitudeGuidance),
        proptest::strategy::Just(Fault::CrossedGuidanceProfile),
        proptest::strategy::Just(Fault::UnknownGuidanceCandidate),
        proptest::strategy::Strategy::prop_map(
            proptest::num::u8::ANY,
            Fault::StaleGuidanceGeneration
        ),
        proptest::strategy::Strategy::prop_map(proptest::num::u8::ANY, Fault::NonImprovingGuidance),
    ]
}

fn guide(
    state: &campaign_core::FrontierState,
    expected: campaign_core::CampaignError,
    record: campaign_core::GuidanceRecord,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    Ok(Some(crate::suite::rejection::Outcome {
        expected,
        actual: crate::suite::fixture::denied(campaign_core::admit_guidance(state, record))?,
    }))
}

pub(crate) fn unknown_candidate() -> campaign_core::Id {
    crate::suite::fixture::id(campaign_core::IdKind::Candidate, b"unknown-candidate", 0)
}

/// First-generation guidance for an unknown candidate: shape checks precede candidate lookup.
fn unmatched(
    state: &campaign_core::FrontierState,
    ranks: Vec<i64>,
) -> campaign_core::GuidanceRecord {
    crate::suite::fixture::guidance(
        state,
        unknown_candidate(),
        crate::suite::fixture::FIRST_GENERATION,
        ranks,
    )
}

/// Latest guidance of the `pick`-th guided candidate, when one exists.
fn guided(
    state: &campaign_core::FrontierState,
    pick: u8,
) -> Option<&campaign_core::GuidanceRecord> {
    let guided: Vec<&campaign_core::GuidanceRecord> = state
        .candidates
        .iter()
        .filter_map(|candidate| candidate.latest_guidance.as_ref())
        .collect();
    let index = crate::suite::fixture::position(pick, guided.len()).ok()?;
    guided.get(index).copied()
}

/// Re-targets `latest` with a shifted first rank at `generation`; ranks outside the magnitude
/// bound are denied as a shape error before relaxation is judged.
fn relaxed(
    state: &campaign_core::FrontierState,
    latest: &campaign_core::GuidanceRecord,
    shift: i64,
    generation: u64,
    relaxation_error: campaign_core::CampaignError,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let mut ranks = latest.ranks.clone();
    let first = ranks
        .first_mut()
        .ok_or_else(|| String::from("admitted guidance has a first rank"))?;
    *first = first.saturating_add(shift);
    let expected = if first.unsigned_abs() > crate::suite::fixture::RANK_MAGNITUDE {
        campaign_core::CampaignError::InvalidGuidanceShape
    } else {
        relaxation_error
    };
    assert_ne!(ranks, latest.ranks);
    guide(
        state,
        expected,
        crate::suite::fixture::guidance(state, latest.candidate_id, generation, ranks),
    )
}

/// Applies one guidance fault to the history's current state.
pub(crate) fn attempt(
    history: &crate::suite::interpreter::History,
    fault: &Fault,
) -> Result<Option<crate::suite::rejection::Outcome>, String> {
    let state = history.state()?;
    let rank_count = usize::try_from(state.admission.bounds.max_rank_dimensions)
        .map_err(|error| format!("{error}"))?;
    assert!(rank_count > 0);
    assert!(state.candidate(unknown_candidate()).is_err());
    match fault {
        Fault::OverDimensionGuidance => guide(
            state,
            campaign_core::CampaignError::BoundExceeded,
            unmatched(state, vec![0; rank_count.saturating_add(1)]),
        ),
        Fault::OverMagnitudeGuidance(excess) => guide(
            state,
            campaign_core::CampaignError::InvalidGuidanceShape,
            unmatched(
                state,
                vec![crate::suite::fixture::RANK_LIMIT.saturating_add(*excess); rank_count],
            ),
        ),
        Fault::CrossedGuidanceProfile => {
            let mut record = unmatched(state, vec![0; rank_count]);
            record.ordering_profile_id =
                crate::suite::fixture::id(campaign_core::IdKind::GuidanceProfile, b"other", 0);
            guide(state, campaign_core::CampaignError::CrossedIdentity, record)
        }
        Fault::UnknownGuidanceCandidate => guide(
            state,
            campaign_core::CampaignError::MissingCandidate,
            unmatched(state, vec![0; rank_count]),
        ),
        Fault::StaleGuidanceGeneration(pick) => guided(state, *pick).map_or(Ok(None), |latest| {
            relaxed(
                state,
                latest,
                1,
                latest.generation,
                campaign_core::CampaignError::StaleGuidance,
            )
        }),
        Fault::NonImprovingGuidance(pick) => guided(state, *pick).map_or(Ok(None), |latest| {
            relaxed(
                state,
                latest,
                -1,
                latest.generation.saturating_add(1),
                campaign_core::CampaignError::NonImprovingRelaxation,
            )
        }),
    }
}
