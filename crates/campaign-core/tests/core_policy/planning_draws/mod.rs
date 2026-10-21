//! Explicit exploratory entropy (`campaign.entropy`).

const THRESHOLD: u64 = 500_000;
const OTHER_THRESHOLD: u64 = 250_000;
const FORGED_BYTE: u8 = u8::MAX;
const EXPLOIT_DRAW: u64 = 900_000;
const EXPLORE_DRAW: u64 = 100;
const INDEX_DRAW: u64 = 5;
const DRAW_BYTES: usize = 8;
const INDEX_DRAW_END: usize = 16;
const RANK_TOP: i64 = 20;
const RANK_MIDDLE: i64 = 15;
const RANK_BOTTOM: i64 = 10;

struct Frontier {
    state: campaign_core::FrontierState,
    top: campaign_core::Id,
    bottom: campaign_core::Id,
}

fn frontier() -> Result<Frontier, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, top) =
        crate::suite::planning_support::guided(events, &state, b"top", RANK_TOP)?;
    let (events, state, _middle) =
        crate::suite::planning_support::guided(events, &state, b"middle", RANK_MIDDLE)?;
    let (_events, state, bottom) =
        crate::suite::planning_support::guided(events, &state, b"bottom", RANK_BOTTOM)?;
    Ok(Frontier { state, top, bottom })
}

fn exploratory(source: campaign_core::Id) -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::selection::profile(campaign_core::SelectionMode::Exploratory {
        threshold_parts_per_million: THRESHOLD,
        entropy_source_id: source,
        exploration_order: vec![crate::suite::constants::FIRST_RANK_DIMENSION],
    })
}

struct TicketFacts {
    campaign_id: campaign_core::Id,
    state_id: campaign_core::Id,
    profile_id: campaign_core::Id,
    source_id: campaign_core::Id,
    explore_draw: u64,
}

impl TicketFacts {
    fn exact(
        state: &campaign_core::FrontierState,
        profile: &campaign_core::SelectionProfile,
        explore_draw: u64,
    ) -> Self {
        Self {
            campaign_id: state.admission.campaign_id,
            state_id: state.state_id,
            profile_id: profile.profile_id,
            source_id: state.admission.entropy_profile_id,
            explore_draw,
        }
    }

    fn ticket(&self) -> Result<campaign_core::EntropyTicket, String> {
        let mut bytes = [0; campaign_core::DIGEST_LENGTH];
        bytes[..DRAW_BYTES].copy_from_slice(&self.explore_draw.to_le_bytes());
        bytes[DRAW_BYTES..INDEX_DRAW_END].copy_from_slice(&INDEX_DRAW.to_le_bytes());
        crate::suite::planning_support::observed(
            campaign_core::EntropyTicket {
                entropy_id: campaign_core::Id::absent(),
                campaign_id: self.campaign_id,
                state_id: self.state_id,
                profile_id: self.profile_id,
                source_id: self.source_id,
                counter: crate::suite::constants::ENTROPY_COUNTER,
                bytes,
            }
            .derive(),
        )
    }
}

fn plan_exploratory(
    frontier: &Frontier,
    explore_draw: u64,
) -> Result<campaign_core::DecisionPlan, String> {
    let profile = exploratory(frontier.state.admission.entropy_profile_id)?;
    let ticket = TicketFacts::exact(&frontier.state, &profile, explore_draw).ticket()?;
    crate::suite::planning_support::observed(crate::suite::planning_support::plan_with(
        &frontier.state,
        profile,
        Some(ticket),
    ))
}

#[test]
// r[verify campaign.entropy.valid]
fn equal_exploratory_inputs_return_equal_exploit_and_explore_choices() -> Result<(), String> {
    let frontier = frontier()?;
    let exploit = plan_exploratory(&frontier, EXPLOIT_DRAW)?;
    assert_eq!(exploit, plan_exploratory(&frontier, EXPLOIT_DRAW)?);
    let exploit_selection = crate::suite::planning_support::selected(&exploit)?;
    assert_eq!(exploit_selection.candidate_id, frontier.top);
    // Explore branch: declared order top(20), middle(15), bottom(10); index draw 5 % 3 = 2.
    let explore = plan_exploratory(&frontier, EXPLORE_DRAW)?;
    assert_eq!(explore, plan_exploratory(&frontier, EXPLORE_DRAW)?);
    let explore_selection = crate::suite::planning_support::selected(&explore)?;
    assert_eq!(explore_selection.candidate_id, frontier.bottom);
    let profile = exploratory(frontier.state.admission.entropy_profile_id)?;
    let ticket = TicketFacts::exact(&frontier.state, &profile, EXPLORE_DRAW).ticket()?;
    assert_eq!(explore_selection.entropy_id, Some(ticket.entropy_id));
    assert_ne!(
        exploit_selection.selection_id,
        explore_selection.selection_id
    );
    Ok(())
}

#[test]
// r[verify campaign.entropy.invalid]
fn a_missing_or_crossed_ticket_emits_no_decision() -> Result<(), String> {
    let frontier = frontier()?;
    let state = &frontier.state;
    let profile = exploratory(state.admission.entropy_profile_id)?;
    assert_eq!(
        crate::suite::planning_support::denied(crate::suite::planning_support::plan_with(
            state,
            profile.clone(),
            None
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    let other_source =
        crate::suite::admission::xid(campaign_core::IdKind::EntropyProfile, b"other-source");
    let crossed_facts = [
        TicketFacts {
            source_id: other_source,
            ..TicketFacts::exact(state, &profile, EXPLORE_DRAW)
        },
        TicketFacts {
            campaign_id: crate::suite::admission::xid(campaign_core::IdKind::Campaign, b"other"),
            ..TicketFacts::exact(state, &profile, EXPLORE_DRAW)
        },
        TicketFacts {
            state_id: crate::suite::admission::xid(campaign_core::IdKind::State, b"other"),
            ..TicketFacts::exact(state, &profile, EXPLORE_DRAW)
        },
        TicketFacts {
            profile_id: crate::suite::selection::profile(
                campaign_core::SelectionMode::Exploratory {
                    threshold_parts_per_million: OTHER_THRESHOLD,
                    entropy_source_id: state.admission.entropy_profile_id,
                    exploration_order: vec![crate::suite::constants::FIRST_RANK_DIMENSION],
                },
            )?
            .profile_id,
            ..TicketFacts::exact(state, &profile, EXPLORE_DRAW)
        },
    ];
    for facts in crossed_facts {
        assert_eq!(
            crate::suite::planning_support::denied(crate::suite::planning_support::plan_with(
                state,
                profile.clone(),
                Some(facts.ticket()?)
            ))?,
            campaign_core::CampaignError::InvalidEntropy
        );
    }
    Ok(())
}

#[test]
// r[verify campaign.entropy.invalid]
fn a_forged_ticket_or_undeclared_source_emits_no_decision() -> Result<(), String> {
    let frontier = frontier()?;
    let state = &frontier.state;
    let profile = exploratory(state.admission.entropy_profile_id)?;
    let other_source =
        crate::suite::admission::xid(campaign_core::IdKind::EntropyProfile, b"other-source");
    let mut forged = TicketFacts::exact(state, &profile, EXPLORE_DRAW).ticket()?;
    assert_ne!(forged.bytes[0], FORGED_BYTE);
    forged.bytes[0] = FORGED_BYTE;
    assert_eq!(
        crate::suite::planning_support::denied(crate::suite::planning_support::plan_with(
            state,
            profile,
            Some(forged)
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    let undeclared_source = exploratory(other_source)?;
    let matching_ticket = TicketFacts {
        source_id: other_source,
        ..TicketFacts::exact(state, &undeclared_source, EXPLORE_DRAW)
    }
    .ticket()?;
    assert_eq!(
        crate::suite::planning_support::denied(crate::suite::planning_support::plan_with(
            state,
            undeclared_source,
            Some(matching_ticket)
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    Ok(())
}

#[test]
fn ranked_mode_rejects_any_supplied_ticket() -> Result<(), String> {
    let frontier = frontier()?;
    let ranked = crate::suite::selection::ranked_profile()?;
    let ticket = TicketFacts::exact(&frontier.state, &ranked, EXPLORE_DRAW).ticket()?;
    assert_eq!(
        crate::suite::planning_support::denied(crate::suite::planning_support::plan_with(
            &frontier.state,
            ranked.clone(),
            Some(ticket)
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    let plan = crate::suite::planning_support::observed(
        crate::suite::planning_support::plan_with(&frontier.state, ranked, None),
    )?;
    let selection = crate::suite::planning_support::selected(&plan)?;
    assert_eq!(selection.candidate_id, frontier.top);
    assert_eq!(selection.entropy_id, None);
    Ok(())
}
