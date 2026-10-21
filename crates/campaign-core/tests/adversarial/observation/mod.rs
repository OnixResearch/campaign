pub(crate) fn result(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
    findings: Vec<campaign_core::FindingRef>,
) -> campaign_core::ResultObservation {
    campaign_core::ResultObservation {
        result_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        parent_moment_id: selection.parent_moment_id,
        operation_id: selection.operation_id,
        adapter_id: state.admission.adapter_id,
        payload_id: crate::suite::admission::tid(
            campaign_core::IdKind::EventPayload,
            b"adversarial-result-payload",
        ),
        children: Vec::new(),
        findings,
        derived_records: Vec::new(),
    }
}
