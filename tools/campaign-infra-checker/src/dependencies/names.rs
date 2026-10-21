//! Resolve the effective package name without losing Cargo workspace aliases.

#[derive(Clone, Copy)]
pub(super) struct Dependency<'data> {
    pub(super) alias: &'data str,
    pub(super) spec: &'data toml::Value,
    pub(super) workspace: &'data toml::Table,
}

/// The package a declaration selects, its effective entry, and whether it inherits the workspace
/// entry.
#[derive(Clone, Copy)]
pub(super) struct Resolved<'data> {
    pub(super) package: &'data str,
    /// The member entry, or the workspace entry it inherits, which names the package source.
    pub(super) spec: &'data toml::Value,
    pub(super) is_inherited: bool,
}

/// A dependency name and the package family it may belong to.
#[derive(Clone, Copy)]
pub(crate) struct FamilyMatch<'name> {
    pub(crate) dependency: &'name str,
    pub(crate) family: &'name str,
}

/// Whether the dependency is the family package or a `family-*` package, with `-` and `_`
/// equivalent as Cargo treats them (`rand_core` belongs to `rand`).
// r[impl campaign.boundary.history_leak]
pub(crate) fn in_family(names: FamilyMatch<'_>) -> bool {
    let mut dependency = names.dependency.bytes().map(normalized);
    let is_prefixed = names
        .family
        .bytes()
        .map(normalized)
        .all(|expected| dependency.next() == Some(expected));
    is_prefixed && dependency.next().is_none_or(|separator| separator == b'-')
}

/// `_` and `-` name the same Cargo package.
const fn normalized(byte: u8) -> u8 {
    if byte == b'_' { b'-' } else { byte }
}

pub(super) fn resolve(
    dependency: Dependency<'_>,
) -> Result<Resolved<'_>, crate::failure::CheckError> {
    let inheritance = dependency
        .spec
        .as_table()
        .and_then(|table| table.get("workspace"));
    let Some(inheritance) = inheritance else {
        let package = direct(dependency)?;
        return Ok(Resolved {
            package,
            spec: dependency.spec,
            is_inherited: false,
        });
    };
    if inheritance.as_bool() != Some(true) {
        return Err(crate::failure::CheckError::failed(
            "workspace dependency flag must be true",
        ));
    }
    let spec = inherited(dependency)?;
    let package = direct(Dependency { spec, ..dependency })?;
    assert!(!package.is_empty(), "a resolved package keeps its name");
    assert!(
        spec.as_table()
            .is_none_or(|table| !table.contains_key("workspace")),
        "an inherited entry cannot inherit again"
    );
    Ok(Resolved {
        package,
        spec,
        is_inherited: true,
    })
}

fn inherited(dependency: Dependency<'_>) -> Result<&toml::Value, crate::failure::CheckError> {
    dependency
        .workspace
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .and_then(|dependencies| dependencies.get(dependency.alias))
        .ok_or_else(|| {
            crate::failure::CheckError::failed("inherited dependency has no workspace entry")
        })
}

fn direct(dependency: Dependency<'_>) -> Result<&str, crate::failure::CheckError> {
    if dependency.alias.is_empty() {
        return Err(empty_package());
    }
    if dependency.spec.is_str() {
        return Ok(dependency.alias);
    }
    let table = dependency.spec.as_table().ok_or_else(|| {
        crate::failure::CheckError::failed("dependency is neither a version string nor a table")
    })?;
    if table.contains_key("workspace") {
        return Err(crate::failure::CheckError::failed(
            "workspace dependency entry cannot inherit again",
        ));
    }
    let package = table
        .get("package")
        .map_or(Ok(dependency.alias), |package| {
            package.as_str().ok_or_else(|| {
                crate::failure::CheckError::failed("dependency package is not a string")
            })
        })?;
    if package.is_empty() {
        return Err(empty_package());
    }
    assert!(!table.contains_key("workspace"));
    assert_eq!(
        table
            .get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(dependency.alias),
        package
    );
    Ok(package)
}

fn empty_package() -> crate::failure::CheckError {
    crate::failure::CheckError::failed("dependency names an empty package")
}
