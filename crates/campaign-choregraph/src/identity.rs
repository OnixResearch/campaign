//! Identity conversion and projection binding for the adapter.

/// Frame label of the stable control-branch name identity.
const CONTROL_BRANCH_NAME_LABEL: &[u8] = b"campaign-choregraph-control-branch-name";

/// Converts a Campaign identity into a Choregraph digest with the same bytes.
#[must_use]
pub const fn digest_from_id(id: campaign_core::Id) -> choregraph_history::Digest {
    choregraph_history::Digest::from_bytes(id.digest)
}

/// Converts a Choregraph digest into a Campaign identity of the requested kind.
#[must_use]
pub const fn id_from_digest(
    kind: campaign_core::IdKind,
    digest: choregraph_history::Digest,
) -> campaign_core::Id {
    campaign_core::Id {
        kind,
        digest: digest.bytes(),
    }
}

/// Converts a Choregraph event identity into a Campaign head identity.
#[must_use]
pub const fn head_id_from_event(event: choregraph_history::EventId) -> campaign_core::Id {
    id_from_digest(campaign_core::IdKind::Head, event.digest())
}

/// Converts a Choregraph event identity into a Campaign event identity.
#[must_use]
pub const fn event_id_from_event(event: choregraph_history::EventId) -> campaign_core::Id {
    id_from_digest(campaign_core::IdKind::Event, event.digest())
}

/// Converts one observed Choregraph branch value into a Campaign publication-fence fact.
#[must_use]
pub fn branch_fact(branch: &choregraph_history::BranchRef) -> campaign_core::BranchFact {
    campaign_core::BranchFact {
        control_branch_id: id_from_digest(campaign_core::IdKind::ControlBranch, branch.identity()),
        head_id: head_id_from_event(branch.head),
        generation: branch.generation,
    }
}

/// Derives the Campaign projection binding for the supplied graph and branch.
///
/// The branch name must be one Choregraph can advance on `graph` (non-empty and within the
/// graph's `max_label_bytes`), so a campaign is never bound to, or loaded from, a control branch
/// that can never record another event.
///
/// # Errors
///
/// Returns an error if one identity has the wrong Campaign kind, and
/// `History(InvalidBranchName)` for a branch name Choregraph refuses on `graph`.
///
/// # Panics
///
/// Panics if identity derivation changes the supplied graph or branch-head bytes.
pub fn derive_projection_binding(
    graph: &choregraph_history::HistoryGraph,
    branch: &choregraph_history::BranchRef,
    projection_schema_id: campaign_core::Id,
    adapter_id: campaign_core::Id,
    control_domain_id: campaign_core::Id,
) -> crate::contracts::AdapterResult<campaign_core::ProjectionBinding> {
    projection_schema_id.require_kind(campaign_core::IdKind::Projection)?;
    adapter_id.require_kind(campaign_core::IdKind::Adapter)?;
    control_domain_id.require_kind(campaign_core::IdKind::ControlDomain)?;
    check_branch_name(graph, branch)?;
    let binding = campaign_core::ProjectionBinding {
        projection_id: campaign_core::Id::absent(),
        graph_id: id_from_digest(campaign_core::IdKind::Graph, graph.identity()),
        control_branch_id: id_from_digest(campaign_core::IdKind::ControlBranch, branch.identity()),
        control_branch_name_id: control_branch_name_id(&branch.name)?,
        generation: branch.generation,
        head_id: head_id_from_event(branch.head),
        projection_schema_id,
        adapter_id,
        control_domain_id,
    }
    .derive()?;
    assert_eq!(binding.graph_id.digest, graph.identity().bytes());
    assert_eq!(binding.head_id, head_id_from_event(branch.head));
    Ok(binding)
}

/// Requires `branch`'s name to be non-empty and within `graph`'s `max_label_bytes`, the name
/// Choregraph requires of every branch advance.
// r[impl campaign.history_projection.stale]
fn check_branch_name(
    graph: &choregraph_history::HistoryGraph,
    branch: &choregraph_history::BranchRef,
) -> crate::contracts::AdapterResult<()> {
    let is_within_limit =
        u32::try_from(branch.name.len()).is_ok_and(|bytes| bytes <= graph.limits().max_label_bytes);
    if branch.name.is_empty() || !is_within_limit {
        return Err(choregraph_history::HistoryError::InvalidBranchName.into());
    }
    Ok(())
}

/// Derives the stable identity of one control-branch name, independent of its head and generation.
///
/// # Errors
///
/// Returns an error if identity framing rejects the name length.
fn control_branch_name_id(name: &str) -> campaign_core::CampaignResult<campaign_core::Id> {
    campaign_core::identity::derive_id_with(
        campaign_core::IdKind::ControlBranch,
        CONTROL_BRANCH_NAME_LABEL,
        |writer| Ok(writer.bytes(name.as_bytes())),
    )
}

/// Derives the non-circular Campaign payload identity for one control event.
///
/// # Errors
///
/// Returns an error if the control event contains a malformed length.
pub fn derive_control_payload_id(
    event: &campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::Id> {
    crate::envelope::derive_control_payload_id(event)
}
