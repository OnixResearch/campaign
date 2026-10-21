//! Positive and negative dependency-source controls.

const REPOSITORY: &str = "https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git";
const REVISION: &str = "b3e08e19750f53bdbcae970cdf58a47a791ed20b";
const OTHER_REVISION: &str = "fedcba9876543210fedcba9876543210fedcba98";
const MEMBER_PATH: &str = "crates/campaign-conformance";
const MANIFEST_PATH: &str = "crates/campaign-conformance/Cargo.toml";
const MEMBERS: &[(&str, &str)] = &[
    ("crates/campaign-core", "campaign-core"),
    ("crates/campaign-choregraph/", "campaign-choregraph"),
    (MEMBER_PATH, "campaign-conformance"),
];

fn workspace_manifest() -> String {
    format!(
        "[workspace.dependencies]\n\
         campaign-core = {{ path = 'crates/campaign-core', default-features = false }}\n\
         blake3 = {{ version = '1', default-features = false }}\n\
         choregraph-history = {{ git = '{REPOSITORY}', rev = '{REVISION}' }}\n\
         weave-blake3 = {{ package = 'blake3', path = '../universal-weave/blake3' }}\n\
         other = {{ git = 'https://example.invalid/other.git', rev = '{OTHER_REVISION}' }}\n"
    )
}

fn parse(content: &str) -> Result<toml::Table, crate::failure::CheckError> {
    crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: MANIFEST_PATH,
        content,
    })
}

/// Check one member manifest body against the fixture workspace.
fn check(content: &str) -> Result<(), crate::failure::CheckError> {
    let root = parse(&workspace_manifest())?;
    let manifest = parse(content)?;
    super::validate_member(
        super::MemberManifest {
            member_path: MEMBER_PATH,
            manifest_path: MANIFEST_PATH,
            manifest: &manifest,
        },
        super::Workspace {
            root: &root,
            members: MEMBERS,
            pin: crate::pins::Pin {
                repository: REPOSITORY,
                revision: REVISION,
            },
        },
    )
}

/// The `(rule, message)` a refusal reports; `(None, None)` for acceptance.
type Refusal = (Option<crate::rules::Rule>, Option<String>);

fn refusal(result: Result<(), crate::failure::CheckError>) -> Refusal {
    let error = result.err();
    (
        error.as_ref().and_then(crate::failure::CheckError::rule),
        error.map(|found| found.message().to_owned()),
    )
}

/// The history-leak refusal for `(package, source)` declared by the fixture member.
fn leak((package, source): (&str, &str)) -> Refusal {
    (
        Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK),
        Some(format!(
            "{MANIFEST_PATH} takes dependency {package} from {source}; only registry releases, \
             workspace members, and the pinned Choregraph revision are admitted"
        )),
    )
}

#[test]
// r[verify campaign.boundary.history_leak]
fn admits_registry_member_and_pinned_choregraph_entries() {
    let accepted = [
        "[dependencies]\nblake3 = '1'",
        "[dependencies]\nblake3 = { version = '1', default-features = false }",
        "[dependencies]\nblake3.workspace = true",
        "[dependencies]\ncampaign-core.workspace = true",
        "[dependencies]\ncampaign-choregraph = { path = '../campaign-choregraph' }",
        "[dev-dependencies]\nchoregraph = { path = './../campaign-choregraph/', package = 'campaign-choregraph' }",
        "[dependencies]\nchoregraph-history.workspace = true",
        "[target.'cfg(unix)'.dependencies]\ncampaign-core = { path = '../campaign-core' }",
    ];
    assert!(!accepted.is_empty());
    for case in accepted {
        assert_eq!(refusal(check(case)), (None, None), "{case}");
    }
}

#[test]
// r[verify campaign.boundary.history_leak]
fn rejects_ambient_paths_including_inherited_and_misnamed_members() {
    let cases = [
        (
            "[dependencies]\nblake3 = { path = '../../../universal-weave/blake3' }",
            ("blake3", "path = \"../../../universal-weave/blake3\""),
        ),
        (
            "[dependencies]\nweave-blake3.workspace = true",
            ("blake3", "path = \"../universal-weave/blake3\""),
        ),
        (
            "[dependencies]\ncampaign-core = { path = '../campaign-choregraph' }",
            ("campaign-core", "path = \"../campaign-choregraph\""),
        ),
        (
            "[dependencies]\ncampaign-core = { path = '/repository/crates/campaign-core' }",
            (
                "campaign-core",
                "path = \"/repository/crates/campaign-core\"",
            ),
        ),
        (
            "[dependencies]\ncampaign-core = { path = '../../../campaign/crates/campaign-core' }",
            (
                "campaign-core",
                "path = \"../../../campaign/crates/campaign-core\"",
            ),
        ),
        (
            "[target.'cfg(unix)'.dev-dependencies]\nblake3 = { version = '1', path = '../blake3' }",
            ("blake3", "path = \"../blake3\""),
        ),
    ];
    assert!(!cases.is_empty());
    for (content, expected) in cases {
        assert_eq!(refusal(check(content)), leak(expected), "{content}");
    }
}

#[test]
// r[verify campaign.boundary.history_leak]
fn rejects_git_other_than_the_pinned_choregraph_revision() {
    let pinned_repository = format!("git = \"{REPOSITORY}\"");
    let cases = [
        (
            format!(
                "[dependencies]\nchoregraph-history = {{ git = '{REPOSITORY}', rev = '{OTHER_REVISION}' }}"
            ),
            ("choregraph-history", pinned_repository.as_str()),
        ),
        (
            format!("[dependencies]\nblake3 = {{ git = '{REPOSITORY}', rev = '{REVISION}' }}"),
            ("blake3", pinned_repository.as_str()),
        ),
        (
            "[dependencies]\nother.workspace = true".to_owned(),
            ("other", "git = \"https://example.invalid/other.git\""),
        ),
    ];
    assert!(!cases.is_empty());
    for (content, expected) in cases {
        assert_eq!(refusal(check(&content)), leak(expected), "{content}");
    }
}

#[test]
// r[verify campaign.boundary.history_leak]
fn rejects_alternate_registries_and_overrides() -> Result<(), crate::failure::CheckError> {
    assert_eq!(
        refusal(check(
            "[dependencies]\nblake3 = { version = '1', registry = 'mirror' }"
        )),
        leak(("blake3", "registry = \"mirror\""))
    );
    assert_eq!(
        refusal(check(
            "[dependencies]\nblake3 = { version = '1', registry-index = 'sparse+https://mirror.invalid/' }"
        )),
        leak((
            "blake3",
            "registry-index = \"sparse+https://mirror.invalid/\""
        ))
    );
    assert_eq!(
        refusal(check("[patch.crates-io]\nblake3 = { path = '../blake3' }")),
        (
            Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK),
            Some(format!(
                "{MANIFEST_PATH} overrides dependency sources through [patch]"
            ))
        )
    );
    assert_eq!(
        refusal(super::validate_root(&parse(
            "[replace]\n'blake3:1.8.2' = { path = '../blake3' }"
        )?)),
        (
            Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK),
            Some("Cargo.toml overrides dependency sources through [replace]".to_owned())
        )
    );
    assert_eq!(
        refusal(super::validate_root(&parse(&workspace_manifest())?)),
        (None, None)
    );
    Ok(())
}

/// Audit9 finding 8 class (input-reachable assertion panics): an empty package name, direct or
/// inherited, and a lockfile checked against no named member are refused, never asserted.
#[test]
// r[verify campaign.boundary.history_leak]
fn refuses_empty_package_names_and_an_unnamed_workspace_without_panicking()
-> Result<(), crate::failure::CheckError> {
    let workspace = crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: crate::policy::ROOT_MANIFEST,
        content: "[workspace.dependencies]\nhash = { package = '', version = '1' }\n'' = '1'",
    })?;
    let cases = [
        "[dependencies]\nhash = { package = '', version = '1' }",
        "[dependencies]\n'' = '1'",
        "[dependencies]\nhash.workspace = true",
        "[dependencies]\n''.workspace = true",
    ];
    assert!(!cases.is_empty());
    for case in cases {
        let manifest = crate::dependencies::parse_manifest(crate::model::ManifestText {
            relative_path: "fixture/Cargo.toml",
            content: case,
        })?;
        let error = crate::dependencies::declared(&manifest, &workspace).err();
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some("dependency names an empty package"),
            "{case}"
        );
    }
    let lock = crate::model::NamedText {
        relative_path: crate::policy::CARGO_LOCK,
        content: "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n".to_owned(),
    };
    let pin = crate::pins::Pin {
        repository: "https://example.invalid/choregraph.git",
        revision: "b3e08e19750f53bdbcae970cdf58a47a791ed20b",
    };
    let error = crate::dependencies::lock::validate(&lock, &[], pin).err();
    assert_eq!(
        error.as_ref().map(crate::failure::CheckError::message),
        Some("Cargo.lock cannot be checked: no workspace member manifest names its package")
    );
    Ok(())
}
