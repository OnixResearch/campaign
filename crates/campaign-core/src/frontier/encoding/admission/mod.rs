//! Admission framing for the v3 frontier-state encoding.

pub(super) fn write(
    writer: crate::identity::FrameWriter,
    admission: &crate::model::CampaignAdmission,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let writer = write_profiles(writer, admission);
    let writer = write_bounds(writer, &admission.bounds)?;
    let writer = crate::model::write_resources(writer, &admission.resource_budgets)?;
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(admission.roots.len())?);
    for root in &admission.roots {
        writer = writer
            .id(root.moment_id)
            .id(root.projection_id)
            .id(root.adapter_id);
    }
    crate::model::write_ids(writer, &admission.operation_schema_ids)
}

fn write_profiles(
    writer: crate::identity::FrameWriter,
    admission: &crate::model::CampaignAdmission,
) -> crate::identity::FrameWriter {
    let writer = write_projection(
        writer
            .id(admission.campaign_id)
            .u32(admission.schema_version),
        &admission.projection,
    );
    writer
        .id(admission.policy_id)
        .id(admission.guidance_profile_id)
        .id(admission.entropy_profile_id)
        .id(admission.budget_profile_id)
        .id(admission.adapter_id)
        .id(admission.control_domain_id)
}

fn write_bounds(
    writer: crate::identity::FrameWriter,
    bounds: &crate::model::ExplorationBounds,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let values = bounds.values();
    let mut writer = writer
        .id(bounds.bounds_id)
        .seq_len(crate::identity::usize_to_u64(values.len())?);
    for value in values {
        writer = writer.u64(value);
    }
    Ok(writer)
}

fn write_projection(
    writer: crate::identity::FrameWriter,
    projection: &crate::model::ProjectionBinding,
) -> crate::identity::FrameWriter {
    writer
        .id(projection.projection_id)
        .id(projection.graph_id)
        .id(projection.control_branch_id)
        .u64(projection.generation)
        .id(projection.head_id)
        .id(projection.projection_schema_id)
        .id(projection.adapter_id)
        .id(projection.control_domain_id)
}
