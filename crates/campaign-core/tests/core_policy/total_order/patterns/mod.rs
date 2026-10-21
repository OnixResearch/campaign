//! Over 21 candidates, where a non-total comparator made `slice::sort_by` panic, no mixed-length
//! guidance history can be admitted or replayed, and the one-shape frontier plans the reference
//! choice.

const CANDIDATES: u64 = 21;
const PATTERNS: u64 = 64;
const RANK_VALUES: u64 = 4;

/// Deterministic xorshift for rank patterns.
const fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// Mixed-length guidance for `ids` from one xorshift pattern: each candidate gets `[5]` or
/// `[5, v]` with `v` in `0..RANK_VALUES`.
fn pattern(
    ids: &[campaign_core::Id],
    index: u64,
) -> Result<Vec<(campaign_core::Id, Vec<i64>)>, String> {
    let mut seed = index;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let value =
            i64::try_from(next(&mut seed) % RANK_VALUES).map_err(crate::suite::s3_fixture::err)?;
        let ranks = if next(&mut seed).is_multiple_of(2) {
            vec![5]
        } else {
            vec![5, value]
        };
        out.push((*id, ranks));
    }
    assert_eq!(out.len(), ids.len());
    Ok(out)
}

/// Admits the pattern up to its first record of another length and requires that record to be
/// refused at admission and replay with `InvalidGuidanceShape`.
fn refused_at_first_mismatch(
    events: &super::History,
    state: &campaign_core::FrontierState,
    guidance: &[(campaign_core::Id, Vec<i64>)],
    position: usize,
) -> Result<(), String> {
    let (prefix_events, prefix) =
        super::guided(events.clone(), state.clone(), &guidance[..position])?;
    let (candidate_id, ranks) = guidance[position].clone();
    let generation = crate::suite::constants::GENERATION_NEXT;
    let update = super::record(&prefix, candidate_id, generation, ranks.clone());
    let shape = campaign_core::CampaignError::InvalidGuidanceShape;
    let admitted = campaign_core::admit_guidance(&prefix, update);
    assert_eq!(crate::suite::s3_fixture::denial(admitted)?, shape);
    let forged = super::sealed_discovery(&prefix, candidate_id, ranks)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(&prefix_events, forged)?,
        shape
    );
    Ok(())
}

#[test]
// r[verify campaign.guidance.shape]
// r[verify campaign.guidance.valid]
fn twenty_one_candidates_never_mix_lengths_and_plan_the_reference_choice() -> Result<(), String> {
    let (events, state, ids) = super::admitted_frontier(CANDIDATES)?;
    let mut mixed_patterns = 0_u64;
    for index in 1..=PATTERNS {
        let guidance = pattern(&ids, index)?;
        let fixed = guidance[0].1.len();
        if let Some(position) = guidance.iter().position(|(_, ranks)| ranks.len() != fixed) {
            mixed_patterns = mixed_patterns.saturating_add(1);
            refused_at_first_mismatch(&events, &state, &guidance, position)?;
        }
        let completed: Vec<(campaign_core::Id, Vec<i64>)> = guidance
            .into_iter()
            .map(|(candidate_id, mut ranks)| {
                ranks.resize(fixed, 0);
                (candidate_id, ranks)
            })
            .collect();
        let mut reference = completed.clone();
        reference.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        let (guided_events, guided_state) =
            super::guided(events.clone(), state.clone(), &completed)?;
        let selection = super::selected(&guided_state)?;
        assert_eq!(selection.candidate_id, reference[0].0);
        let profile = crate::suite::selection::ranked_profile()?;
        let event = crate::suite::selection::event_with(selection, profile, None)?;
        let (_history, published) = crate::suite::s3_fixture::append(guided_events, event)?;
        assert_eq!(published.selection_count, 1);
    }
    assert!(mixed_patterns > 0);
    Ok(())
}
