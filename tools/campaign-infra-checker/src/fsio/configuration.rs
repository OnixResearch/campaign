//! Repository-wide discovery of build configuration: Cargo reads `.cargo/config.toml` and
//! `.cargo/config` in the directory it runs in and every ancestor, and rustup reads
//! `rust-toolchain.toml` and `rust-toolchain` the same way, so every directory below the root is
//! visited. Symlinked directories are not descended (a run inside one resolves outside the
//! repository), but a symlinked `.cargo` directory or configuration file is found. The checker's
//! own fixture roots are skipped.

/// Every Cargo configuration and toolchain file below `root`, in path order.
pub(super) fn discover(
    root: &std::path::Path,
) -> Result<crate::model::BuildConfiguration, crate::failure::CheckError> {
    let mut found = crate::model::BuildConfiguration {
        cargo_configurations: Vec::new(),
        toolchain_files: Vec::new(),
    };
    let mut pending = std::collections::VecDeque::from([root.to_path_buf()]);
    let mut visited: usize = 0;
    while let Some(directory) = pending.pop_front() {
        visited = visited.saturating_add(1);
        if visited > crate::policy::MAX_REPOSITORY_DIRECTORIES {
            return Err(crate::failure::CheckError::failed(
                "repository directory bound exceeded while looking for build configuration",
            ));
        }
        let relative = relative_directory(root, &directory)?;
        inspect(root, &relative, &mut found)?;
        pending.extend(subdirectories(root, &directory)?);
    }
    assert!(visited <= crate::policy::MAX_REPOSITORY_DIRECTORIES);
    assert!(
        found
            .cargo_configurations
            .iter()
            .chain(found.toolchain_files.iter().map(|file| &file.relative_path))
            .all(|path| !is_fixture(path)),
        "the checker's fixture roots are never reported"
    );
    Ok(found)
}

/// Record the configuration files of one directory (`relative` to the root, empty at the root).
fn inspect(
    root: &std::path::Path,
    relative: &str,
    found: &mut crate::model::BuildConfiguration,
) -> Result<(), crate::failure::CheckError> {
    let at = |file: &str| {
        if relative.is_empty() {
            file.to_owned()
        } else {
            format!("{relative}/{file}")
        }
    };
    let before = found.cargo_configurations.len();
    found
        .cargo_configurations
        .reserve(crate::policy::CARGO_CONFIGURATION_FILES.len());
    found
        .toolchain_files
        .reserve(crate::policy::TOOLCHAIN_FILES.len());
    for file in crate::policy::CARGO_CONFIGURATION_FILES {
        let path = at(file);
        if std::fs::symlink_metadata(root.join(&path)).is_ok() {
            found.cargo_configurations.push(path);
        }
    }
    assert!(
        found.cargo_configurations.len()
            <= before.saturating_add(crate::policy::CARGO_CONFIGURATION_FILES.len())
    );
    for file in crate::policy::TOOLCHAIN_FILES {
        let path = at(file);
        if std::fs::symlink_metadata(root.join(&path)).is_err() {
            continue;
        }
        let content = std::fs::read_to_string(root.join(&path)).map_err(|error| {
            crate::failure::CheckError::failed(format!("cannot read {path}: {error}"))
        })?;
        found.toolchain_files.push(crate::model::ConfigurationFile {
            relative_path: path,
            content,
        });
    }
    assert!(
        found
            .toolchain_files
            .iter()
            .all(|file| !is_fixture(&file.relative_path))
    );
    Ok(())
}

/// The real (non-symlink) subdirectories of `directory`, in name order, without fixture roots.
fn subdirectories(
    root: &std::path::Path,
    directory: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, crate::failure::CheckError> {
    let entries =
        std::fs::read_dir(directory).map_err(|error| crate::failure::CheckError::io(&error))?;
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| crate::failure::CheckError::io(&error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| crate::failure::CheckError::io(&error))?;
        let path = entry.path();
        if children.len() >= crate::policy::MAX_REPOSITORY_DIRECTORIES {
            return Err(crate::failure::CheckError::failed(
                "repository directory bound exceeded while looking for build configuration",
            ));
        }
        if file_type.is_dir() && !is_fixture(&relative_directory(root, &path)?) {
            children.push(path);
        }
    }
    children.sort();
    assert!(children.len() <= crate::policy::MAX_REPOSITORY_DIRECTORIES);
    assert!(
        children
            .iter()
            .all(|child| child.parent() == Some(directory))
    );
    Ok(children)
}

fn relative_directory(
    root: &std::path::Path,
    directory: &std::path::Path,
) -> Result<String, crate::failure::CheckError> {
    let relative = directory.strip_prefix(root).map_err(|_| {
        crate::failure::CheckError::failed(format!(
            "configuration walk escaped the repository root: {}",
            directory.display()
        ))
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn is_fixture(path: &str) -> bool {
    crate::syntax::is_contained(crate::syntax::Containment {
        path,
        ancestor: crate::policy::CHECKER_FIXTURE_ROOT,
    })
}

#[cfg(test)]
mod tests;
