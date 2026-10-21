/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
pub fn write_ids(
    mut writer: crate::identity::FrameWriter,
    ids: &[crate::identity::Id],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    writer = writer.seq_len(crate::identity::usize_to_u64(ids.len())?);
    for id in ids {
        writer = writer.id(*id);
    }
    Ok(writer)
}
