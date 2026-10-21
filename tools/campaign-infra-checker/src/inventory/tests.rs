//! Admission and declaration controls over explicit JSON and source values.

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn entry() -> serde_json::Value {
    serde_json::json!({
        "id": "fixture-source",
        "repository": "https://example.invalid/fixture.git",
        "revision": REVISION,
        "license": "MIT OR Apache-2.0",
        "owner": "fixture owner",
        "adoption": "reference_only",
        "reviewed_files": ["README.md"],
        "adopted_mechanisms": ["fixture mechanism"],
        "campaign_paths": ["crates/campaign-core/src"],
        "retained_authority": "fixture authority",
        "non_adoption": ["fixture non-adoption"],
        "notes": [],
    })
}

fn exported(sources: &[serde_json::Value]) -> crate::model::NamedText {
    crate::model::NamedText {
        relative_path: crate::policy::SOURCE_INVENTORY_JSON,
        content: serde_json::json!({
            "schema": crate::policy::SOURCE_INVENTORY_SCHEMA,
            "sources": sources,
        })
        .to_string(),
    }
}

/// The fixture entry with each `(field, value)` replaced.
fn with(replacements: &[(&str, serde_json::Value)]) -> serde_json::Value {
    let mut changed = entry();
    if let Some(object) = changed.as_object_mut() {
        for (field, value) in replacements {
            object.insert((*field).to_owned(), value.clone());
        }
    }
    changed
}

fn without(field: &str) -> serde_json::Value {
    let mut changed = entry();
    if let Some(object) = changed.as_object_mut() {
        object.remove(field);
    }
    changed
}

fn source((path, content): (&str, &str)) -> crate::model::SourceFile {
    crate::model::SourceFile {
        display_path: path.to_owned(),
        content: content.to_owned(),
    }
}

fn admission_rule(sources: &[serde_json::Value]) -> Option<crate::rules::Rule> {
    super::json::parse(&exported(sources))
        .err()
        .and_then(|error| error.rule())
}

#[test]
// r[verify campaign.source_inventory.complete]
fn admits_complete_immutable_entries() -> Result<(), crate::failure::CheckError> {
    let other = with(&[("id", serde_json::json!("other-source"))]);
    let radicle = with(&[
        ("id", serde_json::json!("radicle-source")),
        (
            "repository",
            serde_json::json!("rad:z2QJLUqyAZnnHPiZQ1BFjLsX9ush3"),
        ),
    ]);
    let sources = super::json::parse(&exported(&[entry(), other, radicle]))?;
    assert_eq!(sources.entries.len(), 3);
    assert_eq!(
        sources.entry("fixture-source").map(|found| found.adoption),
        Some(super::Adoption::ReferenceOnly)
    );
    Ok(())
}

#[test]
// r[verify campaign.source_inventory]
fn rejects_incomplete_entries() {
    let cases = [
        ("missing authority", vec![without("retained_authority")]),
        (
            "unknown field",
            vec![with(&[("branch", serde_json::json!("main"))])],
        ),
        (
            "empty authority",
            vec![with(&[("retained_authority", serde_json::json!(" "))])],
        ),
        (
            "empty mechanisms",
            vec![with(&[("adopted_mechanisms", serde_json::json!([]))])],
        ),
        (
            "escaping path",
            vec![with(&[(
                "campaign_paths",
                serde_json::json!(["../outside"]),
            )])],
        ),
        (
            "absolute reviewed file",
            vec![with(&[(
                "reviewed_files",
                serde_json::json!(["/etc/passwd"]),
            )])],
        ),
        (
            "bad id",
            vec![with(&[("id", serde_json::json!("Fixture_Source"))])],
        ),
        (
            "unknown adoption",
            vec![with(&[("adoption", serde_json::json!("fork"))])],
        ),
        ("duplicate id", vec![entry(), entry()]),
        ("no sources", vec![]),
    ];
    assert!(!cases.is_empty());
    for (name, sources) in cases {
        assert_eq!(
            admission_rule(&sources),
            Some(crate::rules::Rule::SOURCE_INVENTORY_COMPLETE),
            "{name}"
        );
    }
}

#[test]
// r[verify campaign.source_inventory]
fn rejects_moving_or_unlicensed_entries() {
    let cases = [
        (
            "short revision",
            vec![("revision", serde_json::json!("0123456"))],
        ),
        (
            "uppercase revision",
            vec![("revision", serde_json::json!(REVISION.to_uppercase()))],
        ),
        (
            "tree URL",
            vec![(
                "repository",
                serde_json::json!("https://github.com/o/r/tree/main"),
            )],
        ),
        (
            "query URL",
            vec![(
                "repository",
                serde_json::json!("https://example.invalid/r.git?ref=main"),
            )],
        ),
        (
            "ssh URL",
            vec![("repository", serde_json::json!("git@example.invalid:r.git"))],
        ),
        (
            "lowercase operator",
            vec![("license", serde_json::json!("MIT or Apache-2.0"))],
        ),
        (
            "transferred NOASSERTION",
            vec![
                ("license", serde_json::json!("NOASSERTION")),
                ("adoption", serde_json::json!("dependency")),
            ],
        ),
    ];
    assert!(!cases.is_empty());
    for (name, changes) in cases {
        assert_eq!(
            admission_rule(&[with(&changes)]),
            Some(crate::rules::Rule::SOURCE_INVENTORY_COMPLETE),
            "{name}"
        );
    }
}

#[test]
fn rejects_wrong_schema_and_invalid_json() {
    let wrong_schema = crate::model::NamedText {
        relative_path: crate::policy::SOURCE_INVENTORY_JSON,
        content: serde_json::json!({"schema": "campaign.provenance.v0", "sources": [entry()]})
            .to_string(),
    };
    let invalid = crate::model::NamedText {
        relative_path: crate::policy::SOURCE_INVENTORY_JSON,
        content: "{".to_owned(),
    };
    for text in [wrong_schema, invalid] {
        assert_eq!(
            super::json::parse(&text)
                .err()
                .and_then(|error| error.rule()),
            Some(crate::rules::Rule::SOURCE_INVENTORY_COMPLETE)
        );
    }
}

#[test]
fn admits_declarations_that_name_a_mapped_source() -> Result<(), crate::failure::CheckError> {
    let sources = super::json::parse(&exported(&[entry()]))?;
    super::declarations::validate(
        &sources,
        &source((
            "crates/campaign-core/src/policy/ranking.rs",
            "// campaign-source: fixture-source\npub fn rank() {}\n",
        )),
    )?;
    super::declarations::validate(
        &sources,
        &source(("crates/campaign-core/src/lib.rs", "pub fn plain() {}")),
    )
}

#[test]
// r[verify campaign.source_inventory.missing]
fn rejects_unknown_unmapped_and_malformed_declarations() -> Result<(), crate::failure::CheckError> {
    let sources = super::json::parse(&exported(&[entry()]))?;
    let cases = [
        (
            "crates/campaign-core/src/lib.rs",
            "// campaign-source: unrecorded-source",
        ),
        (
            "crates/campaign-choregraph/src/lib.rs",
            "// campaign-source: fixture-source",
        ),
        (
            "crates/campaign-core/src/lib.rs",
            "//campaign-source: fixture-source",
        ),
        (
            "crates/campaign-core/src/lib.rs",
            "/* campaign-source: fixture-source */",
        ),
        (
            "crates/campaign-core/src/lib.rs",
            "// campaign-source: fixture-source extra",
        ),
        ("crates/campaign-core/src/lib.rs", "// campaign-source: "),
    ];
    assert!(!cases.is_empty());
    for case in cases {
        let result = super::declarations::validate(&sources, &source(case));
        assert_eq!(
            result.err().and_then(|error| error.rule()),
            Some(crate::rules::Rule::SOURCE_INVENTORY_MISSING),
            "{case:?}"
        );
    }
    Ok(())
}
