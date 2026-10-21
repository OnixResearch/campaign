//! Repository fixture harness: one positive root and negative overlays that each break one rule.
//!
//! A negative fixture directory holds only the files it replaces or adds; the harness copies the
//! positive root into a temporary directory and applies the overlay before validation.

const FIXTURE_DIRECTORY: &str = "fixtures";
const POSITIVE_ROOT: &str = "positive/repository-root";
pub(super) const NEGATIVE_ROOT: &str = "negative";
pub(super) const MISSING_CAIRN_POLICY_FILE: &str = "missing-cairn-policy-file";

pub(super) fn check_boundaries(root: &std::path::Path) -> Result<(), crate::failure::CheckError> {
    let inputs = crate::fsio::load_boundary_inputs(&request(root))?;
    crate::validation::validate_boundaries(&inputs)
}

pub(super) fn request(root: &std::path::Path) -> crate::model::CheckRequest {
    crate::model::CheckRequest {
        command: crate::model::CheckCommand::Boundaries,
        root: root.to_path_buf(),
        cairn_source_root: None,
        choregraph_history_source: None,
    }
}

pub(super) fn materialize(
    negative: Option<&str>,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    copy_tree(&fixture_path(POSITIVE_ROOT), root.path())?;
    if let Some(fixture) = negative {
        copy_tree(&fixture_path(NEGATIVE_ROOT).join(fixture), root.path())?;
    }
    Ok(root)
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    let mut pending = vec![(from.to_path_buf(), to.to_path_buf())];
    while let Some((source, destination)) = pending.pop() {
        std::fs::create_dir_all(&destination)?;
        for entry in std::fs::read_dir(&source)? {
            let entry = entry?;
            let target = destination.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                pending.push((entry.path(), target));
            } else {
                std::fs::copy(entry.path(), target)?;
            }
        }
    }
    Ok(())
}

pub(super) fn fixture_path(relative_path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_DIRECTORY)
        .join(relative_path)
}
