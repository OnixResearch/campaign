//! The configuration walk finds Cargo configuration at any depth and through a symlinked `.cargo`,
//! never descends a symlinked directory, and skips only the checker's own fixture roots.

#[test]
// r[verify campaign.boundary.history_leak]
fn finds_cargo_files_at_any_depth_and_through_symlinked_cargo_directories()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = |relative: &str| root.path().join(relative);
    for directory in [
        "deep/er/.cargo",
        "crates",
        "tools/campaign-infra-checker/fixtures/negative/probe/.cargo",
        "tools/campaign-infra-checker/fixtures-extra/.cargo",
    ] {
        std::fs::create_dir_all(path(directory))?;
    }
    std::fs::write(path("deep/er/.cargo/config"), "")?;
    std::fs::write(
        path("tools/campaign-infra-checker/fixtures/negative/probe/.cargo/config.toml"),
        "",
    )?;
    std::fs::write(
        path("tools/campaign-infra-checker/fixtures-extra/.cargo/config.toml"),
        "",
    )?;
    std::fs::write(path("crates/rust-toolchain"), "1.96.0\n")?;
    std::os::unix::fs::symlink(path("deep/er/.cargo"), path("crates/.cargo"))?;
    std::os::unix::fs::symlink(path("deep"), path("linked"))?;
    let found = super::discover(root.path())?;
    assert_eq!(
        found.cargo_configurations,
        [
            "crates/.cargo/config",
            "deep/er/.cargo/config",
            "tools/campaign-infra-checker/fixtures-extra/.cargo/config.toml",
        ]
    );
    let toolchains: Vec<&str> = found
        .toolchain_files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    assert_eq!(toolchains, ["crates/rust-toolchain"]);
    Ok(())
}
