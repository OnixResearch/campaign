//! Audit9 finding 7 (D90) and Audit10 (D93): the pinned constants back the record only while a
//! registered, armed test names them in its body and runs the suite (see `reach`); every way
//! of dropping, disarming, or configuring that test, its module, or the library root away is
//! refused with its exact denial under `campaign.consumer_pilots.valid`.

pub(super) const SUITE: &str = crate::policy::CONSUMER_PINNED_SUITE;
const LIBRARY_PATH: &str = "crates/campaign-conformance/src/lib.rs";
const TESTS_PATH: &str = "crates/campaign-conformance/src/tests/mod.rs";
const GATED_TESTS: &str = "#[cfg(test)]\nmod tests;";
const CONSUMERS: &str = "mod consumers;";
const TEST_ATTRIBUTE: &str = "#[test]\nfn pins_are_reproduced";
pub(super) const SUITE_RUN: &str =
    "#[test]\nfn pins_are_reproduced() {\n    let report = crate::run_suite();";
const NO_STD: &str = "#![no_std]";

/// One edit of the registered fixture: a file of the conformance member replaced or removed, or
/// its manifest set.
pub(super) enum Edit {
    Replace(&'static str, &'static str, &'static str),
    Remove(&'static str),
    Manifest(&'static str),
}

fn edited(edit: &Edit) -> Result<Vec<crate::model::Member>, Box<dyn std::error::Error>> {
    let mut members = super::members(super::PINNED_SUITE);
    let member = members
        .first_mut()
        .ok_or("the fixture holds the conformance member")?;
    match edit {
        Edit::Replace(path, old, new) => {
            let file = member
                .files
                .iter_mut()
                .find(|file| file.display_path == *path)
                .ok_or_else(|| format!("no {path}"))?;
            if !file.content.contains(old) {
                return Err(format!("{path} lacks {old}").into());
            }
            file.content = file.content.replacen(old, new, 1);
        }
        Edit::Remove(path) => member.files.retain(|file| file.display_path != *path),
        Edit::Manifest(manifest) => (*manifest).clone_into(&mut member.manifest),
    }
    assert_eq!(members.len(), 1, "one conformance member");
    assert!(
        members.iter().all(|member| !member.files.is_empty()),
        "the edited member keeps a pinned suite"
    );
    Ok(members)
}

pub(super) fn refusal(edit: &Edit) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let result = super::validate(
        &super::facts(&super::record()?)?,
        &super::sources(super::REPOSITORY),
        &edited(edit)?,
    );
    Ok(result.err().map(|error| {
        assert_eq!(
            error.rule(),
            Some(crate::rules::Rule::CONSUMER_PILOTS_VALID)
        );
        error.message().to_owned()
    }))
}

pub(super) fn never_runs(reason: &str) -> String {
    format!("{reason}; the pinned consumer suite test in {SUITE} never runs")
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn admits_a_registered_armed_test_in_either_module_layout() -> super::TestResult {
    let admitted = [
        Edit::Replace(
            TESTS_PATH,
            CONSUMERS,
            "/// Pinned evidence.\nmod consumers;",
        ),
        Edit::Replace(
            SUITE,
            TEST_ATTRIBUTE,
            "#[test]\n#[cfg_attr(test, allow(unused))]\nfn pins_are_reproduced",
        ),
        Edit::Manifest("[lib]\npath = \"src/lib.rs\"\ntest = true\n"),
        Edit::Replace(
            LIBRARY_PATH,
            NO_STD,
            "#![no_std]\n#![forbid(unsafe_code)]\n#![cfg_attr(dylint_lib = \"octet\", feature(register_tool))]",
        ),
        Edit::Replace(
            SUITE,
            SUITE_RUN,
            "fn reproduce() -> crate::Report {\n    crate::run_suite()\n}\n\n#[test]\nfn pins_are_reproduced() {\n    let report = reproduce();",
        ),
    ];
    assert!(!admitted.is_empty());
    for edit in &admitted {
        assert_eq!(refusal(edit)?, None);
    }
    let mut flat = super::members(super::PINNED_SUITE);
    for file in flat.iter_mut().flat_map(|member| member.files.iter_mut()) {
        if file.display_path == TESTS_PATH {
            "crates/campaign-conformance/src/tests.rs".clone_into(&mut file.display_path);
        }
    }
    super::validate(
        &super::facts(&super::record()?)?,
        &super::sources(super::REPOSITORY),
        &flat,
    )?;
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_an_unregistered_module_chain() -> super::TestResult {
    let undeclared = never_runs(&format!(
        "{LIBRARY_PATH} does not declare `#[cfg(test)] mod tests;`"
    ));
    let unconsumed = never_runs(&format!("{TESTS_PATH} does not declare `mod consumers;`"));
    let cases = [
        (
            Edit::Replace(LIBRARY_PATH, GATED_TESTS, ""),
            undeclared.clone(),
        ),
        (
            Edit::Replace(LIBRARY_PATH, GATED_TESTS, "mod tests;"),
            undeclared.clone(),
        ),
        (
            Edit::Replace(
                LIBRARY_PATH,
                GATED_TESTS,
                "#[cfg(all(test, any()))]\nmod tests;",
            ),
            undeclared.clone(),
        ),
        (
            Edit::Replace(
                LIBRARY_PATH,
                GATED_TESTS,
                "#[cfg(test)]\n#[path = \"t.rs\"]\nmod tests;",
            ),
            undeclared.clone(),
        ),
        (
            Edit::Replace(LIBRARY_PATH, GATED_TESTS, "#[cfg(test)]\nmod tests {}"),
            undeclared,
        ),
        (Edit::Replace(TESTS_PATH, CONSUMERS, ""), unconsumed.clone()),
        (
            Edit::Replace(TESTS_PATH, CONSUMERS, "#[cfg(any())]\nmod consumers;"),
            unconsumed,
        ),
        (
            Edit::Remove(TESTS_PATH),
            never_runs("crates/campaign-conformance/src/tests/mod.rs is absent"),
        ),
    ];
    assert!(!cases.is_empty());
    for (edit, expected) in cases {
        assert_eq!(refusal(&edit)?, Some(expected));
    }
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_a_disabled_harness_or_configured_away_module() -> super::TestResult {
    let cases = [
        (
            Edit::Replace(LIBRARY_PATH, NO_STD, "#![cfg(not(test))]\n#![no_std]"),
            never_runs(&format!("{LIBRARY_PATH} is configured by an inner #![cfg]")),
        ),
        (
            Edit::Replace(
                LIBRARY_PATH,
                NO_STD,
                "#![no_std]\n#![cfg_attr(test, cfg(any()))]",
            ),
            never_runs(&format!("{LIBRARY_PATH} is configured by an inner #![cfg]")),
        ),
        (
            Edit::Replace(TESTS_PATH, CONSUMERS, "#![cfg(any())]\nmod consumers;"),
            never_runs(&format!("{TESTS_PATH} is configured by an inner #![cfg]")),
        ),
        (
            Edit::Replace(
                SUITE,
                "//! Fixture",
                "#![cfg_attr(test, cfg(any()))]\n//! Fixture",
            ),
            never_runs(&format!("{SUITE} is configured by an inner #![cfg]")),
        ),
        (
            Edit::Manifest("[lib]\ntest = false\n"),
            never_runs("crates/campaign-conformance/Cargo.toml sets [lib] test = false"),
        ),
        (
            Edit::Manifest("[lib]\nharness = false\n"),
            never_runs("crates/campaign-conformance/Cargo.toml sets [lib] harness = false"),
        ),
        (
            Edit::Manifest("[lib]\npath = \"src/entry/lib.rs\"\n"),
            never_runs("crates/campaign-conformance/src/entry/lib.rs is absent"),
        ),
    ];
    assert!(!cases.is_empty());
    for (edit, expected) in cases {
        assert_eq!(refusal(&edit)?, Some(expected));
    }
    Ok(())
}

/// A library root outside `src` resolves its `tests` module elsewhere, so the pinned module is an
/// orphan even when that root declares the gated module.
#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_a_library_root_outside_the_suite_directory() -> super::TestResult {
    let mut members = edited(&Edit::Manifest("[lib]\npath = \"src/entry/lib.rs\"\n"))?;
    for file in members
        .iter_mut()
        .flat_map(|member| member.files.iter_mut())
    {
        if file.display_path == LIBRARY_PATH {
            "crates/campaign-conformance/src/entry/lib.rs".clone_into(&mut file.display_path);
        }
    }
    assert!(
        members
            .iter()
            .flat_map(|member| member.files.iter())
            .any(|file| file.display_path.ends_with("entry/lib.rs"))
    );
    let refusal = super::validate(
        &super::facts(&super::record()?)?,
        &super::sources(super::REPOSITORY),
        &members,
    )
    .err()
    .map(|error| error.message().to_owned());
    assert_eq!(
        refusal.as_deref(),
        Some(
            never_runs(
                "crates/campaign-conformance/src/entry/lib.rs is the library root, so `mod tests` \
             resolves under crates/campaign-conformance/src/entry/tests"
            )
            .as_str()
        )
    );
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_a_disarmed_or_misnamed_pinned_test() -> super::TestResult {
    let unarmed = never_runs(&format!(
        "{SUITE} holds no top-level #[test] without ignore, should_panic, or cfg attributes that \
         names PINNED_FIXTURE_COUNT, PINNED_TRANSCRIPT_DIGEST, PINNED_REORDERED_DIGEST"
    ));
    let disarmed = |replacement: &'static str| Edit::Replace(SUITE, TEST_ATTRIBUTE, replacement);
    let cases = [
        disarmed("#[test]\n#[ignore]\nfn pins_are_reproduced"),
        disarmed("#[test]\n#[should_panic]\nfn pins_are_reproduced"),
        disarmed("#[test]\n#[cfg_attr(test, ignore = \"slow\")]\nfn pins_are_reproduced"),
        disarmed("#[test]\n#[cfg(any())]\nfn pins_are_reproduced"),
        disarmed("fn pins_are_reproduced"),
        Edit::Replace(
            SUITE,
            "assert_eq!(report.reordered_digest, PINNED_REORDERED_DIGEST);",
            "",
        ),
    ];
    assert!(!cases.is_empty());
    for edit in &cases {
        assert_eq!(refusal(edit)?, Some(unarmed.clone()));
    }
    Ok(())
}
