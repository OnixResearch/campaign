//! Filesystem shell for explicit repository validation inputs.

mod configuration;
mod scan;

const MEMBER_MANIFEST: &str = "Cargo.toml";

pub(crate) fn run_request(
    request: &crate::model::CheckRequest,
) -> Result<(), crate::failure::CheckError> {
    if request.command.includes_boundaries() {
        crate::validation::validate_boundaries(&load_boundary_inputs(request)?)?;
    }
    if request.command.includes_cairn_policy_source() {
        crate::validation::validate_cairn_policy_source(&load_cairn_policy_inputs(request)?)?;
    }
    Ok(())
}

pub(crate) fn load_boundary_inputs(
    request: &crate::model::CheckRequest,
) -> Result<crate::model::BoundaryInputs, crate::failure::CheckError> {
    let root = &request.root;
    let root_manifest = read_text(root, crate::policy::ROOT_MANIFEST)?;
    let members = load_members(root, &root_manifest.content)?;
    let inputs = crate::model::BoundaryInputs {
        cargo_lock: read_text(root, crate::policy::CARGO_LOCK)?,
        flake_nix: read_text(root, crate::policy::FLAKE_NIX)?,
        flake_lock: read_text(root, crate::policy::FLAKE_LOCK)?,
        repository_ncl: read_text(root, crate::policy::REPOSITORY_NCL)?,
        generated_repository_json: read_text(root, crate::policy::GENERATED_REPOSITORY_JSON)?,
        profile_copy: load_profile_copy(root)?,
        core_manifest: read_text(root, crate::policy::CORE_MANIFEST)?,
        adapter_manifest: read_text(root, crate::policy::ADAPTER_MANIFEST)?,
        pre_commit_config: read_text(root, crate::policy::PRE_COMMIT_CONFIG)?,
        dylint_config: read_text(root, crate::policy::DYLINT_CONFIG)?,
        publication_record: read_text(root, crate::policy::CHOREGRAPH_PUBLICATION_RECORD)?,
        cited_documents: crate::policy::CITED_SOURCE_DOCUMENTS
            .iter()
            .map(|path| read_text(root, path))
            .collect::<Result<Vec<_>, _>>()?,
        choregraph_history_manifest: load_optional_choregraph_manifest(request)?,
        build_configuration: configuration::discover(root)?,
        inventory: load_inventory(root)?,
        consumers: load_consumers(root)?,
        root_manifest,
        members,
    };
    assert_eq!(
        inputs.root_manifest.relative_path,
        crate::policy::ROOT_MANIFEST
    );
    assert_eq!(
        inputs.cited_documents.len(),
        crate::policy::CITED_SOURCE_DOCUMENTS.len()
    );
    Ok(inputs)
}

pub(crate) fn load_cairn_policy_inputs(
    request: &crate::model::CheckRequest,
) -> Result<crate::model::CairnPolicyInputs, crate::failure::CheckError> {
    let source_manifest = read_text(&request.root, crate::policy::CAIRN_POLICY_MANIFEST)?;
    let compared_files = match &request.cairn_source_root {
        Some(source_root) => load_vendored_comparisons(&request.root, source_root)?,
        None => Vec::new(),
    };
    Ok(crate::model::CairnPolicyInputs {
        source_manifest,
        compared_files,
    })
}

/// Parse the exported inventory and record which of its Campaign paths are absent.
fn load_inventory(
    root: &std::path::Path,
) -> Result<crate::model::InventoryFacts, crate::failure::CheckError> {
    let text = read_text(root, crate::policy::SOURCE_INVENTORY_JSON)?;
    let sources = crate::inventory::json::parse(&text)?;
    let absent_campaign_paths = sources
        .entries
        .iter()
        .flat_map(|entry| entry.campaign_paths.iter())
        .filter(|path| std::fs::symlink_metadata(root.join(path)).is_err())
        .cloned()
        .collect();
    Ok(crate::model::InventoryFacts {
        sources,
        absent_campaign_paths,
    })
}

/// Admit the exported consumer pilot record and record which in-repo pilot paths are absent.
fn load_consumers(
    root: &std::path::Path,
) -> Result<crate::model::ConsumerFacts, crate::failure::CheckError> {
    let text = read_text(root, crate::policy::CONSUMER_RECORD_JSON)?;
    let pilots = crate::consumers::json::parse(&text)?;
    let absent_campaign_paths = pilots
        .campaign_paths()
        .filter(|path| std::fs::symlink_metadata(root.join(path)).is_err())
        .cloned()
        .collect();
    Ok(crate::model::ConsumerFacts {
        pilots,
        absent_campaign_paths,
    })
}

/// Every declared workspace member with its manifest and all Rust files below it.
fn load_members(
    root: &std::path::Path,
    root_manifest: &str,
) -> Result<Vec<crate::model::Member>, crate::failure::CheckError> {
    let paths = crate::rustscope::members::parse(root_manifest)?;
    let mut members = Vec::with_capacity(paths.len());
    for path in paths {
        let manifest = std::fs::read_to_string(root.join(&path).join(MEMBER_MANIFEST))
            .map_err(|error| crate::failure::CheckError::io(&error))?;
        let files = scan::sources(root, &path)?;
        members.push(crate::model::Member {
            path,
            manifest,
            files,
        });
    }
    assert!(!members.is_empty());
    Ok(members)
}

fn load_optional_choregraph_manifest(
    request: &crate::model::CheckRequest,
) -> Result<Option<crate::model::NamedText>, crate::failure::CheckError> {
    let Some(source_root) = &request.choregraph_history_source else {
        return Ok(None);
    };
    read_text(source_root, crate::policy::CHOREGRAPH_HISTORY_MANIFEST).map(Some)
}

fn load_vendored_comparisons(
    root: &std::path::Path,
    source_root: &std::path::Path,
) -> Result<Vec<crate::model::VendoredFileComparison>, crate::failure::CheckError> {
    let mut comparisons = Vec::with_capacity(crate::policy::VENDORED_CAIRN_POLICY_FILES.len());
    for file in crate::policy::VENDORED_CAIRN_POLICY_FILES {
        let source_relative_path = strip_cairn_policy_prefix(file.path)?;
        comparisons.push(crate::model::VendoredFileComparison {
            relative_path: file.path,
            local_bytes: std::fs::read(root.join(file.path))
                .map_err(|error| crate::failure::CheckError::io(&error))?,
            source_bytes: std::fs::read(source_root.join(&source_relative_path))
                .map_err(|error| crate::failure::CheckError::io(&error))?,
            source_relative_path,
        });
    }
    assert_eq!(
        comparisons.len(),
        crate::policy::VENDORED_CAIRN_POLICY_FILES.len()
    );
    assert!(
        comparisons
            .iter()
            .zip(crate::policy::VENDORED_CAIRN_POLICY_FILES)
            .all(|(comparison, source)| comparison.relative_path == source.path)
    );
    Ok(comparisons)
}

/// Read the profile crate's embedded default and the generated default-profile projection it must equal.
fn load_profile_copy(
    root: &std::path::Path,
) -> Result<crate::model::ProjectionCopy, crate::failure::CheckError> {
    Ok(crate::model::ProjectionCopy {
        copy_path: crate::policy::PROFILE_CRATE_DEFAULT_JSON,
        projection_path: crate::policy::GENERATED_DEFAULT_PROFILE_JSON,
        copy_bytes: read_bytes(root, crate::policy::PROFILE_CRATE_DEFAULT_JSON)?,
        projection_bytes: read_bytes(root, crate::policy::GENERATED_DEFAULT_PROFILE_JSON)?,
    })
}

fn read_bytes(
    root: &std::path::Path,
    relative_path: &'static str,
) -> Result<Vec<u8>, crate::failure::CheckError> {
    std::fs::read(root.join(relative_path)).map_err(|error| {
        crate::failure::CheckError::failed(format!("cannot read {relative_path}: {error}"))
    })
}

fn read_text(
    root: &std::path::Path,
    relative_path: &'static str,
) -> Result<crate::model::NamedText, crate::failure::CheckError> {
    let content = std::fs::read_to_string(root.join(relative_path)).map_err(|error| {
        crate::failure::CheckError::failed(format!("cannot read {relative_path}: {error}"))
    })?;
    if content.is_empty() {
        return Err(crate::failure::CheckError::failed(format!(
            "required file is empty: {relative_path}"
        )));
    }
    Ok(crate::model::NamedText {
        relative_path,
        content,
    })
}

fn strip_cairn_policy_prefix(path: &str) -> Result<std::path::PathBuf, crate::failure::CheckError> {
    std::path::Path::new(path)
        .strip_prefix("cairn-policy")
        .map(std::path::Path::to_path_buf)
        .map_err(|error| {
            crate::failure::CheckError::failed(format!("invalid Cairn policy path: {error}"))
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn loads_member_sources_with_root_relative_paths() -> Result<(), crate::failure::CheckError> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/positive/repository-root");
        let manifest = std::fs::read_to_string(root.join(crate::policy::ROOT_MANIFEST))
            .map_err(|error| crate::failure::CheckError::io(&error))?;
        let members = super::load_members(&root, &manifest)?;
        let core = members
            .iter()
            .find(|member| member.path == "crates/campaign-core")
            .ok_or_else(|| crate::failure::CheckError::failed("fixture lacks the core member"))?;
        assert!(
            core.files
                .iter()
                .any(|file| file.display_path == "crates/campaign-core/src/lib.rs")
        );
        assert!(
            core.files.windows(2).all(
                |pair| matches!(pair, [left, right] if left.display_path < right.display_path)
            )
        );
        Ok(())
    }
}
