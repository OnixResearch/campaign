//! No repository build configuration may redirect what Cargo builds or which compiler builds it.
//!
//! Any `.cargo/config.toml` or `.cargo/config` below the root (outside the checker's own fixture
//! roots) is refused whatever it sets: its `paths`, `[patch]`, and `[source]` tables build against
//! an ambient worktree while Cargo.lock stays unchanged; its `rustflags` pass `--extern` of an
//! ambient library; its `[build] rustc`, `rustc-wrapper`, and `rustc-workspace-wrapper` replace the
//! compiler invocation; its `include` pulls in another file. A toolchain file's `[toolchain] path`
//! selects a compiler from a local directory. Each is `campaign.boundary.history_leak`.
//! Configuration outside the repository (ancestors of the root, `CARGO_HOME`, `--config`, the
//! environment) is a documented residual.

const TOOLCHAIN_TABLE: &str = "toolchain";
const TOOLCHAIN_PATH_KEY: &str = "path";
/// The TOML toolchain file; the legacy `rust-toolchain` may instead hold a bare channel name.
const TOML_TOOLCHAIN_SUFFIX: &str = ".toml";

/// Refuse every repository Cargo configuration file and every toolchain `path`.
// r[impl campaign.boundary.history_leak]
pub(crate) fn validate(
    configuration: &crate::model::BuildConfiguration,
) -> Result<(), crate::failure::CheckError> {
    if let Some(path) = configuration.cargo_configurations.first() {
        return Err(crate::failure::CheckError::violation(
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
            format!(
                "{path} is repository Cargo configuration, which may redirect dependency sources, \
                 compiler flags, or the compiler; none is admitted"
            ),
        ));
    }
    for file in &configuration.toolchain_files {
        validate_toolchain(file)?;
    }
    Ok(())
}

fn validate_toolchain(
    file: &crate::model::ConfigurationFile,
) -> Result<(), crate::failure::CheckError> {
    let table = match toml::from_str::<toml::Table>(&file.content) {
        Ok(table) => table,
        Err(_) if !file.relative_path.ends_with(TOML_TOOLCHAIN_SUFFIX) => return Ok(()),
        Err(error) => {
            return Err(crate::failure::CheckError::failed(format!(
                "invalid toolchain TOML in {}: {error}",
                file.relative_path
            )));
        }
    };
    assert!(
        crate::policy::TOOLCHAIN_FILES
            .iter()
            .any(|name| file.relative_path.ends_with(name)),
        "only toolchain files reach the toolchain check"
    );
    let has_path = table
        .get(TOOLCHAIN_TABLE)
        .and_then(|toolchain| toolchain.get(TOOLCHAIN_PATH_KEY))
        .is_some();
    if has_path {
        return Err(crate::failure::CheckError::violation(
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
            format!(
                "{} sets [toolchain] path; a repository toolchain file may not select a compiler \
                 from a local directory",
                file.relative_path
            ),
        ));
    }
    assert!(
        !has_path,
        "an admitted toolchain file selects no local compiler"
    );
    Ok(())
}
