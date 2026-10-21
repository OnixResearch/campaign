//! A core-allowlisted dependency is declared once with its approved feature set: every member
//! (exempt or not, in every table) inherits it unchanged, and the workspace entry keeps default
//! features off and lists none.

const APPROVED: &str = "[workspace.dependencies]\nblake3 = { version = '1.8.2', default-features = false }\nhash = { package = 'blake3', version = '1.8.2', default-features = false }";
const SUFFIX: &str = "; a core-allowlisted dependency is declared once, in [workspace.dependencies] \
                      with its approved feature set";

fn table(content: &str) -> Result<toml::Table, crate::failure::CheckError> {
    crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: "fixture/Cargo.toml",
        content,
    })
}

/// The exact denial of one member manifest of `package` under the approved workspace.
fn member((package, content): (&str, &str)) -> Result<Option<String>, crate::failure::CheckError> {
    let row = crate::policy::scope::for_package(package)
        .ok_or_else(|| crate::failure::CheckError::failed("fixture package has no row"))?;
    let result = super::validate_member(crate::dependencies::MemberCheck {
        manifest_path: "fixture/Cargo.toml",
        manifest: &table(content)?,
        workspace: &table(APPROVED)?,
        package: row,
    });
    Ok(result.err().map(|error| {
        assert_eq!(
            error.rule(),
            Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT)
        );
        error.message().to_owned()
    }))
}

fn workspace(content: &str) -> Result<Option<String>, crate::failure::CheckError> {
    Ok(super::validate_workspace(&table(content)?)
        .err()
        .map(|error| {
            assert_eq!(
                error.rule(),
                Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT)
            );
            error.message().to_owned()
        }))
}

#[test]
// r[verify campaign.functional_core.effect]
fn admits_plain_inheritance_in_every_member_and_the_approved_workspace_entry()
-> Result<(), crate::failure::CheckError> {
    let admitted = [
        ("campaign-core", "[dependencies]\nblake3.workspace = true"),
        (
            "campaign-infra-checker",
            "[dependencies]\nhash = { workspace = true, optional = true }\ntoml = { version = '1', features = ['preserve_order'] }\n[features]\nfast = ['dep:hash', 'toml/preserve_order']",
        ),
        (
            "campaign-conformance",
            "[target.'cfg(unix)'.dev-dependencies]\nblake3.workspace = true",
        ),
    ];
    for case in admitted {
        assert_eq!(member(case)?, None, "{case:?}");
    }
    assert_eq!(workspace(APPROVED)?, None);
    assert_eq!(
        workspace(
            "[workspace.dependencies]\nblake3 = { version = '1', default_features = false, features = [] }"
        )?,
        None
    );
    Ok(())
}

/// Audit9 finding 5 and finding 13's `default-features = true`: a feature any member sets on the
/// allowlisted crate unifies into the core's build.
#[test]
// r[verify campaign.functional_core.effect]
fn refuses_member_feature_changes_and_direct_declarations_in_every_member()
-> Result<(), crate::failure::CheckError> {
    let refused =
        |change: &str| format!("fixture/Cargo.toml (campaign-infra-checker) {change}{SUFFIX}");
    let cases = [
        (
            "[dependencies]\nblake3 = { version = '1.8.7', features = ['mmap'] }",
            refused("declares blake3 without inheriting its [workspace.dependencies] entry"),
        ),
        (
            "[dev-dependencies]\nhashing = { package = 'blake3', version = '1' }",
            refused("declares blake3 without inheriting its [workspace.dependencies] entry"),
        ),
        (
            "[dependencies]\nblake3 = { workspace = true, features = ['mmap'] }",
            refused("sets features on blake3"),
        ),
        (
            "[target.'cfg(unix)'.build-dependencies]\nblake3 = { workspace = true, default-features = true }",
            refused("sets default-features on blake3"),
        ),
        (
            "[dependencies]\nhash = { workspace = true, default_features = true }",
            refused("sets default_features on blake3"),
        ),
        (
            "[dependencies]\nhash = { workspace = true, optional = true }\n[features]\nfast = ['hash?/mmap']",
            refused("enables a feature of blake3 through [features] fast = [\"hash?/mmap\"]"),
        ),
    ];
    for (content, expected) in cases {
        assert_eq!(
            member(("campaign-infra-checker", content))?,
            Some(expected),
            "{content}"
        );
    }
    assert_eq!(
        member((
            "campaign-core",
            "[dependencies]\nblake3 = { workspace = true, default-features = true }"
        ))?,
        Some(format!(
            "fixture/Cargo.toml (campaign-core) sets default-features on blake3{SUFFIX}"
        ))
    );
    Ok(())
}

#[test]
// r[verify campaign.functional_core.effect]
fn refuses_a_workspace_entry_outside_the_approved_feature_set()
-> Result<(), crate::failure::CheckError> {
    let refused = |alias: &str, spec: &str| {
        format!(
            "Cargo.toml [workspace.dependencies] {alias} declares core-allowlisted blake3 with \
             {spec}; its approved feature set is default-features = false and no features"
        )
    };
    let cases = [
        ("blake3 = '1.8.2'", refused("blake3", "\"1.8.2\"")),
        (
            "blake3 = { version = '1.8.2' }",
            refused("blake3", "{ version = \"1.8.2\" }"),
        ),
        (
            "blake3 = { version = '1.8.2', default-features = true }",
            refused("blake3", "{ default-features = true, version = \"1.8.2\" }"),
        ),
        (
            "blake3 = { version = '1.8.2', default-features = false, features = ['mmap'] }",
            refused(
                "blake3",
                "{ default-features = false, features = [\"mmap\"], version = \"1.8.2\" }",
            ),
        ),
        (
            "hash = { package = 'blake3', version = '1', default-features = false, default_features = false }",
            refused(
                "hash",
                "{ default-features = false, default_features = false, package = \"blake3\", version = \"1\" }",
            ),
        ),
    ];
    assert!(!cases.is_empty());
    for (entry, expected) in cases {
        assert_eq!(
            workspace(&format!("[workspace.dependencies]\n{entry}"))?,
            Some(expected),
            "{entry}"
        );
    }
    Ok(())
}
