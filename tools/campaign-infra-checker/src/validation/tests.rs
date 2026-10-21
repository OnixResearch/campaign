//! Every negative overlay fails with its rule, and the positive root passes.

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
// r[verify campaign.boundary.shared]
// r[verify campaign.choregraph_dependency.valid]
// r[verify campaign.source_inventory.complete]
fn accepts_positive_repository_fixture() -> TestResult {
    let root = super::fixtures::materialize(None)?;
    super::fixtures::check_boundaries(root.path())?;
    crate::validation::validate_cairn_policy_source(&crate::fsio::load_cairn_policy_inputs(
        &super::fixtures::request(root.path()),
    )?)?;
    Ok(())
}

#[test]
// r[verify campaign.boundary.consumer_leak]
// r[verify campaign.boundary.lexical_guard]
// r[verify campaign.chaoscontrol_migration.seed_runner]
// r[verify campaign.choregraph_dependency.blocked]
// r[verify campaign.configuration.stale]
// r[verify campaign.consumer_pilots.leak]
// r[verify campaign.functional_core.effect]
// r[verify campaign.non_vm_candidate_fixture.foreign_types]
// r[verify campaign.replay_boundary.invalid]
fn rejects_each_negative_boundary_fixture_with_its_rule() -> TestResult {
    for (fixture, expected) in super::overlays::NEGATIVE_OVERLAYS {
        let root = super::fixtures::materialize(Some(fixture))?;
        let result = super::fixtures::check_boundaries(root.path());
        let observed = result
            .as_ref()
            .err()
            .and_then(crate::failure::CheckError::rule);
        assert_eq!(
            observed,
            Some(*expected),
            "{fixture} produced {result:?} instead of {}",
            expected.spec_id()
        );
    }
    Ok(())
}

#[test]
fn every_rule_has_a_negative_fixture_and_every_fixture_is_declared() -> TestResult {
    for rule in crate::rules::Rule::ALL {
        let is_covered = super::overlays::NEGATIVE_OVERLAYS
            .iter()
            .map(|(_, expected)| *expected)
            .chain(
                super::consumers::REFUSALS
                    .iter()
                    .map(|(_, expected, _)| *expected),
            )
            .any(|expected| expected == rule);
        assert!(is_covered, "no negative fixture for {}", rule.spec_id());
    }
    let mut on_disk = std::fs::read_dir(super::fixtures::fixture_path(
        super::fixtures::NEGATIVE_ROOT,
    ))?
    .map(|entry| entry.map(|found| found.file_name().to_string_lossy().into_owned()))
    .collect::<Result<Vec<_>, _>>()?;
    on_disk.sort();
    let mut declared: Vec<String> = super::overlays::NEGATIVE_OVERLAYS
        .iter()
        .map(|(fixture, _)| (*fixture).to_owned())
        .chain(
            super::scope::overlays::REFUSALS
                .iter()
                .chain(super::scope::redirects::REFUSALS)
                .chain(super::consumers::REFUSALS)
                .map(|(fixture, _, _)| (*fixture).to_owned()),
        )
        .chain([super::fixtures::MISSING_CAIRN_POLICY_FILE.to_owned()])
        .collect();
    declared.sort();
    assert_eq!(on_disk, declared);
    Ok(())
}

#[test]
fn rejects_missing_cairn_policy_file_fixture() -> TestResult {
    let root = super::fixtures::materialize(Some(super::fixtures::MISSING_CAIRN_POLICY_FILE))?;
    let inputs = crate::fsio::load_cairn_policy_inputs(&super::fixtures::request(root.path()))?;
    let result = crate::validation::validate_cairn_policy_source(&inputs);
    assert!(
        result
            .err()
            .is_some_and(|error| error.message().contains("nominal-evidence-identities.ncl"))
    );
    Ok(())
}

#[test]
fn rejects_module_wide_suppression_in_any_member() -> TestResult {
    let root = super::fixtures::materialize(None)?;
    let policy = root.path().join("crates/campaign-core/src/policy/mod.rs");
    let original = std::fs::read_to_string(&policy)?;
    std::fs::write(
        &policy,
        format!("#![expect(assertion_density)]\n{original}"),
    )?;
    let result = super::fixtures::check_boundaries(root.path());
    assert!(result.err().is_some_and(
        |error| error.rule().is_none() && error.message().contains("file-wide or module-wide")
    ));
    Ok(())
}

/// D66: a root package (an implicit member), the exempt member's history and effect dependencies,
/// and allowlisted packages taken from an ambient source are refused with their exact denial.
#[test]
// r[verify campaign.boundary.shared]
// r[verify campaign.boundary.history_leak]
// r[verify campaign.functional_core.effect]
fn refuses_root_package_exempt_families_and_ambient_sources_exactly() -> TestResult {
    let exempt = |package: &str| {
        format!(
            "tools/campaign-infra-checker/Cargo.toml (campaign-infra-checker) declares dependency \
             {package} refused by its dependency policy"
        )
    };
    let ambient = |manifest: &str, dependency: &str| {
        format!(
            "{manifest} takes dependency {dependency}; only registry releases, workspace members, \
             and the pinned Choregraph revision are admitted"
        )
    };
    let cases = [
        (
            "root-package",
            crate::rules::Rule::BOUNDARY_SHARED,
            "Cargo.toml declares a root [package], an implicit workspace member with no allowlist \
             or source profile"
                .to_owned(),
        ),
        (
            "exempt-weave-dependency",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
            exempt("universal-weave"),
        ),
        (
            "exempt-random-dependency",
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
            exempt("rand"),
        ),
        (
            "core-worktree-dependency",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
            ambient(
                crate::policy::CORE_MANIFEST,
                "blake3 from path = \"../../../universal-weave/blake3\"",
            ),
        ),
        (
            "profile-git-dependency",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
            ambient(
                "crates/campaign-profile/Cargo.toml",
                "serde from git = \"https://github.com/serde-rs/serde\"",
            ),
        ),
        (
            "workspace-patch-override",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
            "Cargo.toml overrides dependency sources through [patch]".to_owned(),
        ),
    ];
    for (fixture, rule, expected) in cases {
        let root = super::fixtures::materialize(Some(fixture))?;
        let error = super::fixtures::check_boundaries(root.path()).err();
        assert_eq!(
            error.as_ref().and_then(crate::failure::CheckError::rule),
            Some(rule),
            "{fixture}: {error:?}"
        );
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some(expected.as_str()),
            "{fixture}"
        );
    }
    Ok(())
}

/// The legacy `[project]` table also makes the workspace root an implicit package.
#[test]
// r[verify campaign.boundary.shared]
fn refuses_a_legacy_root_project_table() -> TestResult {
    let root = super::fixtures::materialize(None)?;
    let manifest = root.path().join(crate::policy::ROOT_MANIFEST);
    let original = std::fs::read_to_string(&manifest)?;
    std::fs::write(
        &manifest,
        format!("{original}\n[project]\nname = \"campaign\"\nversion = \"0.1.0\"\n"),
    )?;
    let error = super::fixtures::check_boundaries(root.path()).err();
    assert_eq!(
        error.as_ref().and_then(crate::failure::CheckError::rule),
        Some(crate::rules::Rule::BOUNDARY_SHARED)
    );
    assert_eq!(
        error.as_ref().map(crate::failure::CheckError::message),
        Some(
            "Cargo.toml declares a root [project], an implicit workspace member with no allowlist or source profile"
        )
    );
    Ok(())
}

/// A comment cannot satisfy the core's inherited `blake3` requirement: a core that names blake3
/// only in a comment passes every source rule but not the parsed inheritance check (a direct
/// registry `blake3` is refused earlier, as every core-allowlisted declaration must inherit).
#[test]
// r[verify campaign.boundary.history_leak]
fn requires_parsed_workspace_inheritance_for_core_blake3() -> TestResult {
    let root = super::fixtures::materialize(None)?;
    std::fs::write(
        root.path().join(crate::policy::CORE_MANIFEST),
        "[package]\nname = \"campaign-core\"\n\n[dependencies]\n# blake3.workspace = true\n\n[dev-dependencies]\nproptest = \"1\"\n",
    )?;
    let error = super::fixtures::check_boundaries(root.path()).err();
    assert_eq!(
        error.as_ref().and_then(crate::failure::CheckError::rule),
        None
    );
    assert_eq!(
        error.as_ref().map(crate::failure::CheckError::message),
        Some(
            "crates/campaign-core/Cargo.toml must inherit blake3 from the workspace dependency entry"
        )
    );
    Ok(())
}
