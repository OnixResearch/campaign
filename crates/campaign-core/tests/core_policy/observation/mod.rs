pub(crate) fn result_for(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
    child_label: &'static [u8],
    payload_label: &'static [u8],
) -> Result<campaign_core::ResultObservation, String> {
    let child = (campaign_core::ChildObservation {
        reservation_id: campaign_core::Id::absent(),
        child_moment_id: crate::suite::admission::xid(
            campaign_core::IdKind::ChildMoment,
            child_label,
        ),
        discovery_id: campaign_core::Id::absent(),
        event_payload_id: crate::suite::admission::xid(
            campaign_core::IdKind::EventPayload,
            payload_label,
        ),
    }
    .derive(state.admission.campaign_id, selection.selection_id))
    .map_err(|error| format!("{error:?}"))?;
    assert_ne!(child.reservation_id, campaign_core::Id::absent());
    assert_eq!(child.discovery_id.kind, campaign_core::IdKind::Discovery);
    Ok(campaign_core::ResultObservation {
        result_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        parent_moment_id: selection.parent_moment_id,
        operation_id: selection.operation_id,
        adapter_id: state.admission.adapter_id,
        payload_id: crate::suite::s3_fixture::result_payload(selection),
        children: vec![child],
        findings: Vec::new(),
        derived_records: Vec::new(),
    })
}
