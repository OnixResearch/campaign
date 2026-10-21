//! One Choregraph history event binds exactly one Campaign selection payload: the publication plan's
//! campaign and state are pinned by `SelectionDecision::derive`, so a payload that keeps the payload
//! identity but alters those fields never replays.

#[test]
fn one_history_event_validates_exactly_one_selection_payload() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let honest = crate::suite::selection::select_from_current(&current)?;
    let (history, _publication) = history.append(crate::suite::selection::event(&honest))?;
    let mut altered = honest.clone();
    altered.publication_plan.state_before_id =
        crate::suite::identity::derive(campaign_core::IdKind::State, b"audit-unbound-state");
    altered.publication_plan.campaign_id =
        crate::suite::identity::derive(campaign_core::IdKind::Campaign, b"audit-unbound-campaign");
    assert_ne!(altered, honest);
    assert_eq!(
        crate::suite::identity::observed(altered.clone().derive())?,
        honest
    );
    let mut swapped = crate::suite::history::Fixture {
        graph: history.graph.clone(),
        branch: history.branch.clone(),
        payloads: history.payloads.clone(),
    };
    let last = swapped
        .payloads
        .last_mut()
        .ok_or_else(|| String::from("payload"))?;
    last.control_event = crate::suite::selection::event(&altered);
    assert_eq!(
        swapped.validate().err(),
        Some(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::MalformedIdentity
        ))
    );
    history.current()?;
    Ok(())
}
