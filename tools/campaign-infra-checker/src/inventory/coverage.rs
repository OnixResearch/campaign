//! Inventory completeness against the repository: mapped paths, cited revisions, and Git dependencies.

/// A pinned Git workspace dependency.
struct GitPin<'data> {
    name: &'data str,
    repository: &'data str,
    revision: &'data str,
}

// r[impl campaign.source_inventory.complete]
pub(crate) fn validate(
    facts: &crate::model::InventoryFacts,
    cited_documents: &[crate::model::NamedText],
    root_manifest: &toml::Table,
) -> Result<(), crate::failure::CheckError> {
    if let Some(path) = facts.absent_campaign_paths.first() {
        return Err(super::violation(format!(
            "inventory maps absent Campaign path: {path}"
        )));
    }
    require_cited_revisions(&facts.sources, cited_documents)?;
    let pins = git_pins(root_manifest);
    require_pinned_entries(&facts.sources, &pins)?;
    require_entry_pins(&facts.sources, &pins)
}

/// Every full revision cited by README or a provenance record is an inventoried revision.
fn require_cited_revisions(
    sources: &super::Sources,
    cited_documents: &[crate::model::NamedText],
) -> Result<(), crate::failure::CheckError> {
    for document in cited_documents {
        if let Some(revision) = crate::syntax::full_revisions(&document.content)
            .into_iter()
            .find(|revision| !sources.records_revision(revision))
        {
            return Err(super::violation(format!(
                "{} cites revision {revision} that no inventory source records",
                document.relative_path
            )));
        }
    }
    Ok(())
}

/// Every pinned Git workspace dependency has a `dependency` entry at its pinned revision.
fn require_pinned_entries(
    sources: &super::Sources,
    pins: &[GitPin<'_>],
) -> Result<(), crate::failure::CheckError> {
    for pin in pins {
        let has_entry = sources.entries.iter().any(|entry| {
            entry.adoption == super::Adoption::Dependency
                && entry.repository == pin.repository
                && entry.revision == pin.revision
        });
        if !has_entry {
            return Err(super::violation(format!(
                "Git workspace dependency {} has no dependency inventory entry at its pinned revision",
                pin.name
            )));
        }
    }
    Ok(())
}

/// Every `dependency` entry names a pinned Git workspace dependency.
fn require_entry_pins(
    sources: &super::Sources,
    pins: &[GitPin<'_>],
) -> Result<(), crate::failure::CheckError> {
    let dependencies = sources
        .entries
        .iter()
        .filter(|entry| entry.adoption == super::Adoption::Dependency);
    for entry in dependencies {
        let has_pin = pins
            .iter()
            .any(|pin| entry.repository == pin.repository && entry.revision == pin.revision);
        if !has_pin {
            return Err(super::violation(format!(
                "dependency inventory entry {} matches no pinned Git workspace dependency",
                entry.id
            )));
        }
    }
    Ok(())
}

fn git_pins(root_manifest: &toml::Table) -> Vec<GitPin<'_>> {
    root_manifest
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
        .filter_map(|(name, spec)| {
            let table = spec.as_table()?;
            Some(GitPin {
                name,
                repository: table.get("git")?.as_str()?,
                revision: table
                    .get("rev")
                    .and_then(toml::Value::as_str)
                    .unwrap_or_default(),
            })
        })
        .collect()
}
