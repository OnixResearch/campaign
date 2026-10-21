//! Campaign application-envelope construction and exact validation.

mod payload;
#[cfg(test)]
mod tests;

const CONTROL_PARENT_ROLE: &str = "campaign-control-parent";
const METADATA_ADAPTER_KEY: &str = "campaign.adapter";
const METADATA_CONTROL_DOMAIN_KEY: &str = "campaign.control_domain";
const METADATA_CONTROL_EVENT_KEY: &str = "campaign.control_event";
const ROOT_PARENT_COUNT: usize = 0;
const LINEAR_PARENT_COUNT: usize = 1;
const EXACT_METADATA_COUNT: usize = 3;
const CONTROL_EVENT_METADATA_INDEX: usize = 2;

/// Creates the exact Choregraph application draft for a Campaign control event.
///
/// # Errors
///
/// Returns an error if the schema, adapter, or control-domain identity has the wrong kind.
///
/// # Panics
///
/// Panics if payload framing changes its identity kind or parent construction changes cardinality.
pub fn control_event_draft(
    event: &campaign_core::CampaignControlEvent,
    projection_schema_id: campaign_core::Id,
    adapter_id: campaign_core::Id,
    control_domain_id: campaign_core::Id,
    parent: Option<choregraph_history::EventId>,
) -> crate::contracts::AdapterResult<choregraph_history::EventDraft> {
    projection_schema_id.require_kind(campaign_core::IdKind::Projection)?;
    adapter_id.require_kind(campaign_core::IdKind::Adapter)?;
    control_domain_id.require_kind(campaign_core::IdKind::ControlDomain)?;
    let payload_id = derive_control_payload_id(event)?;
    let parents = parent.map_or_else(alloc::vec::Vec::new, |event_id| {
        alloc::vec![choregraph_history::ParentLink {
            role: alloc::string::String::from(CONTROL_PARENT_ROLE),
            event: event_id,
        }]
    });
    assert_eq!(parents.len(), usize::from(parent.is_some()));
    assert_eq!(payload_id.kind, campaign_core::IdKind::EventPayload);
    Ok(choregraph_history::EventDraft {
        version: choregraph_history::HISTORY_EVENT_VERSION_V1,
        class: choregraph_history::EventClass::Application,
        payload_identity: crate::identity::digest_from_id(payload_id),
        parents,
        schema_identity: crate::identity::digest_from_id(projection_schema_id),
        projection_identity: Some(crate::identity::digest_from_id(control_domain_id)),
        metadata: control_metadata(event.event_id(), adapter_id, control_domain_id),
    })
}

pub(crate) fn derive_control_payload_id(
    event: &campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::Id> {
    payload::derive_id(event)
}

pub(crate) fn validate_history_content(
    graph: &choregraph_history::HistoryGraph,
    history_event_id: choregraph_history::EventId,
    control_event: &campaign_core::CampaignControlEvent,
    binding: &campaign_core::ProjectionBinding,
    expected_parent: Option<choregraph_history::EventId>,
) -> crate::contracts::AdapterResult<()> {
    let history_event = graph
        .event(history_event_id)
        .ok_or(crate::contracts::AdapterError::ProjectionPayloadMismatch)?;
    let content = history_event.content();
    if content.class != choregraph_history::EventClass::Application {
        return Err(crate::contracts::AdapterError::NonApplicationEnvelope);
    }
    if content.payload_identity
        != crate::identity::digest_from_id(derive_control_payload_id(control_event)?)
    {
        return Err(crate::contracts::AdapterError::PayloadIdentityMismatch);
    }
    if content.schema_identity != crate::identity::digest_from_id(binding.projection_schema_id) {
        return Err(crate::contracts::AdapterError::SchemaIdentityMismatch);
    }
    if content.projection_identity
        != Some(crate::identity::digest_from_id(binding.control_domain_id))
    {
        return Err(crate::contracts::AdapterError::ProjectionIdentityMismatch);
    }
    if content.metadata
        != control_metadata(
            control_event.event_id(),
            binding.adapter_id,
            binding.control_domain_id,
        )
    {
        return Err(crate::contracts::AdapterError::MetadataMismatch);
    }
    match expected_parent {
        Some(parent) => validate_linear_parent(&content.parents, parent)?,
        None => validate_root_parent(&content.parents)?,
    }
    assert_eq!(content.metadata.len(), EXACT_METADATA_COUNT);
    assert_eq!(
        content.parents.len(),
        usize::from(expected_parent.is_some())
    );
    Ok(())
}

/// Whether `content` is a Campaign control event under `binding`'s projection schema, adapter, and
/// control domain (the envelope), whatever its parents.
// r[impl campaign.publication_fence.pending]
pub(crate) fn is_control_event(
    content: &choregraph_history::EventDraft,
    binding: &campaign_core::ProjectionBinding,
) -> bool {
    content.class == choregraph_history::EventClass::Application
        && content.schema_identity == crate::identity::digest_from_id(binding.projection_schema_id)
        && content.projection_identity
            == Some(crate::identity::digest_from_id(binding.control_domain_id))
        && control_event_id(content, binding).is_some()
}

/// Whether `content` is a Campaign control root under `binding`'s envelope: a control event
/// without parents, the first event of some campaign's control lineage.
pub(crate) fn is_control_root(
    content: &choregraph_history::EventDraft,
    binding: &campaign_core::ProjectionBinding,
) -> bool {
    content.parents.len() == ROOT_PARENT_COUNT && is_control_event(content, binding)
}

/// The parent of a Campaign control draft under `binding`'s envelope whose only parent is one
/// control-parent link, or `None` for any other event.
// r[impl campaign.publication_fence.pending]
pub(crate) fn control_parent(
    content: &choregraph_history::EventDraft,
    binding: &campaign_core::ProjectionBinding,
) -> Option<choregraph_history::EventId> {
    let [link] = content.parents.as_slice() else {
        return None;
    };
    (link.role == CONTROL_PARENT_ROLE && is_control_event(content, binding)).then_some(link.event)
}

/// Whether `content` is a Campaign control draft under `binding`'s projection schema, adapter, and
/// control domain whose only parent is the control-parent link to `head`.
// r[impl campaign.publication_fence.pending]
pub(crate) fn is_control_child(
    content: &choregraph_history::EventDraft,
    binding: &campaign_core::ProjectionBinding,
    head: choregraph_history::EventId,
) -> bool {
    control_parent(content, binding) == Some(head)
}

/// The Campaign control-event identity a control draft's metadata names under `binding`'s adapter
/// and control domain, or `None` for any other metadata.
// r[impl campaign.publication_fence.pending]
pub(crate) fn control_event_id(
    content: &choregraph_history::EventDraft,
    binding: &campaign_core::ProjectionBinding,
) -> Option<campaign_core::Id> {
    let named = content.metadata.get(CONTROL_EVENT_METADATA_INDEX)?;
    let control_event_id =
        crate::identity::id_from_digest(campaign_core::IdKind::Event, named.value);
    let expected = control_metadata(
        control_event_id,
        binding.adapter_id,
        binding.control_domain_id,
    );
    debug_assert_eq!(expected.len(), EXACT_METADATA_COUNT);
    (content.metadata == expected).then_some(control_event_id)
}

const fn validate_root_parent(
    parents: &[choregraph_history::ParentLink],
) -> crate::contracts::AdapterResult<()> {
    if parents.len() == ROOT_PARENT_COUNT {
        return Ok(());
    }
    Err(crate::contracts::AdapterError::ParentLineageMismatch)
}

fn validate_linear_parent(
    parents: &[choregraph_history::ParentLink],
    parent: choregraph_history::EventId,
) -> crate::contracts::AdapterResult<()> {
    if parents.len() != LINEAR_PARENT_COUNT {
        return Err(crate::contracts::AdapterError::ParentLineageMismatch);
    }
    let link = parents
        .first()
        .ok_or(crate::contracts::AdapterError::ParentLineageMismatch)?;
    if link.role == CONTROL_PARENT_ROLE && link.event == parent {
        return Ok(());
    }
    Err(crate::contracts::AdapterError::ParentLineageMismatch)
}

fn control_metadata(
    control_event_id: campaign_core::Id,
    adapter_id: campaign_core::Id,
    control_domain_id: campaign_core::Id,
) -> alloc::vec::Vec<choregraph_history::MetadataIdentity> {
    alloc::vec![
        choregraph_history::MetadataIdentity {
            key: alloc::string::String::from(METADATA_ADAPTER_KEY),
            value: crate::identity::digest_from_id(adapter_id),
        },
        choregraph_history::MetadataIdentity {
            key: alloc::string::String::from(METADATA_CONTROL_DOMAIN_KEY),
            value: crate::identity::digest_from_id(control_domain_id),
        },
        choregraph_history::MetadataIdentity {
            key: alloc::string::String::from(METADATA_CONTROL_EVENT_KEY),
            value: crate::identity::digest_from_id(control_event_id),
        },
    ]
}
