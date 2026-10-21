//! One published Choregraph revision across the inventory, Cargo, Nix, configuration, and records.
//!
//! Disagreement is `campaign.choregraph_dependency.valid`; a branch, tag, path, patch, replacement,
//! or reference-only selection is `campaign.choregraph_dependency.moving`.

mod cargo;
mod nix;

const CONFIGURATION_FIELDS: &[&str] = &["git_url", "history_revision", "selected_full_revision"];

/// The pinned repository location and full revision recorded by the inventory.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pin<'data> {
    pub(crate) repository: &'data str,
    pub(crate) revision: &'data str,
}

/// Parsed Cargo manifests needed for dependency pin checks.
#[derive(Clone, Copy)]
pub(crate) struct Manifests<'data> {
    pub(crate) root: &'data toml::Table,
    pub(crate) members: &'data [(String, toml::Table)],
}

/// Validate every pin and return the inventoried one, the only admitted Git dependency source.
// r[impl campaign.choregraph_dependency]
pub(crate) fn validate<'data>(
    inputs: &'data crate::model::BoundaryInputs,
    manifests: Manifests<'_>,
) -> Result<Pin<'data>, crate::failure::CheckError> {
    let pin = inventory_pin(&inputs.inventory.sources)?;
    cargo::validate_workspace(manifests.root, pin)?;
    for (path, manifest) in manifests.members {
        cargo::validate_member(path, manifest, manifests.root)?;
    }
    cargo::validate_lock(&inputs.cargo_lock, pin)?;
    nix::validate_flake(&inputs.flake_nix, pin)?;
    nix::validate_flake_lock(&inputs.flake_lock, pin)?;
    validate_configuration(&inputs.generated_repository_json, pin)?;
    validate_record(&inputs.publication_record, pin)?;
    Ok(pin)
}

fn inventory_pin(
    sources: &crate::inventory::Sources,
) -> Result<Pin<'_>, crate::failure::CheckError> {
    let entry = sources
        .entry(crate::policy::CHOREGRAPH_SOURCE_ID)
        .filter(|entry| entry.adoption == crate::inventory::Adoption::Dependency)
        .ok_or_else(|| {
            mismatch(format!(
                "the inventory lacks the {} dependency entry",
                crate::policy::CHOREGRAPH_SOURCE_ID
            ))
        })?;
    Ok(Pin {
        repository: &entry.repository,
        revision: &entry.revision,
    })
}

/// The generated repository configuration names the same URL and revision.
fn validate_configuration(
    text: &crate::model::NamedText,
    pin: Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let value: serde_json::Value = serde_json::from_str(&text.content).map_err(|error| {
        crate::failure::CheckError::failed(format!(
            "{} is not valid JSON: {error}",
            text.relative_path
        ))
    })?;
    let choregraph = value.get("choregraph");
    let expected = [pin.repository, pin.revision, pin.revision];
    assert_eq!(CONFIGURATION_FIELDS.len(), expected.len());
    assert!(crate::syntax::is_full_revision(pin.revision));
    for (name, wanted) in CONFIGURATION_FIELDS.iter().zip(expected) {
        let found = choregraph
            .and_then(|table| table.get(*name))
            .and_then(serde_json::Value::as_str);
        if found != Some(wanted) {
            return Err(mismatch(format!(
                "{} choregraph.{name} must equal {wanted}",
                text.relative_path
            )));
        }
    }
    Ok(())
}

/// The publication record names the pinned URL and revision.
// r[impl campaign.choregraph_dependency.blocked]
fn validate_record(
    record: &crate::model::NamedText,
    pin: Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    for expected in [pin.repository, pin.revision] {
        if !record.content.contains(expected) {
            return Err(mismatch(format!(
                "{} does not record {expected}",
                record.relative_path
            )));
        }
    }
    Ok(())
}

fn mismatch(message: impl AsRef<str>) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_VALID, message)
}

fn moving(message: impl AsRef<str>) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING, message)
}

fn is_choregraph(package: &str) -> bool {
    crate::dependencies::names::in_family(crate::dependencies::names::FamilyMatch {
        dependency: package,
        family: crate::policy::CHOREGRAPH_PACKAGE_FAMILY,
    })
}

#[cfg(test)]
mod tests;
