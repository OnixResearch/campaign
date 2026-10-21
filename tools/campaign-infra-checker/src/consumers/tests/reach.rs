//! Audit10 (D93): an armed pinned test counts only when it calls `run_suite` in its body or
//! through a same-module function its body calls; naming the entry point, a suite call in another
//! test or an uncalled function, and a two-hop helper chain are each refused exactly.

const SUITE: &str = super::registration::SUITE;
const SUITE_RUN: &str = super::registration::SUITE_RUN;

#[test]
// r[verify campaign.consumer_pilots.valid]
fn refuses_an_armed_test_that_never_runs_the_suite() -> super::TestResult {
    let suite_free = Some(super::registration::never_runs(&format!(
        "{SUITE} holds no top-level armed #[test] naming the pinned constants that calls \
         run_suite in its body or through a same-module function its body calls, so no suite \
         run reproduces its constants"
    )));
    let cases = [
        super::registration::Edit::Replace(SUITE, "crate::run_suite()", "crate::report()"),
        super::registration::Edit::Replace(
            SUITE,
            SUITE_RUN,
            "#[test]\nfn pins_are_named() {\n    let _ = (PINNED_FIXTURE_COUNT, PINNED_TRANSCRIPT_DIGEST, PINNED_REORDERED_DIGEST);\n}\n\n#[test]\n#[ignore]\nfn pins_are_reproduced() {\n    let report = crate::run_suite();",
        ),
        super::registration::Edit::Replace(
            SUITE,
            SUITE_RUN,
            "fn unused() -> crate::Report {\n    crate::run_suite()\n}\n\n#[test]\nfn pins_are_reproduced() {\n    let report = crate::report();",
        ),
        super::registration::Edit::Replace(
            SUITE,
            SUITE_RUN,
            "fn inner() -> crate::Report {\n    crate::run_suite()\n}\n\nfn outer() -> crate::Report {\n    inner()\n}\n\n#[test]\nfn pins_are_reproduced() {\n    let report = outer();",
        ),
        super::registration::Edit::Replace(SUITE, "crate::run_suite()", "crate::run_suite"),
    ];
    assert!(!cases.is_empty());
    for edit in &cases {
        assert_eq!(super::registration::refusal(edit)?, suite_free);
    }
    Ok(())
}
