//! Consumer pilot record admission and repository checks over explicit JSON values.
//!
//! Every case edits the positive fixture root's record, so a case differs from an admitted
//! record in exactly the named way.

mod evidence;
mod reach;
mod registration;
mod rejections;

/// The positive fixture root's consumer record: an in-repo fixture and a migrated consumer.
const RECORD: &str =
    include_str!("../../../fixtures/positive/repository-root/provenance/generated/consumers.json");
/// The positive fixture root's pinned consumer suite, whose constants the record claims.
const PINNED_SUITE: &str = include_str!(
    "../../../fixtures/positive/repository-root/crates/campaign-conformance/src/tests/consumers/mod.rs"
);
/// The positive fixture root's conformance crate root and `tests` module, which register the
/// pinned suite's test.
const LIBRARY: &str = include_str!(
    "../../../fixtures/positive/repository-root/crates/campaign-conformance/src/lib.rs"
);
const TESTS_MODULE: &str = include_str!(
    "../../../fixtures/positive/repository-root/crates/campaign-conformance/src/tests/mod.rs"
);
const CHOREGRAPH_REVISION: &str = "b3e08e19750f53bdbcae970cdf58a47a791ed20b";
const REPOSITORY: &str = "https://example.invalid/reference.git";

/// One edit of the valid record, `(name, pointer, json)`: the value at `pointer` becomes `json`,
/// or is removed when `None`. Array pointers address an element; one past the end appends.
type Edit = (&'static str, &'static str, Option<&'static str>);

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record() -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(RECORD)
}

fn edited(edit: &Edit) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (case, pointer, json) = *edit;
    assert!(pointer.starts_with('/'), "{case}: pointers are absolute");
    let mut value = record()?;
    let (parent, name) = pointer.rsplit_once('/').unwrap_or_default();
    let replacement = json.map(serde_json::from_str).transpose()?;
    let target = value
        .pointer_mut(parent)
        .ok_or_else(|| format!("{case}: no value at {parent}"))?;
    match (target, replacement) {
        (serde_json::Value::Object(object), Some(replacement)) => {
            object.insert(name.to_owned(), replacement);
        }
        (serde_json::Value::Object(object), None) => {
            object.remove(name);
        }
        (serde_json::Value::Array(array), replacement) => {
            edit_element(array, name.parse()?, replacement);
        }
        _ => return Err(format!("{case}: {parent} holds no container").into()),
    }
    assert_ne!(value, record()?, "{case}: the edit changes the record");
    Ok(value)
}

/// Replace, remove, or (one past the end) append the element at `index`.
fn edit_element(
    array: &mut Vec<serde_json::Value>,
    index: usize,
    replacement: Option<serde_json::Value>,
) {
    if index < array.len() {
        array.remove(index);
    }
    if let Some(replacement) = replacement {
        array.insert(index.min(array.len()), replacement);
    }
}

fn exported(value: &serde_json::Value) -> crate::model::NamedText {
    crate::model::NamedText {
        relative_path: crate::policy::CONSUMER_RECORD_JSON,
        content: value.to_string(),
    }
}

fn sources(repository: &str) -> crate::inventory::Sources {
    let entry = |id: &str, repository: &str| crate::inventory::Entry {
        id: id.to_owned(),
        repository: repository.to_owned(),
        revision: CHOREGRAPH_REVISION.to_owned(),
        adoption: crate::inventory::Adoption::ReferenceOnly,
        campaign_paths: vec!["crates".to_owned()],
    };
    crate::inventory::Sources {
        entries: vec![
            entry(
                crate::policy::CHOREGRAPH_SOURCE_ID,
                "https://example.invalid/choregraph.git",
            ),
            entry("consumer", repository),
        ],
    }
}

fn facts(
    value: &serde_json::Value,
) -> Result<crate::model::ConsumerFacts, Box<dyn std::error::Error>> {
    Ok(crate::model::ConsumerFacts {
        pilots: super::json::parse(&exported(value))?,
        absent_campaign_paths: Vec::new(),
    })
}

/// The conformance member holding `pinned_suite` at the pinned-suite path, registered through the
/// fixture's crate root and `tests` module.
fn members(pinned_suite: &str) -> Vec<crate::model::Member> {
    let file = |display_path: &str, content: &str| crate::model::SourceFile {
        display_path: display_path.to_owned(),
        content: content.to_owned(),
    };
    vec![crate::model::Member {
        path: crate::policy::CONFORMANCE_MEMBER.to_owned(),
        manifest: String::new(),
        files: vec![
            file("crates/campaign-conformance/src/lib.rs", LIBRARY),
            file("crates/campaign-conformance/src/tests/mod.rs", TESTS_MODULE),
            file(crate::policy::CONSUMER_PINNED_SUITE, pinned_suite),
        ],
    }]
}

fn validate(
    facts: &crate::model::ConsumerFacts,
    sources: &crate::inventory::Sources,
    members: &[crate::model::Member],
) -> Result<(), crate::failure::CheckError> {
    super::validate(super::Repository {
        facts,
        sources,
        members,
    })
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn admits_pilots_of_both_kinds_that_claim_the_pinned_evidence() -> TestResult {
    let pilots = super::json::parse(&exported(&record()?))?;
    assert_eq!(pilots.pilots.len(), 2);
    assert_eq!(
        pilots.campaign_paths().collect::<Vec<_>>(),
        [
            "crates/campaign-conformance/src/lib.rs",
            "crates/campaign-core/src/tests.rs",
        ]
    );
    let extra_authority = (
        "an authority beyond the required ones",
        "/consumers/1/retained_authority/9",
        Some(r#""promotion""#),
    );
    super::json::parse(&exported(&edited(&extra_authority)?))?;
    validate(
        &facts(&record()?)?,
        &sources(REPOSITORY),
        &members(PINNED_SUITE),
    )?;
    Ok(())
}

#[test]
// r[verify campaign.chaoscontrol_migration]
fn rejects_an_uninventoried_or_foreign_choregraph_migration_and_absent_paths() -> TestResult {
    let foreign = edited(&(
        "foreign Choregraph",
        "/consumers/1/pins/choregraph_revision",
        Some(r#""2222222222222222222222222222222222222222""#),
    ))?;
    let mut absent = facts(&record()?)?;
    absent.absent_campaign_paths = vec!["crates/campaign-core/src/tests.rs".to_owned()];
    let cases = [
        (
            facts(&record()?)?,
            sources("https://example.invalid/other.git"),
            crate::rules::Rule::CHAOSCONTROL_MIGRATION,
        ),
        (
            facts(&foreign)?,
            sources(REPOSITORY),
            crate::rules::Rule::CHAOSCONTROL_MIGRATION,
        ),
        (
            absent,
            sources(REPOSITORY),
            crate::rules::Rule::CONSUMER_PILOTS_VALID,
        ),
    ];
    assert_eq!(cases.len(), 3);
    for (facts, sources, expected) in cases {
        let result = validate(&facts, &sources, &members(PINNED_SUITE));
        assert_eq!(result.err().and_then(|error| error.rule()), Some(expected));
    }
    Ok(())
}
