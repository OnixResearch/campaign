//! Pure validation for explicit Campaign infrastructure facts.

mod configuration;
mod members;

/// Root manifest tables that make the workspace root an implicit package.
const ROOT_PACKAGE_TABLES: &[&str] = &["package", "project"];

// r[impl campaign.boundary.lexical_guard]
pub(crate) fn validate_boundaries(
    inputs: &crate::model::BoundaryInputs,
) -> Result<(), crate::failure::CheckError> {
    assert!(
        !inputs.root_manifest.content.is_empty(),
        "boundary validation requires the root manifest text"
    );
    assert!(
        !inputs.members.is_empty(),
        "boundary validation requires admitted workspace members"
    );

    require_contains(
        &inputs.root_manifest,
        &format!("repository = \"{}\"", crate::policy::CANONICAL_CAMPAIGN_URL),
    )?;
    require_contains(&inputs.flake_nix, "flake = false;")?;
    require_absent(&inputs.flake_nix, "skipCheck")?;
    require_absent(&inputs.flake_nix, "rustGateReady")?;
    require_contains(
        &inputs.repository_ncl,
        "selected_full_revision = Constants.choregraph_history_revision",
    )?;
    validate_manifest_text(inputs)?;
    validate_dylint_config(&inputs.dylint_config)?;
    validate_choregraph_history_source(inputs)?;
    crate::validation::configuration::validate_projection_copy(&inputs.profile_copy)?;

    let root = crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: inputs.root_manifest.relative_path,
        content: &inputs.root_manifest.content,
    })?;
    reject_root_package(&root)?;
    crate::dependencies::configuration::validate(&inputs.build_configuration)?;
    let manifests = members::parse_manifests(&inputs.members)?;
    let pin = crate::pins::validate(
        inputs,
        crate::pins::Manifests {
            root: &root,
            members: &manifests,
        },
    )?;
    crate::inventory::coverage::validate(&inputs.inventory, &inputs.cited_documents, &root)?;
    members::validate(
        inputs,
        members::Workspace {
            root: &root,
            manifests: &manifests,
            pin,
        },
    )?;
    crate::dependencies::features::allowlisted::validate_workspace(&root)?;
    require_inherited(
        &root,
        Inheritance {
            manifest: &inputs.core_manifest,
            package: "blake3",
        },
    )?;
    require_inherited(
        &root,
        Inheritance {
            manifest: &inputs.adapter_manifest,
            package: crate::policy::CORE_PACKAGE,
        },
    )?;
    crate::consumers::validate(crate::consumers::Repository {
        facts: &inputs.consumers,
        sources: &inputs.inventory.sources,
        members: &inputs.members,
    })
}

/// A root `[package]` (or legacy `[project]`) is an implicit workspace member outside every
/// policy row, so the workspace manifest must stay virtual.
// r[impl campaign.boundary.shared]
fn reject_root_package(root: &toml::Table) -> Result<(), crate::failure::CheckError> {
    let Some(table) = ROOT_PACKAGE_TABLES
        .iter()
        .find(|table| root.contains_key(**table))
    else {
        return Ok(());
    };
    Err(crate::failure::CheckError::violation(
        crate::rules::Rule::BOUNDARY_SHARED,
        format!(
            "{} declares a root [{table}], an implicit workspace member with no allowlist or \
             source profile",
            crate::policy::ROOT_MANIFEST
        ),
    ))
}

/// A manifest that must declare `package` as a normal dependency inherited from the workspace.
#[derive(Clone, Copy)]
struct Inheritance<'data> {
    manifest: &'data crate::model::NamedText,
    package: &'data str,
}

/// Parsed rather than textual, so a comment cannot satisfy the requirement.
// r[impl campaign.boundary.history_leak]
fn require_inherited(
    root: &toml::Table,
    required: Inheritance<'_>,
) -> Result<(), crate::failure::CheckError> {
    assert!(
        !required.package.is_empty(),
        "an inheritance names its package"
    );
    assert!(
        !required.manifest.relative_path.is_empty(),
        "an inheritance names its manifest"
    );
    let table = crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: required.manifest.relative_path,
        content: &required.manifest.content,
    })?;
    let is_inherited = crate::dependencies::declared(&table, root)?
        .iter()
        .any(|declaration| {
            declaration.package == required.package
                && declaration.is_inherited
                && !declaration.is_dev
        });
    if !is_inherited {
        return Err(crate::failure::CheckError::failed(format!(
            "{} must inherit {} from the workspace dependency entry",
            required.manifest.relative_path, required.package
        )));
    }
    Ok(())
}

pub(crate) fn validate_cairn_policy_source(
    inputs: &crate::model::CairnPolicyInputs,
) -> Result<(), crate::failure::CheckError> {
    validate_cairn_policy_source_manifest(&inputs.source_manifest)?;
    validate_vendored_comparisons(&inputs.compared_files)
}

fn validate_manifest_text(
    inputs: &crate::model::BoundaryInputs,
) -> Result<(), crate::failure::CheckError> {
    require_contains(&inputs.pre_commit_config, "id: octet-deny-all")?;
    require_contains(
        &inputs.pre_commit_config,
        &format!("rev: {}", crate::policy::OCTET_REVISION),
    )?;
    require_contains(
        &inputs.pre_commit_config,
        "args: [\"--workspace\", \"--\", \"--all-targets\", \"--all-features\"]",
    )
}

fn validate_dylint_config(
    config: &crate::model::NamedText,
) -> Result<(), crate::failure::CheckError> {
    require_contains(config, "[octet]")?;
    for token in crate::policy::FORBIDDEN_DYLINT_TOKENS {
        require_absent(config, token)?;
    }
    Ok(())
}

fn validate_choregraph_history_source(
    inputs: &crate::model::BoundaryInputs,
) -> Result<(), crate::failure::CheckError> {
    let Some(manifest) = &inputs.choregraph_history_manifest else {
        return Ok(());
    };
    require_contains(manifest, "name = \"choregraph-history\"")
}

pub(crate) fn validate_cairn_policy_source_manifest(
    manifest: &crate::model::NamedText,
) -> Result<(), crate::failure::CheckError> {
    require_contains(manifest, crate::policy::CAIRN_SOURCE_REFERENCE)?;
    require_contains(manifest, crate::policy::CAIRN_REVISION)?;
    require_contains(manifest, crate::policy::CAIRN_SOURCE_NAR_HASH)?;
    for file in crate::policy::VENDORED_CAIRN_POLICY_FILES {
        require_contains(manifest, file.path)?;
        require_contains(manifest, file.digest_blake3)?;
    }
    Ok(())
}

fn validate_vendored_comparisons(
    comparisons: &[crate::model::VendoredFileComparison],
) -> Result<(), crate::failure::CheckError> {
    assert!(
        comparisons.len() <= crate::policy::VENDORED_CAIRN_POLICY_FILES.len(),
        "the shell must not create extra vendored file comparisons"
    );
    assert!(
        comparisons
            .iter()
            .all(|comparison| !comparison.relative_path.is_empty()),
        "each vendored file comparison must retain its local path"
    );
    assert!(
        comparisons
            .iter()
            .all(|comparison| !comparison.source_relative_path.as_os_str().is_empty()),
        "each vendored file comparison must retain its source path"
    );

    for comparison in comparisons {
        if comparison.local_bytes != comparison.source_bytes {
            return Err(crate::failure::CheckError::failed(format!(
                "vendored Cairn policy file differs from pinned source: {}",
                comparison.relative_path
            )));
        }
    }
    Ok(())
}

fn require_contains(
    text: &crate::model::NamedText,
    expected: &str,
) -> Result<(), crate::failure::CheckError> {
    require_text(crate::model::TextRequirement {
        content: &text.content,
        relative_path: text.relative_path,
        expected,
    })
}

fn require_absent(
    text: &crate::model::NamedText,
    forbidden: &str,
) -> Result<(), crate::failure::CheckError> {
    let absence = crate::model::TextAbsence {
        relative_path: text.relative_path,
        forbidden,
    };
    if text.content.contains(absence.forbidden) {
        return Err(crate::failure::CheckError::failed(format!(
            "forbidden text remains in {}: {}",
            absence.relative_path, absence.forbidden
        )));
    }
    Ok(())
}

fn require_text(
    requirement: crate::model::TextRequirement<'_>,
) -> Result<(), crate::failure::CheckError> {
    if !requirement.content.contains(requirement.expected) {
        return Err(crate::failure::CheckError::failed(format!(
            "missing expected text in {}: {}",
            requirement.relative_path, requirement.expected
        )));
    }
    Ok(())
}

#[cfg(test)]
mod consumers;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod overlays;
#[cfg(test)]
mod scope;
#[cfg(test)]
mod tests;
