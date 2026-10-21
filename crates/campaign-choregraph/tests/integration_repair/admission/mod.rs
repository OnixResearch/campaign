pub(crate) fn initialized() -> Result<crate::suite::history::Fixture, String> {
    let exploration = exploration_root()?;
    let before = campaign_choregraph::BranchRef {
        name: String::from("repair-control"),
        head: exploration.event,
        generation: crate::suite::constants::ROOT_GENERATION,
    };
    let projection =
        crate::suite::identity::observed(campaign_choregraph::derive_projection_binding(
            &exploration.graph,
            &before,
            crate::suite::identity::schema(),
            crate::suite::identity::adapter(),
            crate::suite::identity::domain(),
        ))?;
    let admission = configure(&projection, exploration.event)?;
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &admission,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        None,
    ))?;
    let root = crate::suite::identity::observed(exploration.graph.admit(draft))?;
    let history = crate::suite::history::Fixture {
        graph: root.graph,
        branch: choregraph_history::BranchRef {
            name: before.name,
            head: root.event,
            generation: crate::suite::constants::ROOT_BRANCH_GENERATION,
        },
        payloads: vec![campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: root.event,
            control_event: admission,
        }],
    };
    let current = history.current()?;
    assert_eq!(current.state().control_event_count, 1);
    assert_eq!(
        current.state().admission.resource_budgets[0].amount,
        crate::suite::constants::RESOURCE_BUDGET
    );
    Ok(history)
}
/// The exploration graph every suite control root is admitted to, holding only its exploration
/// root.
pub(crate) fn exploration_root() -> Result<choregraph_history::Admission, String> {
    let limits = crate::suite::identity::observed(choregraph_history::HistoryLimits::new(
        crate::suite::constants::MAX_EVENTS,
        crate::suite::constants::HISTORY_MAX_PARENTS,
        crate::suite::constants::MAX_METADATA,
        crate::suite::constants::MAX_LABEL_BYTES,
        crate::suite::constants::MAX_EVENTS,
    ))?;
    let empty = crate::suite::identity::observed(choregraph_history::HistoryGraph::new(limits))?;
    let exploration = crate::suite::identity::observed(
        empty.admit(choregraph_history::EventDraft::v1(
            choregraph_history::EventClass::Application,
            choregraph_history::Digest::from_bytes(
                crate::suite::identity::derive(
                    campaign_core::IdKind::EventPayload,
                    b"repair-exploration-root",
                )
                .digest,
            ),
            Vec::new(),
            choregraph_history::Digest::from_bytes(crate::suite::identity::schema().digest),
        )),
    )?;
    assert_eq!(exploration.graph.len(), 1);
    assert!(exploration.graph.contains(exploration.event));
    Ok(exploration)
}
fn exploration_bounds() -> Result<campaign_core::ExplorationBounds, String> {
    let bounds = crate::suite::identity::observed(
        campaign_core::ExplorationBounds {
            bounds_id: campaign_core::Id::absent(),
            max_roots: crate::suite::constants::MODEL_BOUND,
            max_candidates: crate::suite::constants::MODEL_BOUND,
            max_candidate_batch: crate::suite::constants::MODEL_BOUND,
            max_retained_candidates: crate::suite::constants::MODEL_BOUND,
            max_selections: crate::suite::constants::MODEL_BOUND,
            max_guidance_records: crate::suite::constants::MODEL_BOUND,
            max_outstanding: crate::suite::constants::MODEL_BOUND,
            max_depth: crate::suite::constants::MODEL_BOUND,
            max_child_moments: crate::suite::constants::MODEL_BOUND,
            max_derived_records: crate::suite::constants::MODEL_BOUND,
            max_rank_dimensions: crate::suite::constants::MODEL_BOUND,
            max_rank_magnitude: crate::suite::constants::RANK_MAGNITUDE,
            max_findings: crate::suite::constants::MODEL_BOUND,
            max_control_events: crate::suite::constants::MODEL_BOUND,
            max_resource_dimensions: crate::suite::constants::MODEL_BOUND,
            max_operation_schemas: crate::suite::constants::MODEL_BOUND,
            max_prune_batch: crate::suite::constants::MODEL_BOUND,
        }
        .derive(),
    )?;
    assert_ne!(bounds.bounds_id, campaign_core::Id::absent());
    assert_eq!(bounds.max_roots, crate::suite::constants::MODEL_BOUND);
    Ok(bounds)
}
fn configure(
    projection: &campaign_core::ProjectionBinding,
    moment: choregraph_history::EventId,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let bounds = exploration_bounds()?;
    let admission = crate::suite::identity::observed(campaign_core::admit_campaign(
        campaign_core::CampaignAdmission {
            campaign_id: campaign_core::Id::absent(),
            schema_version: campaign_core::CAMPAIGN_SCHEMA_VERSION,
            projection: projection.clone(),
            policy_id: crate::suite::identity::derive(
                campaign_core::IdKind::Policy,
                b"repair-loop-policy",
            ),
            guidance_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::GuidanceProfile,
                b"repair-loop-guidance",
            ),
            entropy_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::EntropyProfile,
                b"repair-loop-entropy",
            ),
            budget_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::BudgetProfile,
                b"repair-loop-budget",
            ),
            adapter_id: crate::suite::identity::adapter(),
            control_domain_id: crate::suite::identity::domain(),
            bounds,
            resource_budgets: vec![campaign_core::ResourceAmount {
                dimension_id: crate::suite::identity::derive(
                    campaign_core::IdKind::ResourceDimension,
                    b"repair-loop-resource",
                ),
                amount: crate::suite::constants::RESOURCE_BUDGET,
            }],
            roots: vec![campaign_core::RootMoment {
                moment_id: campaign_core::Id {
                    kind: campaign_core::IdKind::RootMoment,
                    digest: moment.bytes(),
                },
                projection_id: projection.projection_id,
                adapter_id: crate::suite::identity::adapter(),
            }],
            operation_schema_ids: vec![crate::suite::identity::derive(
                campaign_core::IdKind::OperationSchema,
                b"repair-loop-operation-schema",
            )],
        },
    ))?;
    assert!(admission.state_before_id().is_none());
    assert_eq!(admission.event_id().kind, campaign_core::IdKind::Event);
    Ok(admission)
}
