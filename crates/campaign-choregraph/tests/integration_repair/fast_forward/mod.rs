//! Fast-forward control lineage: only the admission's named branch, advanced exactly one
//! generation per control event, validates as current history.

mod lagging;

/// Moves the fixture's branch back to `head` (a generation-advancing compare-and-set) and drops
/// the payloads after it, as a host rewinding its control branch would.
fn rewound_to(
    history: &crate::suite::history::Fixture,
    head: choregraph_history::EventId,
) -> Result<crate::suite::history::Fixture, String> {
    let rewind = crate::suite::identity::observed(choregraph_history::plan_branch_advance(
        &history.graph,
        &history.branch,
        history.branch.generation,
        history.branch.head,
        head,
    ))?;
    let mut payloads = history.payloads.clone();
    payloads.pop();
    Ok(crate::suite::history::Fixture {
        graph: history.graph.clone(),
        branch: rewind.after,
        payloads,
    })
}

#[test]
// r[verify campaign.history_projection.stale]
fn a_rewound_control_branch_cannot_reopen_a_stopped_campaign() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let before_stop = history.current()?;
    let pre_stop_head = history.branch.head;
    let stop = crate::suite::integrity::consumer_stop(before_stop.state(), b"audit-terminal-stop")?;
    let (stopped, _stop_publication) = history.append(stop)?;
    assert!(stopped.current()?.state().stopped.is_some());
    let rewound = rewound_to(&stopped, pre_stop_head)?;
    assert!(rewound.branch.generation > stopped.branch.generation);
    assert_eq!(
        rewound.validate(),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let reopened = crate::suite::selection::select_from_current(&before_stop)?;
    assert_eq!(
        rewound.publish(crate::suite::selection::event(&reopened)),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert!(stopped.current()?.state().stopped.is_some());
    Ok(())
}

#[test]
fn a_rewound_control_branch_cannot_admit_a_second_settlement() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let selection = crate::suite::selection::select_from_current(&history.current()?)?;
    let (history, _publication) = history.append(crate::suite::selection::event(&selection))?;
    let selected = history.current()?;
    let head_after_selection = history.branch.head;
    let first = crate::suite::identity::observed(campaign_core::admit_result(
        selected.state(),
        crate::suite::selection::result_for(&selected, &selection),
    ))?;
    let (settled, _p) = history.append(first)?;
    assert!(settled.current()?.state().outstanding.is_empty());
    let rewound = rewound_to(&settled, head_after_selection)?;
    assert_eq!(
        rewound.validate(),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let mut different = crate::suite::selection::result_for(&selected, &selection);
    different.payload_id =
        crate::suite::identity::derive(campaign_core::IdKind::EventPayload, b"audit-retry-payload");
    let second =
        crate::suite::identity::observed(campaign_core::admit_result(selected.state(), different))?;
    assert_eq!(
        rewound.publish(second),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert!(settled.current()?.state().outstanding.is_empty());
    Ok(())
}

#[test]
fn only_the_admissions_named_branch_is_the_control_branch() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let renamed = crate::suite::history::Fixture {
        graph: history.graph.clone(),
        branch: choregraph_history::BranchRef {
            name: String::from("foreign-control"),
            ..history.branch.clone()
        },
        payloads: history.payloads.clone(),
    };
    assert_eq!(
        renamed.validate(),
        Err(campaign_choregraph::AdapterError::ControlBranchMismatch)
    );
    let selection = crate::suite::selection::select_from_current(&current)?;
    assert_eq!(
        renamed.publish(crate::suite::selection::event(&selection)),
        Err(campaign_choregraph::AdapterError::ControlBranchMismatch)
    );
    let lineage_len =
        u64::try_from(current.control_events().len()).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        current
            .state()
            .admission
            .projection
            .generation
            .checked_add(lineage_len),
        Some(current.binding().generation)
    );
    Ok(())
}
