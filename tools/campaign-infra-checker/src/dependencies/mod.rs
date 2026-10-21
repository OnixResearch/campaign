//! Dependency boundary checks include aliases, inherited entries, target tables, the legacy
//! underscore table spellings Cargo still reads, and dependency features.

pub(crate) mod configuration;
pub(crate) mod features;
pub(crate) mod lock;
pub(crate) mod names;
pub(crate) mod origin;

/// Normal and build tables, including the underscore spelling Cargo accepts before edition 2024.
const NORMAL_TABLES: &[&str] = &["dependencies", "build-dependencies", "build_dependencies"];
const DEV_TABLES: &[&str] = &["dev-dependencies", "dev_dependencies"];
const TABLE_KINDS: usize = NORMAL_TABLES.len().saturating_add(DEV_TABLES.len());

/// One direct dependency declaration after workspace inheritance.
#[derive(Clone, Copy)]
pub(crate) struct Declared<'data> {
    /// The dependency key, which may rename the package.
    pub(crate) alias: &'data str,
    pub(crate) package: &'data str,
    /// The member's own entry, which may add features to an inherited one.
    pub(crate) entry: &'data toml::Value,
    /// The effective entry after inheritance; it names the package source.
    pub(crate) spec: &'data toml::Value,
    pub(crate) is_inherited: bool,
    pub(crate) is_dev: bool,
}

/// One workspace member manifest checked against its package row.
#[derive(Clone, Copy)]
pub(crate) struct MemberCheck<'data> {
    pub(crate) manifest_path: &'data str,
    pub(crate) manifest: &'data toml::Table,
    pub(crate) workspace: &'data toml::Table,
    pub(crate) package: &'static crate::policy::scope::Package,
}

pub(crate) fn parse_manifest(
    text: crate::model::ManifestText<'_>,
) -> Result<toml::Table, crate::failure::CheckError> {
    toml::from_str(text.content).map_err(|error| {
        crate::failure::CheckError::failed(format!(
            "invalid Cargo manifest TOML in {}: {error}",
            text.relative_path
        ))
    })
}

pub(crate) fn package_name(manifest: &toml::Table) -> Option<&str> {
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
}

/// Every direct dependency in normal, build, dev, and target-specific tables.
pub(crate) fn declared<'data>(
    manifest: &'data toml::Table,
    workspace: &'data toml::Table,
) -> Result<Vec<Declared<'data>>, crate::failure::CheckError> {
    collect_tables(manifest)?
        .into_iter()
        .flat_map(|(table, is_dev)| table.iter().map(move |entry| (entry, is_dev)))
        .map(|((alias, spec), is_dev)| {
            let resolved = names::resolve(names::Dependency {
                alias,
                spec,
                workspace,
            })?;
            Ok(Declared {
                alias,
                package: resolved.package,
                entry: spec,
                spec: resolved.spec,
                is_inherited: resolved.is_inherited,
                is_dev,
            })
        })
        .collect()
}

/// Apply effect families, consumer families, the package allowlist, and the feature policy to one
/// member; an exempt member keeps only the effect and history families.
pub(crate) fn validate_member(check: MemberCheck<'_>) -> Result<(), crate::failure::CheckError> {
    features::validate(check)?;
    for declaration in declared(check.manifest, check.workspace)? {
        let rule = if let crate::policy::scope::Allowlist::Allowed { normal, dev_only } =
            check.package.dependencies
        {
            let is_allowed = normal.contains(&declaration.package)
                || (declaration.is_dev && dev_only.contains(&declaration.package));
            family_rule(declaration)
                .or_else(|| (!is_allowed).then(|| unlisted_rule(declaration.package)))
        } else {
            exempt_rule(declaration)
        };
        if let Some(rule) = rule {
            return Err(rejected(check, rule, declaration.package));
        }
    }
    Ok(())
}

/// Tooling may take any dependency except ambient randomness and Choregraph-owned history.
// r[impl campaign.boundary.history_leak]
// r[impl campaign.functional_core.effect]
fn exempt_rule(declaration: Declared<'_>) -> Option<crate::rules::Rule> {
    if belongs(
        declaration,
        crate::policy::scope::EFFECT_DEPENDENCY_FAMILIES,
    ) {
        return Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT);
    }
    let is_history = belongs(
        declaration,
        crate::policy::scope::HISTORY_DEPENDENCY_FAMILIES,
    ) || belongs(declaration, &[crate::policy::CHOREGRAPH_PACKAGE_FAMILY]);
    is_history.then_some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK)
}

// r[impl campaign.boundary.consumer_leak]
fn family_rule(declaration: Declared<'_>) -> Option<crate::rules::Rule> {
    if belongs(
        declaration,
        crate::policy::scope::EFFECT_DEPENDENCY_FAMILIES,
    ) {
        return Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT);
    }
    if belongs(
        declaration,
        crate::policy::scope::HISTORY_DEPENDENCY_FAMILIES,
    ) {
        return Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK);
    }
    belongs(
        declaration,
        crate::policy::scope::CONSUMER_DEPENDENCY_FAMILIES,
    )
    .then_some(crate::rules::Rule::BOUNDARY_CONSUMER_LEAK)
}

/// Whether the dependency key or its package belongs to one of `families`.
fn belongs(declaration: Declared<'_>, families: &[&str]) -> bool {
    [declaration.alias, declaration.package]
        .iter()
        .any(|dependency| {
            families
                .iter()
                .any(|family| names::in_family(names::FamilyMatch { dependency, family }))
        })
}

// r[impl campaign.boundary.history_leak]
fn unlisted_rule(package: &str) -> crate::rules::Rule {
    let is_history = names::in_family(names::FamilyMatch {
        dependency: package,
        family: crate::policy::CHOREGRAPH_PACKAGE_FAMILY,
    });
    if is_history {
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK
    } else {
        crate::rules::Rule::BOUNDARY_SHARED
    }
}

fn rejected(
    check: MemberCheck<'_>,
    rule: crate::rules::Rule,
    package: &str,
) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(
        rule,
        format!(
            "{} ({}) declares dependency {package} refused by its dependency policy",
            check.manifest_path, check.package.name
        ),
    )
}

fn collect_tables(
    manifest: &toml::Table,
) -> Result<Vec<(&toml::Table, bool)>, crate::failure::CheckError> {
    let targets = table_if_present(manifest, "target")?;
    let target_count = targets.map_or(0, toml::Table::len);
    let mut tables = Vec::with_capacity(TABLE_KINDS.saturating_mul(target_count.saturating_add(1)));
    tables.extend(default_tables(manifest)?);
    for target_value in targets.into_iter().flat_map(toml::Table::values) {
        let target = target_value.as_table().ok_or_else(|| {
            crate::failure::CheckError::failed("Cargo target entry is not a table")
        })?;
        tables.extend(default_tables(target)?);
    }
    Ok(tables)
}

fn default_tables(
    manifest: &toml::Table,
) -> Result<Vec<(&toml::Table, bool)>, crate::failure::CheckError> {
    let mut tables = Vec::with_capacity(TABLE_KINDS);
    for table_name in NORMAL_TABLES {
        if let Some(table) = table_if_present(manifest, table_name)? {
            tables.push((table, false));
        }
    }
    for table_name in DEV_TABLES {
        if let Some(table) = table_if_present(manifest, table_name)? {
            tables.push((table, true));
        }
    }
    assert!(tables.len() <= TABLE_KINDS);
    Ok(tables)
}

fn table_if_present<'manifest>(
    manifest: &'manifest toml::Table,
    table_name: &str,
) -> Result<Option<&'manifest toml::Table>, crate::failure::CheckError> {
    let Some(table_value) = manifest.get(table_name) else {
        return Ok(None);
    };
    let Some(table) = table_value.as_table() else {
        return Err(crate::failure::CheckError::failed(format!(
            "Cargo table {table_name} is not a table"
        )));
    };
    Ok(Some(table))
}

#[cfg(test)]
mod tests;
