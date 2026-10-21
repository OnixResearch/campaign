//! Per-member checks: source scope, dependency allowlist, source declarations, and source patterns.

/// `(manifest path, parsed manifest)` for every member, in member order.
pub(super) fn parse_manifests(
    members: &[crate::model::Member],
) -> Result<Vec<(String, toml::Table)>, crate::failure::CheckError> {
    members
        .iter()
        .map(|member| {
            let path = format!("{}/{}", member.path, crate::policy::ROOT_MANIFEST);
            let table = crate::dependencies::parse_manifest(crate::model::ManifestText {
                relative_path: &path,
                content: &member.manifest,
            })?;
            Ok((path, table))
        })
        .collect()
}

/// Parsed workspace facts shared by every member check.
#[derive(Clone, Copy)]
pub(super) struct Workspace<'data> {
    pub(super) root: &'data toml::Table,
    /// `(manifest path, parsed manifest)` for every member, in member order.
    pub(super) manifests: &'data [(String, toml::Table)],
    pub(super) pin: crate::pins::Pin<'data>,
}

// r[impl campaign.boundary]
pub(super) fn validate(
    inputs: &crate::model::BoundaryInputs,
    workspace: Workspace<'_>,
) -> Result<(), crate::failure::CheckError> {
    assert_eq!(inputs.members.len(), workspace.manifests.len());
    let packages = member_packages(inputs, workspace.manifests);
    crate::dependencies::origin::validate_root(workspace.root)?;
    crate::dependencies::lock::validate(&inputs.cargo_lock, &packages, workspace.pin)?;
    for (member, (manifest_path, manifest)) in inputs.members.iter().zip(workspace.manifests) {
        for file in &member.files {
            crate::rustscope::validate(file)?;
        }
        let package = member_package(manifest_path, manifest)?;
        crate::dependencies::validate_member(crate::dependencies::MemberCheck {
            manifest_path,
            manifest,
            workspace: workspace.root,
            package,
        })?;
        crate::dependencies::origin::validate_member(
            crate::dependencies::origin::MemberManifest {
                member_path: &member.path,
                manifest_path,
                manifest,
            },
            crate::dependencies::origin::Workspace {
                root: workspace.root,
                members: &packages,
                pin: workspace.pin,
            },
        )?;
        crate::dependencies::features::allowlisted::validate_member(
            crate::dependencies::MemberCheck {
                manifest_path,
                manifest,
                workspace: workspace.root,
                package,
            },
        )?;
        let targets =
            crate::rustscope::targets::derive(crate::rustscope::targets::MemberManifest {
                member_path: &member.path,
                manifest_path,
                manifest,
            })?;
        if matches!(
            package.dependencies,
            crate::policy::scope::Allowlist::Exempt
        ) {
            continue;
        }
        validate_shared_member(inputs, member, &targets, package)?;
    }
    assert!(!workspace.manifests.is_empty());
    Ok(())
}

/// `(member path, package name)` for every member whose manifest names its package.
fn member_packages<'data>(
    inputs: &'data crate::model::BoundaryInputs,
    manifests: &'data [(String, toml::Table)],
) -> Vec<(&'data str, &'data str)> {
    inputs
        .members
        .iter()
        .zip(manifests)
        .filter_map(|(member, (_, manifest))| {
            crate::dependencies::package_name(manifest).map(|name| (member.path.as_str(), name))
        })
        .collect()
}

/// Declarations and source patterns for a Campaign crate outside the tooling exemption.
fn validate_shared_member(
    inputs: &crate::model::BoundaryInputs,
    member: &crate::model::Member,
    targets: &crate::rustscope::targets::Targets,
    package: &crate::policy::scope::Package,
) -> Result<(), crate::failure::CheckError> {
    for file in &member.files {
        crate::inventory::declarations::validate(&inputs.inventory.sources, file)?;
    }
    package
        .sources
        .as_ref()
        .map_or(Ok(()), |profile| validate_sources(member, targets, profile))
}

fn member_package(
    manifest_path: &str,
    manifest: &toml::Table,
) -> Result<&'static crate::policy::scope::Package, crate::failure::CheckError> {
    let name = crate::dependencies::package_name(manifest).ok_or_else(|| {
        crate::failure::CheckError::failed(format!("{manifest_path} lacks package.name"))
    })?;
    crate::policy::scope::for_package(name).ok_or_else(|| {
        crate::failure::CheckError::violation(
            crate::rules::Rule::BOUNDARY_SHARED,
            format!("workspace member {name} ({manifest_path}) has no allowlist or source profile"),
        )
    })
}

/// Scan every compiled source (the files under `src/` and the build script) with the profile.
fn validate_sources(
    member: &crate::model::Member,
    targets: &crate::rustscope::targets::Targets,
    profile: &crate::policy::scope::SourceProfile,
) -> Result<(), crate::failure::CheckError> {
    if profile.needs_no_std {
        let file = member
            .files
            .iter()
            .find(|file| file.display_path == targets.library);
        crate::patterns::require_no_std(file, &targets.library)?;
    }
    let sources: Vec<&crate::model::SourceFile> = member
        .files
        .iter()
        .filter(|file| targets.compiles(&file.display_path))
        .collect();
    if sources.is_empty() {
        return Err(crate::failure::CheckError::failed(format!(
            "no Rust sources under {}/{}",
            member.path,
            crate::policy::MEMBER_SOURCE_DIRECTORY
        )));
    }
    let test_modules = crate::patterns::gating::collect(&sources, targets.module_roots())?;
    assert!(
        sources
            .iter()
            .all(|file| targets.compiles(&file.display_path))
    );
    let gated_root = std::iter::once(targets.library.as_str())
        .chain(sources.iter().map(|file| file.display_path.as_str()))
        .find(|path| targets.is_root(path) && test_modules.contains(path));
    if let Some(root) = gated_root {
        return Err(crate::failure::CheckError::violation(
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
            format!(
                "{root} is a crate root that a #[cfg(test)] module declaration names; the \
                 identifier scan never exempts a crate root"
            ),
        ));
    }
    assert!(
        !test_modules.contains(&targets.library),
        "a library root is never its own test module"
    );
    for file in sources {
        let is_test_module = test_modules.contains(&file.display_path);
        crate::patterns::validate_file(file, profile, is_test_module)?;
    }
    Ok(())
}
