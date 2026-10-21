mod admission;
mod candidates;
mod observations;
pub(super) const LABEL_FRONTIER_STATE: &[u8] = b"frontier-state-v4";
pub(super) fn derive_state_id(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<crate::identity::Id> {
    crate::identity::derive_id_with(
        crate::identity::IdKind::State,
        crate::frontier::encoding::LABEL_FRONTIER_STATE,
        |writer| {
            let writer = crate::frontier::encoding::admission::write(writer, &state.admission)?;
            let writer = crate::model::write_ids(writer, &state.event_ids)?;
            let writer = crate::frontier::encoding::candidates::write(writer, &state.candidates)?;
            let writer = crate::frontier::encoding::observations::write_outstanding(
                writer,
                &state.outstanding,
            )?;
            let writer = crate::model::write_ids(writer, &state.accepted_results)?;
            let writer = crate::frontier::encoding::observations::write_discoveries(
                writer,
                &state.discoveries,
            )?;
            let writer = crate::frontier::encoding::observations::write_known_moments(
                writer,
                &state.known_moments,
            )?;
            let writer =
                crate::frontier::encoding::observations::write_settled(writer, &state.settled)?;
            let writer = crate::model::write_ids(writer, &state.reservations)?;
            let writer = crate::frontier::encoding::observations::write_payload_bindings(
                writer,
                &state.payload_bindings,
            )?;
            let writer =
                crate::frontier::encoding::observations::write_findings(writer, &state.findings)?;
            let writer = crate::frontier::encoding::observations::write_stopped(
                writer,
                state.stopped.as_ref(),
            )?;
            let writer = crate::model::write_resources(writer, &state.reserved_resources)?;
            let writer = writer.u64(state.selection_count);
            let writer = writer.u64(state.guidance_record_count);
            Ok(writer.u64(state.control_event_count))
        },
    )
}
