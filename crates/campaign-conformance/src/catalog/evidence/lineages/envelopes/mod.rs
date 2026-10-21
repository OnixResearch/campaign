//! Control events whose Choregraph envelopes are forged: suffix events with another adapter's
//! metadata, a non-application event class, a foreign schema identity, and a foreign projection
//! identity; and the campaign's control root admitted under foreign envelope identities.

/// Frame domain of suite-owned foreign envelope identities.
const FOREIGN_DOMAIN: &[u8] = b"campaign-conformance-foreign-envelope";

/// Name of the adapter whose metadata a forged envelope carries.
const FOREIGN_ADAPTER: &str = "foreign-envelope-adapter";

/// Names of the foreign projection schema and control domain a foreign control root binds.
const FOREIGN_SCHEMA: &str = "foreign-envelope-projection-schema";
const FOREIGN_CONTROL_DOMAIN: &str = "foreign-envelope-control-domain";

/// The control root of `base` (its admission event, `state` being a state of that campaign)
/// admitted again next to its head under foreign projection schema, adapter, and control-domain identities, with the control
/// branch moved onto it at the admitted generation; and the binding of that history under the
/// foreign identities, as a host holding them expects it. Every envelope check against the
/// host's own identities passes; only the admission's identities tell it apart.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the construction.
pub(in crate::catalog::evidence) fn foreign_root(
    (base, state): (
        &crate::harness::oracle::HistoryView,
        &campaign_core::FrontierState,
    ),
) -> crate::verdicts::ConformanceResult<(
    crate::harness::oracle::HistoryView,
    campaign_core::ProjectionBinding,
)> {
    let root = base
        .payloads
        .first()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the session published a control root",
        })?;
    let schema = crate::harness::suite_id(campaign_core::IdKind::Projection, FOREIGN_SCHEMA)?;
    let adapter = crate::harness::suite_id(campaign_core::IdKind::Adapter, FOREIGN_ADAPTER)?;
    let domain =
        crate::harness::suite_id(campaign_core::IdKind::ControlDomain, FOREIGN_CONTROL_DOMAIN)?;
    let draft = campaign_choregraph::control_event_draft(
        &root.control_event,
        schema,
        adapter,
        domain,
        None,
    )?;
    let admitted = base.graph.admit(draft)?;
    let history = crate::harness::oracle::HistoryView {
        graph: admitted.graph,
        branch: campaign_choregraph::BranchRef {
            head: admitted.event,
            generation: state
                .admission
                .projection
                .generation
                .checked_add(1)
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
            ..base.branch.clone()
        },
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads: alloc::vec![campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: admitted.event,
            control_event: root.control_event.clone(),
        }],
    };
    let binding = campaign_choregraph::derive_projection_binding(
        &history.graph,
        &history.branch,
        schema,
        adapter,
        domain,
    )?;
    debug_assert_ne!(admitted.event, root.history_event_id);
    Ok((history, binding))
}

/// `base` extended by `event`, an event Campaign admits against its state, whose admitted
/// envelope carries, in turn, another adapter's metadata, a machine event class, a foreign schema
/// identity, and a foreign projection identity.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the construction.
pub(in crate::catalog::evidence) fn forged_suffixes(
    session: &crate::harness::Session<'_, '_>,
    base: &crate::harness::oracle::HistoryView,
    event: &campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<[crate::harness::oracle::HistoryView; 4]> {
    let foreign_adapter =
        crate::harness::suite_id(campaign_core::IdKind::Adapter, FOREIGN_ADAPTER)?;
    let foreign = campaign_choregraph::control_event_draft(
        event,
        session.identities.projection_schema_id,
        foreign_adapter,
        session.identities.control_domain_id,
        Some(base.branch.head),
    )?;
    let forge = |edit: &dyn Fn(&mut campaign_choregraph::EventDraft)| {
        super::extended_with(session, base, event.clone(), |draft| edit(draft))
            .map(|(history, _)| history)
    };
    let metadata = forge(&|draft| draft.metadata.clone_from(&foreign.metadata))?;
    let class = forge(&|draft| draft.class = campaign_choregraph::EventClass::MachineV1)?;
    let schema = forge(&|draft| {
        draft.schema_identity = campaign_choregraph::Digest::derive(FOREIGN_DOMAIN, b"schema");
    })?;
    let projection = forge(&|draft| {
        draft.projection_identity = Some(campaign_choregraph::Digest::derive(
            FOREIGN_DOMAIN,
            b"projection",
        ));
    })?;
    debug_assert_eq!(
        metadata.payloads.len(),
        base.payloads.len().saturating_add(1)
    );
    debug_assert_ne!(metadata.branch.head, class.branch.head);
    Ok([metadata, class, schema, projection])
}

/// Another campaign's control root (the admission of the campaign `tip` records, with one more
/// budget unit per dimension) written into the graph of `tip` under the same envelope, as a writer
/// who ignores Campaign's refusal of a second campaign on one graph and envelope would: the graph
/// holding it and the root's payload. Campaign's own root admission refuses it with
/// `SharedControlGraph`.
///
/// # Errors
///
/// Returns `Diverged` when `tip` does not start with an admission or Campaign admits the second
/// root, and the Campaign or Choregraph error of the construction.
pub(in crate::catalog::evidence) fn second_root(
    session: &crate::harness::Session<'_, '_>,
    tip: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<(
    campaign_choregraph::HistoryGraph,
    campaign_choregraph::ControlPayloadEnvelope,
)> {
    let Some(campaign_core::CampaignControlEvent::CampaignAdmitted { admission, .. }) =
        tip.payloads.first().map(|payload| &payload.control_event)
    else {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: "the tip starts with the campaign's admission",
        });
    };
    let mut other = admission.clone();
    other.campaign_id = campaign_core::Id::absent();
    for budget in &mut other.resource_budgets {
        budget.amount = budget
            .amount
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    }
    let event = campaign_core::admit_campaign(other)?;
    let refused = campaign_choregraph::admit_control_root(
        &tip.graph,
        event.clone(),
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    );
    super::super::super::common::require(
        matches!(
            refused,
            Err(campaign_choregraph::AdapterError::SharedControlGraph { .. })
        ),
        "Campaign refuses a second campaign's root on a graph and envelope it holds",
    )?;
    let draft = campaign_choregraph::control_event_draft(
        &event,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
        None,
    )?;
    let admitted = tip.graph.admit(draft)?;
    debug_assert!(admitted.graph.contains(admitted.event));
    Ok((
        admitted.graph,
        campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: admitted.event,
            control_event: event,
        },
    ))
}
