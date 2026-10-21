//! Observations for another selection, parent, operation, campaign, adapter, or event fail.

fn edited(
    base: &campaign_core::ResultObservation,
    edit: impl FnOnce(&mut campaign_core::ResultObservation),
) -> campaign_core::ResultObservation {
    let mut observation = base.clone();
    edit(&mut observation);
    observation
}

fn other(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(kind, label)
}

/// One crossed variant of an exact observation per bound identity.
fn crossed_cases(
    exact: &campaign_core::ResultObservation,
    second: &campaign_core::SelectionDecision,
) -> Vec<(
    campaign_core::ResultObservation,
    campaign_core::CampaignError,
)> {
    assert_ne!(exact.selection_id, second.selection_id);
    assert_ne!(exact.operation_id, second.operation_id);
    let crossed = campaign_core::CampaignError::CrossedIdentity;
    vec![
        (
            edited(exact, |observation| {
                observation.campaign_id = other(campaign_core::IdKind::Campaign, b"other-campaign");
            }),
            crossed.clone(),
        ),
        (
            edited(exact, |observation| {
                observation.adapter_id = other(campaign_core::IdKind::Adapter, b"other-adapter");
            }),
            crossed.clone(),
        ),
        (
            edited(exact, |observation| {
                observation.operation_id = second.operation_id;
            }),
            crossed.clone(),
        ),
        (
            edited(exact, |observation| {
                observation.parent_moment_id =
                    other(campaign_core::IdKind::ChildMoment, b"other-parent");
            }),
            crossed.clone(),
        ),
        (
            edited(exact, |observation| {
                observation.selection_id = second.selection_id;
            }),
            crossed.clone(),
        ),
        (
            edited(exact, |observation| {
                observation.payload_id = other(campaign_core::IdKind::Finding, b"not-an-event");
            }),
            crossed,
        ),
        (
            edited(exact, |observation| {
                observation.selection_id = other(campaign_core::IdKind::Selection, b"unknown");
            }),
            campaign_core::CampaignError::MissingSelection,
        ),
    ]
}

#[test]
// r[verify campaign.observations.crossed]
fn crossed_observations_are_denied_at_entry_and_replay_without_mutation() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"crossed")?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let child =
        crate::suite::s3_fixture::child(&state, &first, b"crossed-child", b"crossed-event")?;
    let exact = crate::suite::s3_fixture::observation(&state, &first, vec![child]);
    let foreign =
        crate::suite::s3_fixture::child(&state, &second, b"crossed-child", b"crossed-event")?;
    let mut cases = crossed_cases(&exact, &second);
    cases.push((
        crate::suite::s3_fixture::observation(&state, &first, vec![foreign]),
        campaign_core::CampaignError::InvalidChildReservation,
    ));
    for (observation, expected) in cases {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::admit_result(
                &state,
                observation.clone()
            ))?,
            expected
        );
        if observation.campaign_id == state.admission.campaign_id {
            let forged = crate::suite::s3_fixture::forged_result(&state, observation)?;
            assert_eq!(
                crate::suite::s3_fixture::replay_denial(&events, forged)?,
                expected
            );
        }
        crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    }
    campaign_core::admit_result(&state, exact).map_err(crate::suite::s3_fixture::err)?;
    Ok(())
}
