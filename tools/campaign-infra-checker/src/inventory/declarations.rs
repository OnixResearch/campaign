//! `// campaign-source: <id>` declarations must name an inventory entry that maps the declaring file.

// r[impl campaign.source_inventory.missing]
pub(crate) fn validate(
    sources: &super::Sources,
    file: &crate::model::SourceFile,
) -> Result<(), crate::failure::CheckError> {
    let declarations = file
        .content
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(crate::policy::SOURCE_DECLARATION_MARKER));
    for (index, line) in declarations {
        let location = format!("{}:{}", file.display_path, index.saturating_add(1));
        let id = declared_id(line).ok_or_else(|| {
            missing(format!(
                "{location}: source declarations must read `{}<id>`",
                crate::policy::SOURCE_DECLARATION_PREFIX
            ))
        })?;
        let entry = sources.entry(id).ok_or_else(|| {
            missing(format!(
                "{location}: {id} names no provenance inventory source"
            ))
        })?;
        if !is_mapped(entry, file) {
            return Err(missing(format!(
                "{location}: inventory source {id} does not record this Campaign path"
            )));
        }
        assert_eq!(entry.id, id, "declarations resolve to the named entry");
        assert!(crate::syntax::is_source_id(id));
    }
    Ok(())
}

fn is_mapped(entry: &super::Entry, file: &crate::model::SourceFile) -> bool {
    entry.campaign_paths.iter().any(|ancestor| {
        crate::syntax::is_contained(crate::syntax::Containment {
            path: &file.display_path,
            ancestor,
        })
    })
}

fn declared_id(line: &str) -> Option<&str> {
    let id = line
        .trim_start()
        .strip_prefix(crate::policy::SOURCE_DECLARATION_PREFIX)?
        .trim_end();
    crate::syntax::is_source_id(id).then_some(id)
}

fn missing(message: String) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(crate::rules::Rule::SOURCE_INVENTORY_MISSING, message)
}
