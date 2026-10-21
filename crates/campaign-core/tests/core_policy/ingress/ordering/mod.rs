//! Equal candidate batches normalize to one canonical order and identity.

#[test]
// r[verify campaign.candidate_admission.order]
fn equal_batches_in_different_orders_normalize() -> Result<(), String> {
    let (_events, state, existing) = super::entry::fixture()?;
    let left = crate::suite::candidate::record(&state, b"left", crate::suite::constants::COST_ONE);
    let right =
        crate::suite::candidate::record(&state, b"right", crate::suite::constants::COST_ONE);
    let forward =
        crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
            &state,
            vec![left.clone(), right.clone()],
            crate::suite::ingress::support::REJECT,
        ))?;
    let backward =
        crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
            &state,
            vec![right.clone(), left.clone()],
            crate::suite::ingress::support::REJECT,
        ))?;
    assert_eq!(forward, backward);
    let ids: Vec<campaign_core::Id> = crate::suite::ingress::support::batch_of(&forward)?
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect();
    assert!(ids.is_sorted());
    let existing_record = crate::suite::ingress::support::text(state.candidate(existing))?
        .candidate
        .clone();
    let mixed = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![
            right.clone(),
            existing_record.clone(),
            left.clone(),
            right.clone(),
        ],
        crate::suite::ingress::support::COALESCE,
    ))?;
    let shuffled =
        crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
            &state,
            vec![left, right.clone(), right, existing_record],
            crate::suite::ingress::support::COALESCE,
        ))?;
    assert_eq!(mixed, shuffled);
    Ok(())
}
