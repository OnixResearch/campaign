//! Every Cargo.lock package comes from the crates.io registry, a workspace member, or the pinned
//! Choregraph revision.
//!
//! Manifests name only direct dependencies; the lockfile records every package Cargo builds. A
//! sourceless package that is not a workspace member is an ambient path checkout, and any other
//! registry or Git source is an unpinned origin; both are `campaign.boundary.history_leak`.
//! Choregraph-family packages are checked first against the pin itself (`crate::pins`).

/// The crates.io index in its Git and sparse protocol spellings.
const REGISTRY_SOURCES: &[&str] = &[
    "registry+https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
];

/// One locked package as Cargo.lock records it.
#[derive(Clone, Copy)]
struct Locked<'lock> {
    name: &'lock str,
    source: Option<&'lock str>,
}

/// Refuse the first locked package whose source is not admitted.
// r[impl campaign.boundary.history_leak]
pub(crate) fn validate(
    lock: &crate::model::NamedText,
    members: &[(&str, &str)],
    pin: crate::pins::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    if members.is_empty() {
        return Err(crate::failure::CheckError::failed(format!(
            "{} cannot be checked: no workspace member manifest names its package",
            lock.relative_path
        )));
    }
    let table: toml::Table = toml::from_str(&lock.content).map_err(|error| {
        crate::failure::CheckError::failed(format!("invalid {}: {error}", lock.relative_path))
    })?;
    let packages = table
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            crate::failure::CheckError::failed(format!(
                "{} has no package array",
                lock.relative_path
            ))
        })?;
    let pinned = format!("git+{0}?rev={1}#{1}", pin.repository, pin.revision);
    for package in packages {
        let locked = Locked {
            name: package
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    crate::failure::CheckError::failed(format!(
                        "{} locks a package without a name",
                        lock.relative_path
                    ))
                })?,
            source: package.get("source").and_then(toml::Value::as_str),
        };
        let is_admitted = locked.source.map_or_else(
            || members.iter().any(|(_, member)| *member == locked.name),
            |source| REGISTRY_SOURCES.contains(&source) || source == pinned,
        );
        if !is_admitted {
            return Err(unadmitted(lock, locked));
        }
        debug_assert!(
            locked.source.is_some() || members.iter().any(|(_, member)| *member == locked.name)
        );
    }
    assert!(
        !members.is_empty(),
        "a lockfile is checked against the admitted workspace members"
    );
    Ok(())
}

fn unadmitted(lock: &crate::model::NamedText, locked: Locked<'_>) -> crate::failure::CheckError {
    let origin = locked.source.map_or_else(
        || "a path outside the workspace members".to_owned(),
        |source| format!("source {source}"),
    );
    crate::failure::CheckError::violation(
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        format!(
            "{} resolves {} from {origin}; only registry releases, workspace members, and the \
             pinned Choregraph revision are admitted",
            lock.relative_path, locked.name
        ),
    )
}
