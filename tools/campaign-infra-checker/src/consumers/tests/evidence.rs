//! Every pilot must claim exactly the evidence the conformance suite pins as exact `const`
//! literals; a stale claim, a missing or inexact constant, or another reordered profile is refused
//! with its exact denial under `campaign.consumer_pilots.valid`.

const SUITE: &str = crate::policy::CONSUMER_PINNED_SUITE;
const COUNT: &str = "const PINNED_FIXTURE_COUNT: usize = 3;";
const TRANSCRIPT: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const REORDERED: &str = "0000000000000000000000000000000000000000000000000000000000000002";
const STALE: &str = "9999999999999999999999999999999999999999999999999999999999999999";
/// A conformance section whose fixture count and both digests are all stale.
const ALL_STALE: &str = r#"{"command": "c", "fixtures": 4,
    "transcript_digest": "9999999999999999999999999999999999999999999999999999999999999999",
    "reordered_profile_digest": "9999999999999999999999999999999999999999999999999999999999999999"}"#;

/// The rule and exact message of a refusal.
type Denial = (Option<crate::rules::Rule>, String);

/// The denial `validate` refuses the record with, when `pinned_suite` is pinned and the record
/// carries `edit`.
fn refusal(
    edit: Option<&super::Edit>,
    pinned_suite: &str,
) -> Result<Option<Denial>, Box<dyn std::error::Error>> {
    let record = match edit {
        Some(edit) => super::edited(edit)?,
        None => super::record()?,
    };
    let result = super::validate(
        &super::facts(&record)?,
        &super::sources(super::REPOSITORY),
        &super::members(pinned_suite),
    );
    Ok(result
        .err()
        .map(|error| (error.rule(), error.message().to_owned())))
}

fn valid(message: String) -> Denial {
    (Some(crate::rules::Rule::CONSUMER_PILOTS_VALID), message)
}

fn edited_suite((old, new): (&str, &str)) -> String {
    assert!(
        super::PINNED_SUITE.contains(old),
        "the pinned suite holds {old}"
    );
    super::PINNED_SUITE.replacen(old, new, 1)
}

fn stale((pilot, differences): (&str, &str)) -> Denial {
    valid(format!(
        "consumer pilot {pilot} claims conformance evidence {SUITE} does not pin: {differences}"
    ))
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_each_claim_that_differs_from_the_suite_constants() -> super::TestResult {
    let reordered = format!("PINNED_REORDERED_DIGEST = {REORDERED}");
    let cases: [(super::Edit, Denial); 3] = [
        (
            (
                "stale fixture count",
                "/consumers/1/conformance/fixtures",
                Some("4"),
            ),
            stale((
                "fixture-consumer",
                "fixtures 4 but PINNED_FIXTURE_COUNT = 3",
            )),
        ),
        (
            (
                "stale reordered digest",
                "/consumers/0/conformance/reordered_profile_digest",
                Some(r#""9999999999999999999999999999999999999999999999999999999999999999""#),
            ),
            stale((
                "fixture-pilot",
                &format!("reordered_profile_digest {STALE} but {reordered}"),
            )),
        ),
        (
            (
                "every claim stale",
                "/consumers/1/conformance",
                Some(ALL_STALE),
            ),
            stale((
                "fixture-consumer",
                &format!(
                    "fixtures 4 but PINNED_FIXTURE_COUNT = 3; transcript_digest {STALE} but \
                     PINNED_TRANSCRIPT_DIGEST = {TRANSCRIPT}; reordered_profile_digest {STALE} \
                     but {reordered}"
                ),
            )),
        ),
    ];
    assert_eq!(refusal(None, super::PINNED_SUITE)?, None);
    for (edit, expected) in cases {
        assert_eq!(
            refusal(Some(&edit), super::PINNED_SUITE)?,
            Some(expected),
            "{}",
            edit.0
        );
    }
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_a_reordered_profile_the_digest_is_not_computed_under() -> super::TestResult {
    let expected = valid(format!(
        "reordered_profile must be exploration_order [1] and retention_rank_order [1], the \
         profile {SUITE} computes PINNED_REORDERED_DIGEST under"
    ));
    let edits: [super::Edit; 3] = [
        (
            "another exploration order",
            "/reordered_profile/exploration_order",
            Some("[2]"),
        ),
        (
            "another retention rank order",
            "/reordered_profile/retention_rank_order",
            Some("[0]"),
        ),
        (
            "a longer retention rank order",
            "/reordered_profile/retention_rank_order",
            Some("[1, 0]"),
        ),
    ];
    for edit in edits {
        assert_eq!(
            refusal(Some(&edit), super::PINNED_SUITE)?,
            Some(expected.clone()),
            "{}",
            edit.0
        );
    }
    assert_eq!(refusal(None, super::PINNED_SUITE)?, None);
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_a_missing_repeated_or_inexact_fixture_count() -> super::TestResult {
    let form =
        format!("{SUITE}: const PINNED_FIXTURE_COUNT must be `usize = <positive decimal literal>`");
    let absent = format!("{SUITE} declares no top-level const PINNED_FIXTURE_COUNT");
    let nested = format!("mod nested {{ {COUNT} }}");
    let cases = [
        ("absent", edited_suite((COUNT, "")), absent.clone()),
        ("nested", edited_suite((COUNT, &nested)), absent),
        (
            "repeated",
            format!("{}\n{COUNT}\n", super::PINNED_SUITE),
            format!("{SUITE} declares const PINNED_FIXTURE_COUNT more than once"),
        ),
        (
            "suffixed",
            edited_suite(("= 3;", "= 3usize;")),
            form.clone(),
        ),
        ("separated", edited_suite(("= 3;", "= 0_3;")), form.clone()),
        (
            "zero-prefixed (Audit9 finding 8: panicked before)",
            edited_suite(("= 3;", "= 03;")),
            form.clone(),
        ),
        (
            "hexadecimal",
            edited_suite(("= 3;", "= 0x3;")),
            form.clone(),
        ),
        ("zero", edited_suite(("= 3;", "= 0;")), form.clone()),
        ("computed", edited_suite(("= 3;", "= 1 + 2;")), form.clone()),
        ("other type", edited_suite(("usize = 3", "u64 = 3")), form),
    ];
    assert!(
        cases
            .iter()
            .all(|(_, suite, _)| suite != super::PINNED_SUITE)
    );
    for (case, suite, expected) in cases {
        assert_eq!(refusal(None, &suite)?, Some(valid(expected)), "{case}");
    }
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_an_inexact_digest_or_a_qualified_declaration() -> super::TestResult {
    let form = format!(
        "{SUITE}: const PINNED_TRANSCRIPT_DIGEST must be `&str = \"<lowercase 64-hex BLAKE3 \
         digest>\"`"
    );
    let not_plain = format!(
        "{SUITE}: const PINNED_FIXTURE_COUNT must be a private, attribute-free, non-generic \
         top-level const"
    );
    let public = format!("pub {COUNT}");
    let configured = format!("#[cfg(any())]\n{COUNT}");
    let quoted = format!("\"{TRANSCRIPT}\"");
    let raw = format!("r{quoted}");
    let upper = TRANSCRIPT.replace('1', "A");
    let cases = [
        ("public", edited_suite((COUNT, &public)), not_plain.clone()),
        (
            "configured away",
            edited_suite((COUNT, &configured)),
            not_plain,
        ),
        (
            "uppercase digest",
            edited_suite((TRANSCRIPT, &upper)),
            form.clone(),
        ),
        ("raw digest", edited_suite((&quoted, &raw)), form.clone()),
        (
            "static digest",
            edited_suite((
                "PINNED_TRANSCRIPT_DIGEST: &str",
                "PINNED_TRANSCRIPT_DIGEST: &'static str",
            )),
            form,
        ),
    ];
    assert!(
        cases
            .iter()
            .all(|(_, suite, _)| suite != super::PINNED_SUITE)
    );
    for (case, suite, expected) in cases {
        assert_eq!(refusal(None, &suite)?, Some(valid(expected)), "{case}");
    }
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_an_unparsable_or_absent_pinned_suite() -> super::TestResult {
    let unparsed = refusal(None, "const")?;
    assert!(
        unparsed.as_ref().is_some_and(|(rule, message)| *rule
            == Some(crate::rules::Rule::CONSUMER_PILOTS_VALID)
            && message.starts_with(&format!("{SUITE} is not valid Rust: "))),
        "{unparsed:?}"
    );
    let mut elsewhere = super::members(super::PINNED_SUITE);
    for file in elsewhere
        .iter_mut()
        .flat_map(|member| member.files.iter_mut())
    {
        file.display_path = "crates/campaign-conformance/src/tests/pilots/mod.rs".to_owned();
    }
    let unpinned = super::validate(
        &super::facts(&super::record()?)?,
        &super::sources(super::REPOSITORY),
        &elsewhere,
    )
    .err()
    .map(|error| (error.rule(), error.message().to_owned()));
    assert_eq!(
        unpinned,
        Some(valid(format!(
            "{SUITE} is absent; campaign-conformance must pin the suite evidence consumer pilots \
             claim"
        )))
    );
    Ok(())
}
