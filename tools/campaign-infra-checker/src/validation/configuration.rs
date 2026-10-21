//! `campaign.configuration.stale`: a committed copy of generated configuration equals its projection byte for byte.

// r[impl campaign.configuration.stale]
pub(super) fn validate_projection_copy(
    copy: &crate::model::ProjectionCopy,
) -> Result<(), crate::failure::CheckError> {
    assert!(
        !copy.copy_path.is_empty(),
        "a projection copy must name its copy"
    );
    assert!(
        !copy.projection_path.is_empty(),
        "a projection copy must name its projection"
    );
    assert_ne!(
        copy.copy_path, copy.projection_path,
        "a projection copy must not compare a file with itself"
    );
    if copy.copy_bytes != copy.projection_bytes {
        return Err(crate::failure::CheckError::violation(
            crate::rules::Rule::CONFIGURATION_STALE,
            format!(
                "stale configuration copy: {} differs from {}",
                copy.copy_path, copy.projection_path
            ),
        ));
    }
    Ok(())
}
