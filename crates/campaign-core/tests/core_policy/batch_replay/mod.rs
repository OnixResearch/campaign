//! D71: replay compares a candidate batch's record list with its bounded outcome list before
//! validating any record, so a forged list longer or shorter than the admitted outcomes is
//! refused as `MalformedIdentity` without per-record work.

/// The admitted campaign history, its state, and the next genuine one-candidate batch.
fn genuine_admission() -> Result<
    (
        crate::suite::s3_fixture::History,
        campaign_core::FrontierState,
        campaign_core::CandidateBatchEvent,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|_| {})?;
    let event = campaign_core::admit_candidates(
        &state,
        vec![crate::suite::candidate::record(
            &state,
            b"replay-admitted",
            crate::suite::constants::COST_ONE,
        )],
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) = event else {
        return Err(format!("expected a candidate batch, got {event:?}"));
    };
    assert_eq!(batch.candidates.len(), 1);
    assert_eq!(batch.outcomes.len(), 1);
    Ok((events, state, batch))
}

/// The genuine batch replays; a record list with one extra record that would itself fail
/// validation (over the depth bound) is refused for its length, not for the record; and an empty
/// record list under an `Admitted` outcome is refused the same way.
#[test]
fn a_record_list_unlike_the_outcomes_is_refused_before_record_validation() -> Result<(), String> {
    let (events, before, genuine) = genuine_admission()?;
    let (_events, after) = crate::suite::s3_fixture::append(
        events.clone(),
        campaign_core::CampaignControlEvent::CandidatesAdmitted(genuine.clone()),
    )?;
    assert_eq!(after.candidates.len(), 1);
    let mut over_depth = crate::suite::candidate::record(
        &before,
        b"replay-forged",
        crate::suite::constants::COST_ONE,
    );
    over_depth.depth = before.admission.bounds.max_depth.saturating_add(1);
    let over_depth = over_depth.derive().map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_candidates(
            &before,
            vec![over_depth.clone()],
            genuine.admission_rule,
            genuine.coalescing_policy_id,
        ))?,
        campaign_core::CampaignError::CandidateOverDepth
    );
    let mut longer = genuine.clone();
    longer.candidates.insert(0, over_depth);
    let mut shorter = genuine;
    shorter.candidates.clear();
    for forged in [longer, shorter] {
        assert_eq!(
            crate::suite::s3_fixture::replay_denial(
                &events,
                campaign_core::CampaignControlEvent::CandidatesAdmitted(forged),
            )?,
            campaign_core::CampaignError::MalformedIdentity
        );
    }
    Ok(())
}
