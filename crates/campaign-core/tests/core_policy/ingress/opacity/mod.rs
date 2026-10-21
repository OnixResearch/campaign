//! Guidance ranks stay opaque bounded integers.

const FIRST_GENERATION: u64 = 1;
const SECOND_GENERATION: u64 = 2;
const THIRD_GENERATION: u64 = 3;

fn xid(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(kind, label)
}

fn magnitude() -> Result<i64, String> {
    i64::try_from(crate::suite::constants::RANK_MAGNITUDE).map_err(|error| format!("{error:?}"))
}

fn opaque_selection(
    labels: [&'static [u8]; 2],
) -> Result<(campaign_core::Id, campaign_core::Id), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, low) =
        crate::suite::candidate::admit(events, &state, b"low", crate::suite::constants::COST_ONE)?;
    let (events, state, high) =
        crate::suite::candidate::admit(events, &state, b"high", crate::suite::constants::COST_ONE)?;
    let mut events = events;
    let mut state = state;
    for (candidate_id, rank, label) in [
        (low, crate::suite::constants::RANK_LOW, labels[0]),
        (high, crate::suite::constants::RANK_HIGH, labels[1]),
    ] {
        let record = campaign_core::GuidanceRecord {
            source_facts_id: xid(campaign_core::IdKind::SourceFacts, label),
            ..crate::suite::ingress::support::guidance(
                &state,
                candidate_id,
                FIRST_GENERATION,
                vec![rank],
            )
        };
        let event =
            crate::suite::ingress::support::text(campaign_core::admit_guidance(&state, record))?;
        (events, state) = crate::suite::ingress::support::accept(events, event)?;
    }
    let selection = crate::suite::selection::plan(&state)?;
    assert_eq!(
        selection.guidance_id,
        crate::suite::ingress::support::latest(&state, selection.candidate_id)?.guidance_id
    );
    assert_ne!(low, high);
    Ok((selection.candidate_id, high))
}

#[test]
// r[verify campaign.guidance.opaque]
fn campaign_compares_only_the_supplied_bounded_integers() -> Result<(), String> {
    let (chosen, high) = opaque_selection([b"coverage", b"severity"])?;
    assert_eq!(chosen, high);
    let (swapped, high) = opaque_selection([b"severity", b"coverage"])?;
    assert_eq!(swapped, high);
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_LOW)?;
    let top = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        SECOND_GENERATION,
        vec![magnitude()?],
    );
    let event = crate::suite::ingress::support::text(campaign_core::admit_guidance(&state, top))?;
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(
        crate::suite::ingress::support::latest(&next, candidate_id)?.ranks,
        vec![magnitude()?]
    );
    let over = crate::suite::ingress::support::guidance(
        &next,
        candidate_id,
        THIRD_GENERATION,
        vec![magnitude()?.saturating_add(1)],
    );
    assert_eq!(
        campaign_core::admit_guidance(&next, over),
        Err(campaign_core::CampaignError::InvalidGuidanceShape)
    );
    let floor = crate::suite::ingress::support::guidance(
        &next,
        candidate_id,
        THIRD_GENERATION,
        vec![-magnitude()?],
    );
    assert_eq!(
        campaign_core::admit_guidance(&next, floor),
        Err(campaign_core::CampaignError::NonImprovingRelaxation)
    );
    Ok(())
}
