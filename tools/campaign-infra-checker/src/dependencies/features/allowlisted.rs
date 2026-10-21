//! Core-allowlisted dependencies are declared once.
//!
//! Cargo unifies a package's features across every workspace member that builds it, so a feature
//! any member enables on blake3 (`mmap` reads files) reaches the core's blake3 too. The workspace
//! entry therefore carries the approved feature set (default features off, no features), and every
//! member declaration, exempt or not and in every dependency table, inherits it unchanged; no
//! `[features]` entry enables one of its features.

/// Keys that change a dependency's feature set, in both spellings Cargo reads.
const FEATURE_KEYS: &[&str] = &["features", "default-features", "default_features"];
/// Keys that turn default features off; the approved set requires exactly one of them, `false`.
const DEFAULT_FEATURE_KEYS: &[&str] = &["default-features", "default_features"];
const FEATURES_KEY: &str = "features";
const FEATURES_TABLE: &str = "features";
const DEPENDENCY_FEATURE_SEPARATOR: char = '/';
const WEAK_DEPENDENCY_MARKER: char = '?';

/// Every `[workspace.dependencies]` entry of a core-allowlisted package carries the approved
/// feature set: `default-features = false` and no features.
// r[impl campaign.functional_core.effect]
pub(crate) fn validate_workspace(root: &toml::Table) -> Result<(), crate::failure::CheckError> {
    let Some(entries) = root
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
    else {
        return Ok(());
    };
    let mut approved: usize = 0;
    for (alias, spec) in entries {
        let package = spec
            .get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(alias);
        if !is_core_dependency(package) {
            continue;
        }
        if !is_approved_entry(spec) {
            return Err(crate::failure::CheckError::violation(
                crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
                format!(
                    "{} [workspace.dependencies] {alias} declares core-allowlisted {package} with \
                     {spec}; its approved feature set is default-features = false and no features",
                    crate::policy::ROOT_MANIFEST
                ),
            ));
        }
        approved = approved.saturating_add(1);
    }
    assert!(
        approved <= entries.len(),
        "each entry is approved at most once"
    );
    assert!(
        entries.iter().all(|(alias, spec)| {
            !is_core_dependency(
                spec.get("package")
                    .and_then(toml::Value::as_str)
                    .unwrap_or(alias),
            ) || is_approved_entry(spec)
        }),
        "every core-allowlisted workspace entry carries the approved feature set"
    );
    Ok(())
}

/// Every declaration of a core-allowlisted package in one member inherits the workspace entry
/// without changing its features, and no `[features]` entry enables one of its features.
// r[impl campaign.functional_core.effect]
pub(crate) fn validate_member(
    check: crate::dependencies::MemberCheck<'_>,
) -> Result<(), crate::failure::CheckError> {
    let declarations = crate::dependencies::declared(check.manifest, check.workspace)?;
    let allowlisted: Vec<&crate::dependencies::Declared<'_>> = declarations
        .iter()
        .filter(|declaration| is_core_dependency(declaration.package))
        .collect();
    for declaration in &allowlisted {
        if !declaration.is_inherited {
            return Err(refused(
                check,
                &format!(
                    "declares {} without inheriting its [workspace.dependencies] entry",
                    declaration.package
                ),
            ));
        }
        let changed = FEATURE_KEYS
            .iter()
            .find(|key| declaration.entry.get(**key).is_some());
        if let Some(key) = changed {
            return Err(refused(
                check,
                &format!("sets {key} on {}", declaration.package),
            ));
        }
    }
    assert!(
        allowlisted
            .iter()
            .all(|declaration| declaration.is_inherited),
        "every admitted allowlisted declaration inherits the workspace entry"
    );
    assert!(allowlisted.len() <= declarations.len());
    match enabled_feature(check.manifest, &allowlisted) {
        Some(enabling) => Err(refused(check, &enabling)),
        None => Ok(()),
    }
}

/// The first `[features]` entry that enables a feature of an allowlisted declaration.
fn enabled_feature(
    manifest: &toml::Table,
    allowlisted: &[&crate::dependencies::Declared<'_>],
) -> Option<String> {
    let table = manifest.get(FEATURES_TABLE)?.as_table()?;
    table.iter().find_map(|(name, enables)| {
        let package = enables
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(toml::Value::as_str)
            .find_map(|item| dependency_of(item, allowlisted))?;
        Some(format!(
            "enables a feature of {package} through [{FEATURES_TABLE}] {name} = {enables}"
        ))
    })
}

fn is_core_dependency(package: &str) -> bool {
    crate::policy::scope::CORE_NORMAL_DEPENDENCIES.contains(&package)
}

/// `default-features = false` (one spelling) and an absent or empty feature list.
fn is_approved_entry(spec: &toml::Value) -> bool {
    let defaults: Vec<&toml::Value> = DEFAULT_FEATURE_KEYS
        .iter()
        .filter_map(|key| spec.get(*key))
        .collect();
    let is_default_off = matches!(defaults.as_slice(), [toml::Value::Boolean(false)]);
    let is_feature_free = spec
        .get(FEATURES_KEY)
        .is_none_or(|features| features.as_array().is_some_and(Vec::is_empty));
    is_default_off && is_feature_free
}

/// The allowlisted package whose feature `item` (`alias/feature` or `alias?/feature`) enables.
fn dependency_of<'data>(
    item: &str,
    allowlisted: &[&crate::dependencies::Declared<'data>],
) -> Option<&'data str> {
    let (dependency, _) = item.split_once(DEPENDENCY_FEATURE_SEPARATOR)?;
    let alias = dependency
        .strip_suffix(WEAK_DEPENDENCY_MARKER)
        .unwrap_or(dependency);
    allowlisted
        .iter()
        .find(|declaration| declaration.alias == alias)
        .map(|declaration| declaration.package)
}

fn refused(
    check: crate::dependencies::MemberCheck<'_>,
    change: &str,
) -> crate::failure::CheckError {
    assert!(!change.is_empty(), "a refusal names the change");
    crate::failure::CheckError::violation(
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        format!(
            "{} ({}) {change}; a core-allowlisted dependency is declared once, in \
             [workspace.dependencies] with its approved feature set",
            check.manifest_path, check.package.name
        ),
    )
}

#[cfg(test)]
mod tests;
