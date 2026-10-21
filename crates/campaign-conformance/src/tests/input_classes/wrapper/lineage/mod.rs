//! Holed load paths: the denial classes a journal shell serves over, and an advance that checks
//! lengths instead of the fast-forward prefix.

/// Denial classes the load matrix once never presented.
pub(super) fn is_unpresented_load_denial(error: &campaign_choregraph::AdapterError) -> bool {
    matches!(
        error,
        campaign_choregraph::AdapterError::PublicationFenceMismatch
            | campaign_choregraph::AdapterError::MetadataMismatch
            | campaign_choregraph::AdapterError::ParentLineageMismatch
            | campaign_choregraph::AdapterError::NonApplicationEnvelope
            | campaign_choregraph::AdapterError::SchemaIdentityMismatch
            | campaign_choregraph::AdapterError::ProjectionIdentityMismatch
            | campaign_choregraph::AdapterError::Campaign(_)
    )
}

/// Cold-start validation plus a length check in place of the fast-forward prefix check.
pub(super) fn length_monotone(
    request: &campaign_choregraph::ControlProjectionRequest<'_>,
    previous: Option<&campaign_choregraph::ValidatedControlProjection>,
) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
    if let Some(held) = previous {
        if request.branch.name.is_empty() {
            return Err(campaign_choregraph::AdapterError::ControlBranchMismatch);
        }
        let binding = campaign_choregraph::derive_projection_binding(
            request.graph,
            request.branch,
            request.expected_binding.projection_schema_id,
            request.expected_binding.adapter_id,
            request.expected_binding.control_domain_id,
        )?;
        if binding.control_branch_name_id != held.binding().control_branch_name_id {
            return Err(campaign_choregraph::AdapterError::ControlBranchMismatch);
        }
    }
    let cold = campaign_choregraph::validate_control_projection(request)?;
    if let Some(held) = previous {
        let (now, then) = (cold.control_events().len(), held.control_events().len());
        if now < then || (now == then && cold.binding().head_id != held.binding().head_id) {
            return Err(campaign_choregraph::AdapterError::NonFastForwardBranch);
        }
    }
    debug_assert_eq!(*cold.binding(), request.expected_binding);
    debug_assert!(
        previous.is_none_or(|held| cold.control_events().len() >= held.control_events().len())
    );
    Ok(cold)
}
