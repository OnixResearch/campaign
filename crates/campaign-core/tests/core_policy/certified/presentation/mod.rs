//! Event receipts certify only the canonical presentation replay accepts: every carried derived
//! identity re-derived, canonical lists in canonical order.

/// Reordered prune targets and forged prune plan or invalidation identities keep the event
/// identity, yet replay and the event receipt both refuse them with `MalformedIdentity`.
#[test]
fn a_forged_prune_is_malformed() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let targets = vec![frontier.middle, frontier.lowest];
    let request = crate::suite::s3_pruning::explicit(&frontier.state, targets);
    let prune = campaign_core::prune_frontier(&frontier.state, request)
        .map_err(crate::suite::s3_fixture::err)?;
    let (_events, pruned) =
        crate::suite::s3_fixture::append(frontier.events.clone(), prune.clone())?;
    crate::suite::world::observed(crate::suite::certified::ranked_event_receipt(
        &pruned, &prune,
    )?)?;
    assert_eq!(
        prune.canonical().map_err(crate::suite::s3_fixture::err)?,
        prune
    );
    let campaign_core::CampaignControlEvent::CandidatesPruned(genuine) = prune else {
        return Err(String::from("expected a prune event"));
    };
    let forgeries: [fn(&mut campaign_core::PruneEvent); 3] = [
        |event| event.pruned_candidates.reverse(),
        |event| {
            event.invalidation_id =
                crate::suite::admission::xid(campaign_core::IdKind::Invalidation, b"forged");
        },
        |event| {
            event.prune_plan_id =
                crate::suite::admission::xid(campaign_core::IdKind::PrunePlan, b"forged");
        },
    ];
    let malformed = campaign_core::CampaignError::MalformedIdentity;
    for forge in forgeries {
        let mut forged = genuine.clone();
        forge(&mut forged);
        let forged = campaign_core::CampaignControlEvent::CandidatesPruned(forged);
        let expected = forged
            .expected_event_id()
            .map_err(crate::suite::s3_fixture::err)?;
        assert_eq!(expected, forged.event_id());
        let replayed = campaign_core::apply_control_event(frontier.state.clone(), &forged);
        assert_eq!(crate::suite::s3_fixture::denial(replayed)?, malformed);
        let receipt = crate::suite::certified::ranked_event_receipt(&pruned, &forged)?;
        assert_eq!(crate::suite::s3_fixture::denial(receipt)?, malformed);
    }
    Ok(())
}

/// A retirement with a forged invalidation identity and a result with reordered children keep
/// the event identity but are refused with `MalformedIdentity`; the genuine events project.
#[test]
fn a_forged_retirement_or_result_is_malformed() -> Result<(), String> {
    let malformed = campaign_core::CampaignError::MalformedIdentity;
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let origin = campaign_core::RetirementOrigin::Consumer;
    let retirement = crate::suite::s3_fixture::retirement(&frontier.state, frontier.kept, origin);
    let retire = campaign_core::retire_candidate(&frontier.state, retirement)
        .map_err(crate::suite::s3_fixture::err)?;
    let (_events, retired) = crate::suite::s3_fixture::append(frontier.events, retire.clone())?;
    crate::suite::world::observed(crate::suite::certified::ranked_event_receipt(
        &retired, &retire,
    )?)?;
    let mut forged_retire = retire;
    if let campaign_core::CampaignControlEvent::CandidateRetired(event) = &mut forged_retire {
        event.invalidation_id =
            crate::suite::admission::xid(campaign_core::IdKind::Invalidation, b"forged-retire");
    }
    let receipt = crate::suite::certified::ranked_event_receipt(&retired, &forged_retire)?;
    assert_eq!(crate::suite::s3_fixture::denial(receipt)?, malformed);
    let world = crate::suite::world::populated()?;
    let result = world
        .events
        .iter()
        .find(|event| {
            matches!(
                event,
                campaign_core::CampaignControlEvent::ResultObserved(_)
            )
        })
        .cloned()
        .ok_or("the populated world observes a result")?;
    crate::suite::world::observed(crate::suite::certified::ranked_event_receipt(
        &world.state,
        &result,
    )?)?;
    let mut reordered = result;
    if let campaign_core::CampaignControlEvent::ResultObserved(event) = &mut reordered {
        assert_eq!(event.observation.children.len(), 2);
        event.observation.children.reverse();
    }
    assert_eq!(
        reordered
            .expected_event_id()
            .map_err(crate::suite::s3_fixture::err)?,
        reordered.event_id()
    );
    let receipt = crate::suite::certified::ranked_event_receipt(&world.state, &reordered)?;
    assert_eq!(crate::suite::s3_fixture::denial(receipt)?, malformed);
    Ok(())
}
