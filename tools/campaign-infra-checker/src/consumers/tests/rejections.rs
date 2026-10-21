//! Each edit breaks one structural fact and is rejected under that fact's specification rule.

const FIXTURE_COUNT: &str = "/consumers/0/conformance/fixtures";
const MIGRATED_DIGEST: &str = "/consumers/1/conformance/transcript_digest";
const NIX_PIN: &str = "/consumers/1/pins/nix_campaign_revision";
const CUTOVER_READY: &str = "/consumers/1/parity/cutover_ready";
const FIRST_CLASS: &str = "/consumers/1/parity/agreeing_decision_classes/0/class";
const FIRST_AUTHORITY: &str = "/consumers/1/retained_authority/0";

/// Pilot evidence that is malformed or incomplete.
const PILOT_EDITS: &[super::Edit] = &[
    (
        "wrong schema",
        "/schema",
        Some(r#""campaign.consumers.v0""#),
    ),
    ("unknown kind", "/consumers/0/kind", Some(r#""fork""#)),
    ("no kind", "/consumers/0/kind", None),
    (
        "pilot is not an object",
        "/consumers/0",
        Some(r#""fixture""#),
    ),
    ("missing fixture count", FIXTURE_COUNT, None),
    ("zero fixture count", FIXTURE_COUNT, Some("0")),
    ("fractional fixture count", FIXTURE_COUNT, Some("2.5")),
    (
        "uppercase digest",
        MIGRATED_DIGEST,
        Some(r#""AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA""#),
    ),
    (
        "short digest",
        MIGRATED_DIGEST,
        Some(r#""000000000000000000000000000000000000000000000000000000000000001""#),
    ),
    (
        "duplicate id",
        "/consumers/1/id",
        Some(r#""fixture-pilot""#),
    ),
    ("fixture claims parity", "/consumers/0/parity", Some("{}")),
    (
        "escaping test path",
        "/consumers/0/tests",
        Some(r#"["../outside.rs"]"#),
    ),
    ("empty non-claims", "/consumers/1/non_claims", Some("[]")),
    (
        "unknown fixture authority",
        "/consumers/0/retained_authority/0",
        Some(r#""scheduling""#),
    ),
    (
        "repeated rank order",
        "/reordered_profile/exploration_order",
        Some("[1, 1]"),
    ),
    ("no pilots", "/consumers", Some("[]")),
    ("no migrated consumer", "/consumers/1", None),
    ("no in-repo fixture", "/consumers/0", None),
];

/// Campaign pins or adoption facts that are moving, malformed, or disagree.
const MIGRATION_EDITS: &[super::Edit] = &[
    (
        "cargo and nix pins differ",
        NIX_PIN,
        Some(r#""3333333333333333333333333333333333333333""#),
    ),
    ("short nix pin", NIX_PIN, Some(r#""1111111""#)),
    (
        "uppercase Choregraph pin",
        "/consumers/1/pins/choregraph_revision",
        Some(r#""B3E08E19750F53BDBCAE970CDF58A47A791ED20B""#),
    ),
    (
        "short adoption commit",
        "/consumers/1/adoption/commit",
        Some(r#""0123456""#),
    ),
    (
        "malformed change id",
        "/consumers/1/adoption/change",
        Some(r#""Adopt Fixture""#),
    ),
    (
        "blank repository",
        "/consumers/1/repository",
        Some(r#"" ""#),
    ),
];

/// Parity or KVM smoke evidence that does not establish cutover over every decision class.
const PARITY_EDITS: &[super::Edit] = &[
    ("not cutover ready", CUTOVER_READY, Some("false")),
    ("stringly cutover ready", CUTOVER_READY, Some(r#""true""#)),
    (
        "malformed receipt digest",
        "/consumers/1/parity/receipt/blake3",
        Some(r#""3333333333333333333333333333333333333333""#),
    ),
    (
        "empty compared fields",
        "/consumers/1/parity/agreeing_decision_classes/0/compared_fields",
        Some("[]"),
    ),
    (
        "unknown decision class",
        FIRST_CLASS,
        Some(r#""coverage_order""#),
    ),
    ("repeated decision class", FIRST_CLASS, Some(r#""pruning""#)),
    (
        "without candidate order",
        "/consumers/1/parity/agreeing_decision_classes/0",
        None,
    ),
    (
        "without selected parents",
        "/consumers/1/parity/agreeing_decision_classes/1",
        None,
    ),
    (
        "without priority results",
        "/consumers/1/parity/agreeing_decision_classes/2",
        None,
    ),
    (
        "without pruning",
        "/consumers/1/parity/agreeing_decision_classes/3",
        None,
    ),
    (
        "without budget use",
        "/consumers/1/parity/agreeing_decision_classes/4",
        None,
    ),
    (
        "without stop classes",
        "/consumers/1/parity/agreeing_decision_classes/5",
        None,
    ),
    (
        "zero smoke rounds",
        "/consumers/1/kvm_smoke/bounds/rounds",
        Some("0"),
    ),
    (
        "absolute smoke record",
        "/consumers/1/kvm_smoke/record/path",
        Some(r#""/tmp/record.md""#),
    ),
];

/// A migrated consumer that cedes, repeats, or invents retained authority.
const AUTHORITY_EDITS: &[super::Edit] = &[
    (
        "unknown authority",
        FIRST_AUTHORITY,
        Some(r#""scheduling""#),
    ),
    ("repeated authority", FIRST_AUTHORITY, Some(r#""replay""#)),
    (
        "without execution",
        "/consumers/1/retained_authority/0",
        None,
    ),
    (
        "without snapshot",
        "/consumers/1/retained_authority/1",
        None,
    ),
    (
        "without coverage",
        "/consumers/1/retained_authority/2",
        None,
    ),
    ("without fault", "/consumers/1/retained_authority/3", None),
    (
        "without assertion",
        "/consumers/1/retained_authority/4",
        None,
    ),
    ("without finding", "/consumers/1/retained_authority/5", None),
    ("without replay", "/consumers/1/retained_authority/6", None),
    (
        "without minimization",
        "/consumers/1/retained_authority/7",
        None,
    ),
    (
        "without evidence",
        "/consumers/1/retained_authority/8",
        None,
    ),
];

fn assert_rejected(edits: &[super::Edit], expected: crate::rules::Rule) -> super::TestResult {
    assert!(!edits.is_empty());
    for edit in edits {
        let result = crate::consumers::json::parse(&super::exported(&super::edited(edit)?));
        let observed = result
            .as_ref()
            .err()
            .and_then(crate::failure::CheckError::rule);
        assert_eq!(observed, Some(expected), "{}: {result:?}", edit.0);
    }
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn rejects_malformed_or_incomplete_pilot_evidence() -> super::TestResult {
    let invalid = crate::model::NamedText {
        relative_path: crate::policy::CONSUMER_RECORD_JSON,
        content: "{".to_owned(),
    };
    let observed = crate::consumers::json::parse(&invalid)
        .err()
        .and_then(|error| error.rule());
    assert_eq!(observed, Some(crate::rules::Rule::CONSUMER_PILOTS_VALID));
    assert_rejected(PILOT_EDITS, crate::rules::Rule::CONSUMER_PILOTS_VALID)
}

#[test]
// r[verify campaign.chaoscontrol_migration]
fn rejects_moving_or_mismatched_campaign_pins() -> super::TestResult {
    assert_rejected(MIGRATION_EDITS, crate::rules::Rule::CHAOSCONTROL_MIGRATION)
}

#[test]
// r[verify campaign.chaoscontrol_migration.parity]
fn rejects_parity_without_cutover_over_every_decision_class() -> super::TestResult {
    assert_rejected(
        PARITY_EDITS,
        crate::rules::Rule::CHAOSCONTROL_MIGRATION_PARITY,
    )
}

#[test]
// r[verify campaign.chaoscontrol_migration.authority]
fn rejects_a_migrated_consumer_that_cedes_product_authority() -> super::TestResult {
    assert_rejected(
        AUTHORITY_EDITS,
        crate::rules::Rule::CHAOSCONTROL_MIGRATION_AUTHORITY,
    )
}
