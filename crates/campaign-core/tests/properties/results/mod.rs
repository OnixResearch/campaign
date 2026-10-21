//! Exact settlement inputs for outstanding selections.

/// Exact one-child result for an outstanding selection, labelled by the selection identity.
pub(crate) fn result_for(
    state: &campaign_core::FrontierState,
    outstanding: &campaign_core::OutstandingSelection,
) -> Result<campaign_core::ResultObservation, String> {
    let selection_id = outstanding.selection_id;
    let child = crate::suite::fixture::observed(
        campaign_core::ChildObservation {
            reservation_id: campaign_core::Id::absent(),
            child_moment_id: crate::suite::fixture::bound_id(
                campaign_core::IdKind::ChildMoment,
                b"child",
                selection_id,
            ),
            discovery_id: campaign_core::Id::absent(),
            event_payload_id: crate::suite::fixture::bound_id(
                campaign_core::IdKind::EventPayload,
                b"child-payload",
                selection_id,
            ),
        }
        .derive(state.admission.campaign_id, selection_id),
    )?;
    assert_eq!(child.discovery_id.kind, campaign_core::IdKind::Discovery);
    assert_ne!(child.reservation_id, campaign_core::Id::absent());
    Ok(campaign_core::ResultObservation {
        result_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id,
        parent_moment_id: outstanding.parent_moment_id,
        operation_id: outstanding.operation_id,
        adapter_id: state.admission.adapter_id,
        payload_id: crate::suite::fixture::bound_id(
            campaign_core::IdKind::EventPayload,
            b"result-payload",
            selection_id,
        ),
        children: vec![child],
        findings: Vec::new(),
        derived_records: Vec::new(),
    })
}
