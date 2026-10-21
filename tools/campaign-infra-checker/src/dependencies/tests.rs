//! Positive and negative allowlist controls.

const WORKSPACE: &str = "[workspace.dependencies]\nblake3 = '1'\nalias = { package = 'choregraph-history', version = '1' }\ncampaign-core = { path = 'crates/campaign-core' }";

/// Check one `(package, manifest)` case against its package row.
fn check((package, content): (&str, &str)) -> Result<(), crate::failure::CheckError> {
    let row = crate::policy::scope::for_package(package)
        .ok_or_else(|| crate::failure::CheckError::failed("fixture package has no row"))?;
    let manifest = super::parse_manifest(crate::model::ManifestText {
        relative_path: "fixture/Cargo.toml",
        content,
    })?;
    let workspace = super::parse_manifest(crate::model::ManifestText {
        relative_path: crate::policy::ROOT_MANIFEST,
        content: WORKSPACE,
    })?;
    super::validate_member(super::MemberCheck {
        manifest_path: "fixture/Cargo.toml",
        manifest: &manifest,
        workspace: &workspace,
        package: row,
    })
}

fn rule(case: (&str, &str)) -> Option<crate::rules::Rule> {
    check(case).err().and_then(|error| error.rule())
}

#[test]
// r[verify campaign.boundary.shared]
fn accepts_direct_inherited_and_dev_only_allowances() {
    let accepted = [
        ("campaign-core", "[dependencies]\nblake3 = '1'"),
        (
            "campaign-core",
            "[dependencies]\nblake3.workspace = true\n[dev-dependencies]\nproptest = '1'",
        ),
        (
            "campaign-choregraph",
            "[dependencies]\ncampaign-core.workspace = true\nalias.workspace = true",
        ),
        (
            "campaign-conformance",
            "[dev-dependencies]\nchoregraph-history = '1'",
        ),
        (
            "campaign-profile",
            "[dependencies]\nserde = '1'\nserde_json = '1'",
        ),
        ("campaign-infra-checker", "[dependencies]\ntokio = '1'"),
    ];
    assert!(!accepted.is_empty());
    for case in accepted {
        assert!(check(case).is_ok(), "rejected {case:?}");
    }
}

#[test]
// r[verify campaign.boundary.history_leak]
fn rejects_renamed_and_inherited_history_package_in_core() {
    assert_eq!(
        rule((
            "campaign-core",
            "[dependencies]\ncg = { package = 'choregraph-history', version = '1' }"
        )),
        Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK)
    );
    assert_eq!(
        rule(("campaign-core", "[dependencies]\nalias.workspace = true")),
        Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK)
    );
}

/// A Weave-named dependency, direct, renamed, or Choregraph-packaged, is a Choregraph history
/// leak for the functional core and the adapter; a name merely containing `weave` is only
/// outside the allowlist.
#[test]
// r[verify campaign.boundary.history_leak]
fn rejects_weave_named_crates_as_history_leaks() {
    let cases = [
        ("campaign-core", "[dependencies]\nweave = '1'"),
        ("campaign-core", "[dependencies]\nuniversal-weave = '1'"),
        ("campaign-core", "[dev-dependencies]\nweave-topology = '1'"),
        (
            "campaign-core",
            "[dependencies]\ntopology = { package = 'universal-weave', version = '1' }",
        ),
        ("campaign-core", "[dependencies]\nchoregraph-weave = '1'"),
        ("campaign-choregraph", "[dependencies]\nweave = '1'"),
    ];
    for case in cases {
        assert_eq!(
            rule(case),
            Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK),
            "{case:?}"
        );
    }
    assert_eq!(
        rule(("campaign-core", "[dependencies]\ninterweave = '1'")),
        Some(crate::rules::Rule::BOUNDARY_SHARED)
    );
}

#[test]
// r[verify campaign.boundary.consumer_leak]
// r[verify campaign.entropy.ambient]
fn rejects_effect_and_consumer_families_in_every_table() {
    let cases = [
        (
            "campaign-core",
            "[target.'cfg(unix)'.dependencies]\nrand = '1'",
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        ),
        (
            "campaign-choregraph",
            "[build-dependencies]\ngetrandom = '1'",
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        ),
        (
            "campaign-core",
            "[dependencies]\nalias = { package = 'chaoscontrol-explore', version = '1' }",
            crate::rules::Rule::BOUNDARY_CONSUMER_LEAK,
        ),
        (
            "campaign-conformance",
            "[dev-dependencies]\ntokio = '1'",
            crate::rules::Rule::BOUNDARY_CONSUMER_LEAK,
        ),
        (
            "campaign-profile",
            "[dependencies]\nrusqlite = '1'",
            crate::rules::Rule::BOUNDARY_CONSUMER_LEAK,
        ),
        (
            "campaign-choregraph",
            "[dependencies]\nkvm-ioctls = '1'",
            crate::rules::Rule::BOUNDARY_CONSUMER_LEAK,
        ),
    ];
    assert!(!cases.is_empty());
    for (package, content, expected) in cases {
        assert_eq!(
            rule((package, content)),
            Some(expected),
            "{package}: {content}"
        );
    }
}

/// The exempt tooling member keeps consumer and unlisted dependencies but not ambient randomness
/// or Choregraph-owned history, in any table, by name or by package.
#[test]
// r[verify campaign.boundary.history_leak]
// r[verify campaign.functional_core.effect]
fn rejects_history_and_effect_families_in_exempt_members() {
    let cases = [
        (
            "[dependencies]\nrand = '1'",
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        ),
        (
            "[build-dependencies]\nentropy = { package = 'getrandom', version = '1' }",
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        ),
        (
            "[dependencies]\nuniversal-weave = { path = '../../../universal-weave' }",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ),
        (
            "[dev-dependencies]\nweave = '1'",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ),
        (
            "[target.'cfg(unix)'.dependencies]\nalias.workspace = true",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ),
    ];
    for (content, expected) in cases {
        assert_eq!(
            rule(("campaign-infra-checker", content)),
            Some(expected),
            "{content}"
        );
    }
    assert!(
        check((
            "campaign-infra-checker",
            "[dependencies]\nsyn = '3'\nhyper = '1'"
        ))
        .is_ok()
    );
}

#[test]
fn rejects_packages_outside_the_allowlist() {
    let cases = [
        ("campaign-core", "[dependencies]\nhashbrown = '1'"),
        ("campaign-core", "[dependencies]\nproptest = '1'"),
        ("campaign-choregraph", "[dev-dependencies]\nproptest = '1'"),
        (
            "campaign-conformance",
            "[dependencies]\ncampaign-profile = '1'",
        ),
        ("campaign-profile", "[dependencies]\ntoml = '1'"),
    ];
    assert_eq!(
        rule((
            "campaign-conformance",
            "[dependencies]\nchoregraph-history = '1'"
        )),
        Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK)
    );
    for case in cases {
        assert_eq!(
            rule(case),
            Some(crate::rules::Rule::BOUNDARY_SHARED),
            "{case:?}"
        );
    }
}

#[test]
fn rejects_malformed_inheritance_and_tables() {
    let cases = [
        "[dependencies]\nmissing.workspace = true",
        "[dependencies]\nblake3.workspace = false",
        "[dependencies]\nx = false",
        "[dependencies]\nx.package = false",
        "target = false",
        "[target]\nunix = false",
        "dependencies = false",
    ];
    assert!(!cases.is_empty());
    for case in cases {
        let result = check(("campaign-core", case));
        assert!(
            result.as_ref().is_err_and(|error| error.rule().is_none()),
            "accepted invalid shape: {case}"
        );
    }
}

#[test]
fn rejects_invalid_cargo_manifest_toml() {
    let result = check(("campaign-core", "[dependencies"));
    assert!(
        result
            .err()
            .is_some_and(|error| error.message().contains("invalid Cargo manifest TOML"))
    );
}

/// Cargo treats `-` and `_` alike, so a family matches both spellings and only whole names or
/// separator-delimited prefixes.
#[test]
// r[verify campaign.boundary.history_leak]
fn matches_families_across_separator_spellings() {
    let family = |dependency: &str, family: &str| {
        super::names::in_family(super::names::FamilyMatch { dependency, family })
    };
    for (dependency, name) in [
        ("rand", "rand"),
        ("rand_core", "rand"),
        ("rand-chacha", "rand"),
        ("choregraph_history", "choregraph"),
        ("async_std", "async-std"),
        ("tokio_postgres", "tokio-postgres"),
    ] {
        assert!(family(dependency, name), "{dependency} is outside {name}");
    }
    for (dependency, name) in [
        ("randomizer", "rand"),
        ("ran", "rand"),
        ("weaver_tools", "weave"),
        ("my_rand", "rand"),
    ] {
        assert!(!family(dependency, name), "{dependency} is inside {name}");
    }
}
