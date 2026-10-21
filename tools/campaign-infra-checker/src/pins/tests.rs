//! Pin parsing controls for flake inputs, flake lock nodes, Cargo specs, and lock sources.

const URL: &str = "https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git";
const REVISION: &str = "b3e08e19750f53bdbcae970cdf58a47a791ed20b";
const OTHER_REVISION: &str = "0809c9f10ba48b8ae0192ab3a29ff4c5b335e56e";

fn pin() -> super::Pin<'static> {
    super::Pin {
        repository: URL,
        revision: REVISION,
    }
}

fn text(relative_path: &'static str, content: String) -> crate::model::NamedText {
    crate::model::NamedText {
        relative_path,
        content,
    }
}

fn flake_rule(content: String) -> Option<crate::rules::Rule> {
    super::nix::validate_flake(&text(crate::policy::FLAKE_NIX, content), pin())
        .err()
        .and_then(|error| error.rule())
}

fn lock_rule(source: &str) -> Option<crate::rules::Rule> {
    let content = format!(
        "version = 4\n[[package]]\nname = \"choregraph-history\"\nversion = \"0.1.0\"\n{source}"
    );
    super::cargo::validate_lock(&text(crate::policy::CARGO_LOCK, content), pin())
        .err()
        .and_then(|error| error.rule())
}

fn workspace_rule(spec: &str) -> Result<Option<crate::rules::Rule>, crate::failure::CheckError> {
    let content = format!("[workspace.dependencies]\nchoregraph-history = {spec}");
    let root = crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: crate::policy::ROOT_MANIFEST,
        content: &content,
    })?;
    Ok(super::cargo::validate_workspace(&root, pin())
        .err()
        .and_then(|error| error.rule()))
}

#[test]
// r[verify campaign.choregraph_dependency.valid]
fn accepts_both_flake_input_spellings() {
    let nested = format!(
        "{{ inputs = {{ choregraph-history = {{\n url = \"git+{URL}?rev={REVISION}\";\n flake = false; }}; }}; outputs = {{ choregraph-history, ... }}: \"${{choregraph-history}}\"; }}"
    );
    let dotted = format!("{{ inputs.choregraph-history.url = \"git+{URL}?rev={REVISION}\"; }}");
    assert_eq!(flake_rule(nested), None);
    assert_eq!(flake_rule(dotted), None);
}

#[test]
// r[verify campaign.choregraph_dependency.moving]
fn rejects_moving_or_mismatched_flake_inputs() {
    let cases = [
        (
            format!("git+{URL}?ref=main"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            format!("git+{URL}?rev={REVISION}&ref=main"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            format!("git+{URL}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            "path:../choregraph".to_owned(),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            format!("git+{URL}?rev={OTHER_REVISION}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_VALID,
        ),
        (
            format!("git+https://example.invalid/fork.git?rev={REVISION}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_VALID,
        ),
    ];
    for (url, expected) in cases {
        assert_eq!(
            flake_rule(format!(
                "{{ inputs.choregraph-history = {{ url = \"{url}\"; }}; }}"
            )),
            Some(expected),
            "{url}"
        );
    }
    assert_eq!(
        flake_rule("{ inputs = { }; }".to_owned()),
        Some(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_VALID)
    );
}

#[test]
// r[verify campaign.choregraph_dependency.moving]
fn checks_cargo_lock_sources() {
    assert_eq!(
        lock_rule(&format!("source = \"git+{URL}?rev={REVISION}#{REVISION}\"")),
        None
    );
    assert_eq!(
        lock_rule(""),
        Some(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING)
    );
    assert_eq!(
        lock_rule(&format!("source = \"git+{URL}?branch=main#{REVISION}\"")),
        Some(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING)
    );
    assert_eq!(
        lock_rule("source = \"registry+https://github.com/rust-lang/crates.io-index\""),
        Some(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING)
    );
    assert_eq!(
        lock_rule(&format!(
            "source = \"git+{URL}?rev={REVISION}#{OTHER_REVISION}\""
        )),
        Some(crate::rules::Rule::CHOREGRAPH_DEPENDENCY_VALID)
    );
}

#[test]
// r[verify campaign.choregraph_dependency.blocked]
// r[verify campaign.choregraph_dependency.moving]
fn checks_workspace_dependency_specs() -> Result<(), crate::failure::CheckError> {
    assert_eq!(
        workspace_rule(&format!("{{ git = \"{URL}\", rev = \"{REVISION}\" }}"))?,
        None
    );
    let cases = [
        (
            format!("{{ git = \"{URL}\", branch = \"main\" }}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            format!("{{ git = \"{URL}\", tag = \"v0.1.0\" }}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            "{ path = \"../choregraph/crates/choregraph-history\" }".to_owned(),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            "\"0.1.0\"".to_owned(),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            format!("{{ git = \"{URL}\", rev = \"b3e08e1\" }}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_MOVING,
        ),
        (
            format!("{{ git = \"{URL}\", rev = \"{OTHER_REVISION}\" }}"),
            crate::rules::Rule::CHOREGRAPH_DEPENDENCY_VALID,
        ),
    ];
    for (spec, expected) in cases {
        assert_eq!(workspace_rule(&spec)?, Some(expected), "{spec}");
    }
    Ok(())
}
