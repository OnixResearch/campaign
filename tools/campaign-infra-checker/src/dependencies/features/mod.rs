//! Dependency features a package enables beyond each dependency's defaults.
//!
//! A non-default feature can grant an allowlisted dependency ambient authority (blake3's `mmap`
//! reads files), so a `DefaultOnly` package keeps every normal and build dependency at its default
//! feature set, and every member declares a core-allowlisted dependency only by inheriting its
//! approved workspace entry (`allowlisted`).

pub(crate) mod allowlisted;

const FEATURES_KEY: &str = "features";
const FEATURES_TABLE: &str = "features";
const DEPENDENCY_FEATURE_SEPARATOR: char = '/';

/// Refuse non-default dependency features for a `DefaultOnly` package.
// r[impl campaign.functional_core.effect]
pub(super) fn validate(check: super::MemberCheck<'_>) -> Result<(), crate::failure::CheckError> {
    if check.package.features != crate::policy::scope::Features::DefaultOnly {
        return Ok(());
    }
    let declarations = super::declared(check.manifest, check.workspace)?;
    for declaration in &declarations {
        if declaration.is_dev {
            continue;
        }
        let listed = [declaration.entry, declaration.spec]
            .into_iter()
            .find_map(|entry| {
                entry
                    .get(FEATURES_KEY)
                    .filter(|features| !is_empty(features))
            });
        if let Some(features) = listed {
            return Err(refused(
                check,
                &format!(
                    "dependency {} with features = {features}",
                    declaration.package
                ),
            ));
        }
    }
    assert!(
        declarations
            .iter()
            .filter(|declaration| !declaration.is_dev)
            .all(|declaration| declaration.entry.get(FEATURES_KEY).is_none_or(is_empty)),
        "every admitted normal and build entry keeps its default features"
    );
    let Some(table) = check.manifest.get(FEATURES_TABLE) else {
        return Ok(());
    };
    let table = table.as_table().ok_or_else(|| {
        crate::failure::CheckError::failed(format!(
            "{} [{FEATURES_TABLE}] is not a table",
            check.manifest_path
        ))
    })?;
    assert!(
        check.package.features == crate::policy::scope::Features::DefaultOnly,
        "only a default-only package reaches the feature table"
    );
    for (name, enables) in table {
        let is_dependency_feature = enables.as_array().is_none_or(|list| {
            list.iter().any(|item| {
                item.as_str()
                    .is_none_or(|text| text.contains(DEPENDENCY_FEATURE_SEPARATOR))
            })
        });
        if is_dependency_feature {
            return Err(refused(
                check,
                &format!("[{FEATURES_TABLE}] {name} = {enables}"),
            ));
        }
    }
    Ok(())
}

/// An absent or empty feature list enables nothing beyond the defaults.
fn is_empty(features: &toml::Value) -> bool {
    features.as_array().is_some_and(Vec::is_empty)
}

fn refused(check: super::MemberCheck<'_>, enabling: &str) -> crate::failure::CheckError {
    assert!(!enabling.is_empty(), "a refusal names what it enables");
    crate::failure::CheckError::violation(
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        format!(
            "{} ({}) enables non-default dependency features through {enabling}; its dependencies \
             keep their default features",
            check.manifest_path, check.package.name
        ),
    )
}
