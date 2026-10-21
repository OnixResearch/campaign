//! Bounded source discovery rejects observed symlinks and preserves path order.

struct Progress<'root> {
    root: &'root std::path::Path,
    pending: std::collections::VecDeque<std::path::PathBuf>,
    files: Vec<crate::model::SourceFile>,
    visited: usize,
}

/// Rust files below `root/relative_root`, displayed relative to `root`.
pub(super) fn sources(
    root: &std::path::Path,
    relative_root: &str,
) -> Result<Vec<crate::model::SourceFile>, crate::failure::CheckError> {
    let source_root = root.join(relative_root);
    let metadata = inspect(&source_root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(crate::failure::CheckError::failed(format!(
            "source root is not a real directory: {}",
            source_root.display()
        )));
    }
    let mut scan = Progress {
        root,
        pending: std::collections::VecDeque::from([source_root]),
        files: Vec::new(),
        visited: 0,
    };
    while let Some(directory) = scan.pending.pop_front() {
        for child in children(&directory)? {
            scan.visit(&child)?;
        }
    }
    scan.files
        .sort_by(|left, right| left.display_path.cmp(&right.display_path));
    assert!(scan.visited <= crate::policy::MAX_SCAN_ENTRIES);
    assert!(scan.files.len() <= scan.visited);
    Ok(scan.files)
}

impl Progress<'_> {
    fn visit(&mut self, path: &std::path::Path) -> Result<(), crate::failure::CheckError> {
        if self.visited >= crate::policy::MAX_SCAN_ENTRIES {
            return Err(crate::failure::CheckError::failed(
                "source scan entry bound exceeded",
            ));
        }
        self.visited = self
            .visited
            .saturating_add(crate::policy::VISITED_ENTRY_STEP);
        let metadata = inspect(path)?;
        if metadata.file_type().is_symlink() {
            return Err(crate::failure::CheckError::failed(format!(
                "source scan rejects symlink: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            self.pending.push_back(path.to_path_buf());
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == crate::policy::RS_EXTENSION)
        {
            self.files.push(read_source(self.root, path)?);
        }
        assert!(self.files.len() <= self.visited);
        assert!(self.pending.len() <= self.visited);
        Ok(())
    }
}

fn children(
    directory: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, crate::failure::CheckError> {
    let entries =
        std::fs::read_dir(directory).map_err(|error| crate::failure::CheckError::io(&error))?;
    let mut paths = Vec::new();
    for entry in entries {
        if paths.len() >= crate::policy::MAX_DIRECTORY_ENTRIES {
            return Err(crate::failure::CheckError::failed(
                "directory entry bound exceeded",
            ));
        }
        paths.push(
            entry
                .map_err(|error| crate::failure::CheckError::io(&error))?
                .path(),
        );
    }
    paths.sort();
    assert!(paths.len() <= crate::policy::MAX_DIRECTORY_ENTRIES);
    assert!(paths.iter().all(|path| path.parent() == Some(directory)));
    Ok(paths)
}

fn inspect(path: &std::path::Path) -> Result<std::fs::Metadata, crate::failure::CheckError> {
    std::fs::symlink_metadata(path).map_err(|error| crate::failure::CheckError::io(&error))
}

fn read_source(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<crate::model::SourceFile, crate::failure::CheckError> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|error| crate::failure::CheckError::io(&error))?;
    let mut content = String::new();
    file.take(crate::policy::SOURCE_READ_LIMIT)
        .read_to_string(&mut content)
        .map_err(|error| crate::failure::CheckError::io(&error))?;
    let size_bytes = u64::try_from(content.len())
        .map_err(|_| crate::failure::CheckError::failed("source length is not representable"))?;
    if size_bytes > crate::policy::MAX_SOURCE_BYTES {
        return Err(crate::failure::CheckError::failed(
            "Rust source byte bound exceeded",
        ));
    }
    assert!(size_bytes <= crate::policy::MAX_SOURCE_BYTES);
    assert!(
        !path.as_os_str().is_empty(),
        "a successful open must name a file"
    );
    let relative = path.strip_prefix(root).map_err(|_| {
        crate::failure::CheckError::failed(format!(
            "source escaped the repository root: {}",
            path.display()
        ))
    })?;
    Ok(crate::model::SourceFile {
        display_path: relative.display().to_string(),
        content,
    })
}

#[cfg(test)]
mod tests;
