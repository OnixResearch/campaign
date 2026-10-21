//! Cargo manifests and lockfile must select the inventoried Choregraph Git revision.

const MOVING_KEYS: &[&str] = &["branch", "tag", "path"];
const OVERRIDE_TABLES: &[&str] = &["patch", "replace"];
const GIT_SOURCE_PREFIX: &str = "git+";
const REVISION_QUERY_PREFIX: &str = "rev=";

pub(super) fn validate_workspace(
    root: &toml::Table,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    reject_overrides(crate::policy::ROOT_MANIFEST, root)?;
    let pinned = workspace_choregraph_entries(root);
    if pinned.is_empty() {
        return Err(super::mismatch(
            "the root workspace declares no Choregraph history dependency",
        ));
    }
    for (name, spec) in pinned {
        validate_spec(name, spec, pin)?;
    }
    Ok(())
}

/// Root workspace dependencies whose key or package is in the Choregraph family.
fn workspace_choregraph_entries(root: &toml::Table) -> Vec<(&String, &toml::Value)> {
    root.get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
        .filter(|(name, spec)| {
            let package = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(name);
            super::is_choregraph(name) || super::is_choregraph(package)
        })
        .collect()
}

// r[impl campaign.choregraph_dependency]
pub(super) fn validate_member(
    path: &str,
    manifest: &toml::Table,
    root: &toml::Table,
) -> Result<(), crate::failure::CheckError> {
    reject_overrides(path, manifest)?;
    for declaration in crate::dependencies::declared(manifest, root)? {
        let is_history =
            super::is_choregraph(declaration.alias) || super::is_choregraph(declaration.package);
        if is_history && !declaration.is_inherited {
            return Err(super::moving(format!(
                "{path} declares {} directly instead of inheriting the pinned workspace dependency",
                declaration.package
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_lock(
    text: &crate::model::NamedText,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let lock: toml::Table = toml::from_str(&text.content).map_err(|error| {
        crate::failure::CheckError::failed(format!("invalid {}: {error}", text.relative_path))
    })?;
    let locked = locked_packages(&lock)?;
    if locked.is_empty() {
        return Err(super::mismatch(
            "Cargo.lock locks no Choregraph history package",
        ));
    }
    for (name, source) in locked {
        validate_lock_source(name, source, pin)?;
    }
    Ok(())
}

/// `(name, source)` for every locked Choregraph-family package.
fn locked_packages(
    lock: &toml::Table,
) -> Result<Vec<(&str, Option<&str>)>, crate::failure::CheckError> {
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| crate::failure::CheckError::failed("Cargo.lock has no package array"))?
        .iter()
        .filter_map(|package| {
            let name = package.get("name").and_then(toml::Value::as_str)?;
            let source = package.get("source").and_then(toml::Value::as_str);
            super::is_choregraph(name).then_some((name, source))
        })
        .collect();
    Ok(packages)
}

// r[impl campaign.choregraph_dependency.moving]
fn validate_spec(
    name: &str,
    spec: &toml::Value,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let table = spec.as_table().ok_or_else(|| {
        super::moving(format!(
            "{name} selects a registry version instead of a Git revision"
        ))
    })?;
    if let Some(key) = MOVING_KEYS.iter().find(|key| table.contains_key(**key)) {
        return Err(super::moving(format!("{name} selects Choregraph by {key}")));
    }
    let git = table.get("git").and_then(toml::Value::as_str);
    let revision = table.get("rev").and_then(toml::Value::as_str);
    let (Some(git), Some(revision)) = (git, revision) else {
        return Err(super::moving(format!(
            "{name} lacks an explicit git and rev pin"
        )));
    };
    if !crate::syntax::is_full_revision(revision) {
        return Err(super::moving(format!(
            "{name} rev is not a full revision: {revision}"
        )));
    }
    if git != pin.repository || revision != pin.revision {
        return Err(super::mismatch(format!(
            "{name} selects {git} at {revision}; the inventory pins {} at {}",
            pin.repository, pin.revision
        )));
    }
    assert_eq!(revision, pin.revision);
    assert!(MOVING_KEYS.iter().all(|key| !table.contains_key(*key)));
    Ok(())
}

fn validate_lock_source(
    name: &str,
    source: Option<&str>,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let Some(git_source) = source.and_then(|text| text.strip_prefix(GIT_SOURCE_PREFIX)) else {
        return Err(super::moving(format!(
            "Cargo.lock resolves {name} from a path or registry instead of the pinned Git revision"
        )));
    };
    let (locator, commit) = git_source.split_once('#').unwrap_or((git_source, ""));
    let (repository, query) = locator.split_once('?').unwrap_or((locator, ""));
    let Some(revision) = query.strip_prefix(REVISION_QUERY_PREFIX) else {
        return Err(super::moving(format!(
            "Cargo.lock resolves {name} without an explicit rev query: {query}"
        )));
    };
    if repository != pin.repository || revision != pin.revision || commit != pin.revision {
        return Err(super::mismatch(format!(
            "Cargo.lock resolves {name} to {git_source}; the inventory pins {} at {}",
            pin.repository, pin.revision
        )));
    }
    assert_eq!(commit, revision);
    assert!(crate::syntax::is_full_revision(commit));
    Ok(())
}

// r[impl campaign.choregraph_dependency.moving]
fn reject_overrides(path: &str, manifest: &toml::Table) -> Result<(), crate::failure::CheckError> {
    let is_overridden = |key: &str| super::is_choregraph(key.split(':').next().unwrap_or(key));
    for table_name in OVERRIDE_TABLES {
        let Some(table) = manifest.get(*table_name).and_then(toml::Value::as_table) else {
            continue;
        };
        let has_override = table.iter().any(|(key, value)| {
            is_overridden(key)
                || value
                    .as_table()
                    .is_some_and(|source| source.keys().any(|package| is_overridden(package)))
        });
        if has_override {
            return Err(super::moving(format!(
                "{path} overrides Choregraph through [{table_name}]"
            )));
        }
    }
    assert!(!OVERRIDE_TABLES.is_empty());
    assert!(!path.is_empty(), "override diagnostics name their manifest");
    Ok(())
}
