//! Exact projection binding at the adapter and caller-owned lineage choices
//! (`campaign.history_projection`, `campaign.replay_boundary.valid`).

const EXPLORATION_ROLE: &str = "exploration-parent";
const REPLAY_BRANCH: &str = "consumer-replay";
const PRIOR_PAYLOAD_OFFSET: usize = 2;

fn validate_with(
    history: &crate::suite::history::Fixture,
    branch: &choregraph_history::BranchRef,
    parent_choices: &choregraph_history::ParentChoices,
    expected_binding: campaign_core::ProjectionBinding,
) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
    campaign_choregraph::validate_control_projection(
        &campaign_choregraph::ControlProjectionRequest {
            graph: &history.graph,
            branch,
            parent_choices,
            maximum_events: crate::suite::constants::MAX_EVENTS,
            expected_binding,
            payloads: &history.payloads,
        },
    )
}

fn binding_for(
    graph: &choregraph_history::HistoryGraph,
    branch: &choregraph_history::BranchRef,
    schema: campaign_core::Id,
) -> Result<campaign_core::ProjectionBinding, String> {
    crate::suite::identity::observed(campaign_choregraph::derive_projection_binding(
        graph,
        branch,
        schema,
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
    ))
}

fn exploration_event(
    graph: &choregraph_history::HistoryGraph,
    label: &[u8],
    parents: Vec<choregraph_history::EventId>,
) -> Result<choregraph_history::Admission, String> {
    let mut links: Vec<choregraph_history::ParentLink> = parents
        .into_iter()
        .map(|event| choregraph_history::ParentLink {
            role: String::from(EXPLORATION_ROLE),
            event,
        })
        .collect();
    links.sort();
    crate::suite::identity::observed(graph.admit(choregraph_history::EventDraft::v1(
        choregraph_history::EventClass::Application,
        choregraph_history::Digest::from_bytes(
            crate::suite::identity::derive(campaign_core::IdKind::EventPayload, label).digest,
        ),
        links,
        choregraph_history::Digest::from_bytes(crate::suite::identity::schema().digest),
    )))
}

#[test]
// r[verify campaign.history_projection.stale]
fn a_stale_wrong_head_or_foreign_graph_binding_is_denied() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let exact = binding_for(
        &history.graph,
        &history.branch,
        crate::suite::identity::schema(),
    )?;
    let current = crate::suite::identity::observed(validate_with(
        &history,
        &history.branch,
        &choregraph_history::ParentChoices::new(),
        exact,
    ))?;
    assert_eq!(current, history.current()?);
    let stale_branch = choregraph_history::BranchRef {
        generation: crate::suite::constants::ROOT_BRANCH_GENERATION,
        ..history.branch.clone()
    };
    let prior_head = history
        .payloads
        .len()
        .checked_sub(PRIOR_PAYLOAD_OFFSET)
        .and_then(|index| history.payloads.get(index))
        .map(|payload| payload.history_event_id)
        .ok_or_else(|| String::from("the fixture publishes more than one control event"))?;
    let wrong_head = choregraph_history::BranchRef {
        head: prior_head,
        ..history.branch.clone()
    };
    let older_graph = crate::suite::admission::initialized()?.graph;
    let mismatched = [
        binding_for(
            &history.graph,
            &stale_branch,
            crate::suite::identity::schema(),
        )?,
        binding_for(
            &history.graph,
            &wrong_head,
            crate::suite::identity::schema(),
        )?,
        binding_for(
            &older_graph,
            &history.branch,
            crate::suite::identity::schema(),
        )?,
    ];
    let no_choices = choregraph_history::ParentChoices::new();
    for expected in mismatched {
        assert_eq!(
            validate_with(&history, &history.branch, &no_choices, expected),
            Err(campaign_choregraph::AdapterError::ProjectionBindingMismatch)
        );
    }
    Ok(())
}

#[test]
fn an_unsupported_schema_binding_is_denied() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let unsupported = binding_for(
        &history.graph,
        &history.branch,
        crate::suite::identity::derive(campaign_core::IdKind::Projection, b"unsupported-schema"),
    )?;
    assert_eq!(
        validate_with(
            &history,
            &history.branch,
            &choregraph_history::ParentChoices::new(),
            unsupported
        ),
        Err(campaign_choregraph::AdapterError::SchemaIdentityMismatch)
    );
    Ok(())
}

/// Adds a consumer exploration merge (`root -> left`, `root -> right`, `left + right -> merge`).
fn consumer_merge(
    history: &mut crate::suite::history::Fixture,
    root: choregraph_history::EventId,
) -> Result<
    (
        choregraph_history::EventId,
        choregraph_history::EventId,
        choregraph_history::EventId,
    ),
    String,
> {
    let left = exploration_event(&history.graph, b"replay-left", vec![root])?;
    let right = exploration_event(&left.graph, b"replay-right", vec![root])?;
    let merge = exploration_event(&right.graph, b"replay-merge", vec![left.event, right.event])?;
    assert_ne!(left.event, right.event);
    assert!(merge.graph.contains(root));
    history.graph = merge.graph;
    Ok((left.event, right.event, merge.event))
}

#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.replay_boundary.valid]
fn an_ambiguous_lineage_is_resolved_only_by_explicit_consumer_choices() -> Result<(), String> {
    let mut history = crate::suite::candidate::selectable()?;
    let before = history.current()?;
    let root = choregraph_history::EventId::new(choregraph_history::Digest::from_bytes(
        before.state().admission.roots[crate::suite::constants::FIRST_ROOT_INDEX]
            .moment_id
            .digest,
    ));
    let (left, right, merge) = consumer_merge(&mut history, root)?;
    let replay_branch = choregraph_history::BranchRef {
        name: String::from(REPLAY_BRANCH),
        head: merge,
        generation: crate::suite::constants::ROOT_GENERATION,
    };
    let replay_binding = binding_for(
        &history.graph,
        &replay_branch,
        crate::suite::identity::schema(),
    )?;
    assert_eq!(
        validate_with(
            &history,
            &replay_branch,
            &choregraph_history::ParentChoices::new(),
            replay_binding.clone(),
        ),
        Err(campaign_choregraph::AdapterError::History(
            choregraph_history::HistoryError::AmbiguousLineage { event: merge }
        ))
    );
    let consumer_choices = [
        (
            left,
            choregraph_history::ParentChoices::from([(merge, left)]),
        ),
        (
            right,
            choregraph_history::ParentChoices::from([(merge, right)]),
        ),
    ];
    for (chosen, choices) in &consumer_choices {
        let chosen = *chosen;
        let lineage = crate::suite::identity::observed(choregraph_history::linear_lineage(
            &history.graph,
            merge,
            choices,
            crate::suite::constants::MAX_EVENTS,
        ))?;
        assert_eq!(lineage.events(), [root, chosen, merge]);
        assert_eq!(
            validate_with(&history, &replay_branch, choices, replay_binding.clone()),
            Err(campaign_choregraph::AdapterError::ProjectionPayloadMismatch)
        );
        let control = crate::suite::identity::observed(validate_with(
            &history,
            &history.branch,
            choices,
            binding_for(
                &history.graph,
                &history.branch,
                crate::suite::identity::schema(),
            )?,
        ))?;
        assert_eq!(control.state(), before.state());
        assert_eq!(control.control_events(), before.control_events());
    }
    Ok(())
}
