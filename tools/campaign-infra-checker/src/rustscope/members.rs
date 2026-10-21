//! Admission of explicit member paths for complete source scanning.

const MAX_MEMBERS: usize = 32;

pub(crate) fn parse(manifest: &str) -> Result<Vec<String>, crate::failure::CheckError> {
    let table: toml::Table = toml::from_str(manifest)
        .map_err(|error| crate::failure::CheckError::failed(error.to_string()))?;
    let members = table
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            crate::failure::CheckError::failed("explicit workspace members are required")
        })?;
    if members.is_empty() || members.len() > MAX_MEMBERS {
        return Err(crate::failure::CheckError::failed(
            "workspace member count is out of bounds",
        ));
    }
    let mut paths = members.iter().map(admit).collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    paths.dedup();
    if paths.len() != members.len() {
        return Err(crate::failure::CheckError::failed(
            "duplicate workspace member",
        ));
    }
    assert!(paths.len() <= MAX_MEMBERS);
    assert_eq!(paths.len(), members.len());
    Ok(paths)
}

fn admit(value: &toml::Value) -> Result<String, crate::failure::CheckError> {
    let path = value
        .as_str()
        .ok_or_else(|| crate::failure::CheckError::failed("workspace member is not a string"))?;
    if path.is_empty() || path.contains(['*', '?', '[', ']']) {
        return Err(crate::failure::CheckError::failed(
            "workspace member must be an explicit path",
        ));
    }
    if !std::path::Path::new(path)
        .components()
        .all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(crate::failure::CheckError::failed(
            "workspace member must remain inside the repository",
        ));
    }
    Ok(path.to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn admits_explicit_paths() -> Result<(), crate::failure::CheckError> {
        assert_eq!(
            super::parse("[workspace]\nmembers = ['crates/b', 'crates/a']")?,
            ["crates/a", "crates/b"]
        );
        Ok(())
    }

    #[test]
    fn rejects_missing_duplicate_escaping_and_implicit_members() {
        let cases = [
            "[workspace]",
            "[workspace]\nmembers = []",
            "[workspace]\nmembers = ['']",
            "[workspace]\nmembers = ['../outside']",
            "[workspace]\nmembers = ['/outside']",
            "[workspace]\nmembers = ['crates/*']",
            "[workspace]\nmembers = ['a', 'a']",
            "[workspace]\nmembers = [true]",
        ];
        for case in cases {
            assert!(
                super::parse(case).is_err(),
                "accepted invalid members: {case}"
            );
        }
    }
}
