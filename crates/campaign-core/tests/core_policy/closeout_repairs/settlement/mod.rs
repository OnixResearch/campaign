//! Exploration payload uniqueness and consistent finding re-reports.

/// An exploration payload accepted with a rediscovery of child C stays bound to C: a later result
/// binding it to child D, or reusing an accepted result payload, is a conflict.
#[test]
// r[verify campaign.fork.duplicate]
fn an_accepted_payload_is_bound_to_one_child_or_result() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, _candidate) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"audit-fork",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, first) = crate::suite::s3_fixture::published(events, &state)?;
    let first_child =
        crate::suite::s3_fixture::child(&state, &first, b"audit-child-c", b"audit-payload-1")?;
    let first_result = crate::suite::s3_fixture::observation(&state, &first, vec![first_child]);
    let event =
        campaign_core::admit_result(&state, first_result).map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let rediscovered =
        crate::suite::s3_fixture::child(&state, &second, b"audit-child-c", b"audit-payload-2")?;
    let second_result = crate::suite::s3_fixture::observation(&state, &second, vec![rediscovered]);
    let event = campaign_core::admit_result(&state, second_result)
        .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(state.discoveries.len(), 1);
    let payload =
        crate::suite::admission::xid(campaign_core::IdKind::EventPayload, b"audit-payload-2");
    let child_c =
        crate::suite::admission::xid(campaign_core::IdKind::ChildMoment, b"audit-child-c");
    assert_eq!(state.payload_owner(payload), Some(child_c));

    let (events, state, third) = crate::suite::s3_fixture::published(events, &state)?;
    let other_child =
        crate::suite::s3_fixture::child(&state, &third, b"audit-child-d", b"audit-payload-2")?;
    let third_result = crate::suite::s3_fixture::observation(&state, &third, vec![other_child]);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(
            &state,
            third_result.clone()
        ))?,
        campaign_core::CampaignError::ConflictingResult
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, third_result)?,
        )?,
        campaign_core::CampaignError::ConflictingResult
    );
    let mut reused = crate::suite::s3_fixture::observation(&state, &third, Vec::new());
    reused.payload_id =
        crate::suite::s3_fixture::observation(&state, &first, Vec::new()).payload_id;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, reused))?,
        campaign_core::CampaignError::ConflictingResult
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    Ok(())
}

/// A finding re-reported with other source facts conflicts with the accepted finding.
#[test]
// r[verify campaign.findings]
fn a_rereported_finding_must_repeat_its_source_facts() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"audit-find")?;
    let mut observation = crate::suite::s3_fixture::observation(&state, &first, Vec::new());
    observation.findings = vec![crate::suite::s3_fixture::finding(b"audit-finding")];
    let event =
        campaign_core::admit_result(&state, observation).map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let mut repeated = crate::suite::s3_fixture::observation(&state, &second, Vec::new());
    repeated.findings = vec![crate::suite::s3_fixture::finding(b"audit-finding")];
    let event =
        campaign_core::admit_result(&state, repeated).map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(state.findings.len(), 1);
    let (events, state, third) = crate::suite::s3_fixture::published(events, &state)?;
    let mut conflicting = crate::suite::s3_fixture::observation(&state, &third, Vec::new());
    conflicting.findings = vec![campaign_core::FindingRef {
        finding_id: crate::suite::admission::xid(campaign_core::IdKind::Finding, b"audit-finding"),
        priority_facts_id: crate::suite::admission::xid(
            campaign_core::IdKind::SourceFacts,
            b"audit-other-facts",
        ),
    }];
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, conflicting))?,
        campaign_core::CampaignError::ConflictingResult
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    Ok(())
}
