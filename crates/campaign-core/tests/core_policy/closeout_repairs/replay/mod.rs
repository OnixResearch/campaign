//! Replay re-runs the declared policy and pins canonical event content: off-policy choices,
//! unbound policy inputs, unpinned publication plans, non-canonical prunes, and guidance for
//! non-retained candidates never replay.

/// A selection of the lower-ranked candidate, re-derived so only the policy check can deny it.
fn off_policy(
    state: &campaign_core::FrontierState,
    planned: &campaign_core::SelectionDecision,
    lower: campaign_core::Id,
) -> Result<campaign_core::SelectionDecision, String> {
    let lower_state = state
        .candidate(lower)
        .map_err(crate::suite::s3_fixture::err)?
        .clone();
    let mut forged = planned.clone();
    forged.candidate_id = lower;
    forged.parent_moment_id = lower_state.candidate.parent_moment_id;
    forged.guidance_id = lower_state
        .latest_guidance
        .as_ref()
        .ok_or_else(|| String::from("guided"))?
        .guidance_id;
    forged.reserved_costs = lower_state.candidate.declared_costs;
    forged.derive().map_err(crate::suite::s3_fixture::err)
}

/// Replay re-runs the declared policy on the pre-state: only the policy's own choice replays, the
/// event's profile and ticket must be the ones its decision names, and a ticket or a reference
/// implementation is refused.
#[test]
fn a_published_selection_must_be_the_declared_policy_choice_with_its_profile_and_entropy()
-> Result<(), String> {
    let (events, state, best, lower) = super::guided_pair()?;
    let planned = crate::suite::selection::plan(&state)?;
    assert_eq!(planned.candidate_id, best);
    let (_honest_events, honest) =
        crate::suite::selection::publish(events.clone(), planned.clone())?;
    assert_eq!(
        honest
            .candidate(best)
            .map_err(crate::suite::s3_fixture::err)?
            .selection_count,
        1
    );

    let forged = off_policy(&state, &planned, lower)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(&events, crate::suite::selection::event(forged)?)?,
        campaign_core::CampaignError::CrossedIdentity
    );

    let mut unbound = planned.clone();
    unbound.profile_id =
        crate::suite::admission::xid(campaign_core::IdKind::Graph, b"audit-not-a-profile");
    unbound.entropy_id = Some(crate::suite::admission::xid(
        campaign_core::IdKind::Finding,
        b"audit-not-a-ticket",
    ));
    let unbound = unbound.derive().map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::SelectionEvent::new(
            unbound,
            crate::suite::selection::ranked_profile()?,
            None,
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );

    let ranked = crate::suite::selection::ranked_profile()?;
    let ticket = crate::suite::schedules::ticket(&state, &ranked, 0)?;
    let mut ticketed = planned.clone();
    ticketed.entropy_id = Some(ticket.entropy_id);
    let ticketed = ticketed.derive().map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::selection::event_with(ticketed, ranked, Some(ticket))?,
        )?,
        campaign_core::CampaignError::InvalidEntropy
    );

    let reference = crate::suite::schedules::variant(|profile| {
        profile.implementation = campaign_core::PolicyImplementation::ReferenceEvidence {
            reference_id: crate::suite::admission::xid(
                campaign_core::IdKind::Policy,
                b"audit-reference",
            ),
        };
    })?;
    let referenced = super::selection_of(&state, best, &reference)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::selection::event_with(referenced, reference, None)?,
        )?,
        campaign_core::CampaignError::ReferencePolicySelected
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    Ok(())
}

/// `SelectionDecision::derive` pins `PublicationPlan.campaign_id` and `state_before_id` to the
/// decision, so replay rejects any other value.
#[test]
fn selection_derive_pins_the_publication_plan_campaign_and_state() -> Result<(), String> {
    let (events, state, _best, _lower) = super::guided_pair()?;
    let planned = crate::suite::selection::plan(&state)?;
    let mut altered = planned.clone();
    altered.publication_plan.campaign_id =
        crate::suite::admission::xid(campaign_core::IdKind::Campaign, b"audit-other-campaign");
    altered.publication_plan.state_before_id =
        crate::suite::admission::xid(campaign_core::IdKind::State, b"audit-other-state");
    let rederived = altered
        .clone()
        .derive()
        .map_err(crate::suite::s3_fixture::err)?;
    assert_ne!(rederived, altered);
    assert_eq!(rederived, planned);
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(&events, crate::suite::selection::event(altered)?)?,
        campaign_core::CampaignError::MalformedIdentity
    );
    Ok(())
}

/// Refuses improved guidance, a forged guidance event, and a revision for `candidate_id`.
fn assert_guidance_refused(
    events: &crate::suite::s3_fixture::History,
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    refusal: &campaign_core::CampaignError,
) -> Result<(), String> {
    let latest = state
        .candidate(candidate_id)
        .map_err(crate::suite::s3_fixture::err)?
        .latest_guidance
        .clone()
        .ok_or_else(|| String::from("guided"))?;
    let improved = crate::suite::ingress::support::guidance(
        state,
        candidate_id,
        latest.generation.saturating_add(1),
        vec![latest.ranks[0].saturating_add(1)],
    );
    assert_eq!(
        &crate::suite::s3_fixture::denial(campaign_core::admit_guidance(state, improved.clone()))?,
        refusal
    );
    let derived = improved.derive().map_err(crate::suite::s3_fixture::err)?;
    let dynamics = campaign_core::GuidanceDynamics::StrictImprovement {
        previous_guidance_id: latest.guidance_id,
    };
    assert_eq!(
        &crate::suite::s3_fixture::replay_denial(
            events,
            crate::suite::ingress::support::forged_guidance(state, derived.clone(), dynamics)?,
        )?,
        refusal
    );
    let revision = campaign_core::GuidanceRevisionRequest {
        candidate_id,
        successor: derived,
        invalidated_guidance_id: latest.guidance_id,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"audit-revise"),
    };
    assert_eq!(
        &crate::suite::s3_fixture::denial(campaign_core::revise_guidance(state, revision))?,
        refusal
    );
    Ok(())
}

/// Guidance and revisions name only retained candidates: a retired or pruned candidate is refused
/// at intake and on replay.
#[test]
fn guidance_and_revisions_for_non_retained_candidates_are_refused() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let (events, state, _retirement) = crate::suite::s3_retirements::retire(
        frontier.events,
        &frontier.state,
        frontier.kept,
        campaign_core::RetirementOrigin::Consumer,
    )?;
    let prune = campaign_core::prune_frontier(
        &state,
        crate::suite::s3_pruning::explicit(&state, vec![frontier.lowest]),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, prune)?;
    assert!(matches!(
        state
            .candidate(frontier.kept)
            .map_err(crate::suite::s3_fixture::err)?
            .status,
        campaign_core::CandidateStatus::Retired { .. }
    ));
    assert!(matches!(
        state
            .candidate(frontier.lowest)
            .map_err(crate::suite::s3_fixture::err)?
            .status,
        campaign_core::CandidateStatus::Pruned { .. }
    ));
    assert_guidance_refused(
        &events,
        &state,
        frontier.kept,
        &campaign_core::CampaignError::CandidateRetired,
    )?;
    assert_guidance_refused(
        &events,
        &state,
        frontier.lowest,
        &campaign_core::CampaignError::CandidatePruned,
    )?;
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    Ok(())
}

/// A prune event lists its candidates in canonical order: replay refuses a permuted list even
/// though the prune plan identity frames the sorted set.
#[test]
fn a_prune_event_lists_its_candidates_in_canonical_order() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::WIDE_BATCH)?;
    let targets = vec![frontier.middle, frontier.lowest];
    let event = crate::suite::s3_pruning::forged_prune(
        &frontier.state,
        crate::suite::s3_pruning::explicit(&frontier.state, targets),
    )?;
    let campaign_core::CampaignControlEvent::CandidatesPruned(mut permuted) = event.clone() else {
        return Err(String::from("expected a prune event"));
    };
    permuted.pruned_candidates.reverse();
    assert!(!permuted.pruned_candidates.is_sorted());
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &frontier.events,
            campaign_core::CampaignControlEvent::CandidatesPruned(permuted),
        )?,
        campaign_core::CampaignError::MalformedIdentity
    );
    crate::suite::s3_fixture::append(frontier.events, event)?;
    Ok(())
}

/// Replay requires `ControlBranch` and `Head` identities on a selection's publication fence, as
/// `plan_next` produces from a validated projection binding: a re-derived selection whose fence
/// ids have other kinds never replays.
#[test]
fn a_selection_publication_fence_names_a_control_branch_and_a_head() -> Result<(), String> {
    let (events, state, best, _lower) = super::guided_pair()?;
    let honest = crate::suite::selection::plan(&state)?;
    assert_eq!(honest.candidate_id, best);
    let mut foreign_branch = honest.clone();
    foreign_branch.publication_plan.control_branch_id =
        crate::suite::admission::xid(campaign_core::IdKind::Graph, b"fence-graph-branch");
    let foreign_branch = foreign_branch
        .derive()
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::selection::event(foreign_branch)?
        )?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut foreign_head = honest.clone();
    foreign_head.publication_plan.expected_head_id =
        crate::suite::admission::xid(campaign_core::IdKind::Finding, b"fence-finding-head");
    let foreign_head = foreign_head
        .derive()
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::selection::event(foreign_head)?
        )?,
        campaign_core::CampaignError::CrossedIdentity
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    crate::suite::selection::publish(events, honest)?;
    Ok(())
}
