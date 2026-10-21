//! Dependency sources: registry releases, workspace members, and the pinned Choregraph revision.
//!
//! Any other source (a sibling worktree path, another Git repository, an alternate registry, or a
//! `[patch]`/`[replace]` override) lets a member depend on an ambient checkout such as Universal
//! Weave, so it is `campaign.boundary.history_leak`.

const GIT_KEY: &str = "git";
const REVISION_KEY: &str = "rev";
const PATH_KEY: &str = "path";
const REGISTRY_KEYS: &[&str] = &["registry", "registry-index"];
const OVERRIDE_TABLES: &[&str] = &["patch", "replace"];

/// The workspace facts every dependency source is checked against.
#[derive(Clone, Copy)]
pub(crate) struct Workspace<'data> {
    pub(crate) root: &'data toml::Table,
    /// `(member path, package name)` for every admitted workspace member.
    pub(crate) members: &'data [(&'data str, &'data str)],
    pub(crate) pin: crate::pins::Pin<'data>,
}

/// One member manifest whose direct dependency sources are checked.
#[derive(Clone, Copy)]
pub(crate) struct MemberManifest<'data> {
    /// The member directory relative to the repository root.
    pub(crate) member_path: &'data str,
    pub(crate) manifest_path: &'data str,
    pub(crate) manifest: &'data toml::Table,
}

/// One dependency entry and the directory its `path` key is relative to.
#[derive(Clone, Copy)]
struct Entry<'data> {
    declaration: super::Declared<'data>,
    table: &'data toml::Table,
    base: &'data str,
}

/// The root manifest overrides no package source.
// r[impl campaign.boundary.history_leak]
pub(crate) fn validate_root(root: &toml::Table) -> Result<(), crate::failure::CheckError> {
    reject_overrides(crate::policy::ROOT_MANIFEST, root)
}

/// Every direct dependency of one member comes from an admitted source.
// r[impl campaign.boundary.history_leak]
pub(crate) fn validate_member(
    member: MemberManifest<'_>,
    workspace: Workspace<'_>,
) -> Result<(), crate::failure::CheckError> {
    assert!(!member.manifest_path.is_empty());
    assert!(!workspace.pin.revision.is_empty());
    reject_overrides(member.manifest_path, member.manifest)?;
    for declaration in super::declared(member.manifest, workspace.root)? {
        let Some(table) = declaration.spec.as_table() else {
            continue;
        };
        let base = if declaration.is_inherited {
            ""
        } else {
            member.member_path
        };
        let entry = Entry {
            declaration,
            table,
            base,
        };
        if let Some(source) = ambient_source(entry, workspace) {
            return Err(ambient(member, declaration, &source));
        }
    }
    Ok(())
}

/// The first source key of `entry` that names neither the registry, an admitted member, nor the
/// pinned Choregraph revision.
fn ambient_source(entry: Entry<'_>, workspace: Workspace<'_>) -> Option<String> {
    git_source(entry, workspace.pin)
        .or_else(|| path_source(entry, workspace.members))
        .or_else(|| {
            REGISTRY_KEYS.iter().find_map(|key| {
                let value = entry.table.get(*key)?;
                Some(format!("{key} = {value}"))
            })
        })
}

/// A Git source other than a Choregraph-family package at the inventoried repository and
/// revision.
fn git_source(entry: Entry<'_>, pin: crate::pins::Pin<'_>) -> Option<String> {
    let git = entry.table.get(GIT_KEY)?;
    let is_history = super::names::in_family(super::names::FamilyMatch {
        dependency: entry.declaration.package,
        family: crate::policy::CHOREGRAPH_PACKAGE_FAMILY,
    });
    let revision = entry.table.get(REVISION_KEY).and_then(toml::Value::as_str);
    let is_pinned =
        is_history && git.as_str() == Some(pin.repository) && revision == Some(pin.revision);
    (!is_pinned).then(|| format!("{GIT_KEY} = {git}"))
}

/// A path other than the directory of the admitted member that provides the declared package.
fn path_source(entry: Entry<'_>, members: &[(&str, &str)]) -> Option<String> {
    let path = entry.table.get(PATH_KEY)?;
    let directory = path.as_str().and_then(|relative| {
        crate::syntax::resolve(crate::syntax::RelativePath {
            base: entry.base,
            path: relative,
        })
    });
    let is_member = members.iter().any(|(member, package)| {
        *package == entry.declaration.package
            && crate::syntax::resolve(crate::syntax::RelativePath {
                base: "",
                path: member,
            })
            .is_some_and(|member_directory| Some(member_directory) == directory)
    });
    (!is_member).then(|| format!("{PATH_KEY} = {path}"))
}

fn ambient(
    member: MemberManifest<'_>,
    declaration: super::Declared<'_>,
    source: &str,
) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        format!(
            "{} takes dependency {} from {source}; only registry releases, workspace members, and \
             the pinned Choregraph revision are admitted",
            member.manifest_path, declaration.package
        ),
    )
}

fn reject_overrides(path: &str, manifest: &toml::Table) -> Result<(), crate::failure::CheckError> {
    let Some(table) = OVERRIDE_TABLES
        .iter()
        .find(|table| manifest.contains_key(**table))
    else {
        return Ok(());
    };
    Err(crate::failure::CheckError::violation(
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        format!("{path} overrides dependency sources through [{table}]"),
    ))
}

#[cfg(test)]
mod tests;
