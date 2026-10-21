//! Controls separate identifiers from comments, strings, attributes, lifetimes, and test items.

const CORE_PATH: &str = "crates/campaign-core/src/policy/mod.rs";

fn core_profile() -> Result<&'static crate::policy::scope::SourceProfile, crate::failure::CheckError>
{
    crate::policy::scope::for_package(crate::policy::CORE_PACKAGE)
        .and_then(|package| package.sources.as_ref())
        .ok_or_else(|| crate::failure::CheckError::failed("core has no source profile"))
}

fn scan(
    (path, content): (&str, &str),
) -> Result<Option<crate::rules::Rule>, crate::failure::CheckError> {
    scan_file((path, content), false)
}

/// Scan one core source, marked as a declared `#[cfg(test)]` module when `is_test_module`.
fn scan_file(
    (path, content): (&str, &str),
    is_test_module: bool,
) -> Result<Option<crate::rules::Rule>, crate::failure::CheckError> {
    let source = crate::model::SourceFile {
        display_path: path.to_owned(),
        content: content.to_owned(),
    };
    Ok(
        super::validate_file(&source, core_profile()?, is_test_module)
            .err()
            .and_then(|error| error.rule()),
    )
}

#[test]
// r[verify campaign.boundary.shared]
fn accepts_neutral_identifiers_and_non_code_text() -> Result<(), crate::failure::CheckError> {
    let content = r#"
        //! Does not implement ChaosControl replay, minimization, std::fs, or BranchContext prompts.
        #![no_std]
        #![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]
        mod replay;
        /// Mentions `static mut` and a severity only in documentation.
        #[derive(Debug)]
        pub enum NonClaim { ReplaySuccess, ReleaseReadiness, FindingCorrectness }
        pub struct SeedIdentity;
        pub fn dispatch_candidate() -> &'static str { "resume seed runner progress" }
        pub fn validate_projection(buffer: &'static mut [u8]) -> &'static str {
            debug_assert!(buffer.len() < 8, "assertion text is a literal");
            let _ = crate::frontier::replay::reconstruct;
            "std::fs::read thread_local! getrandom"
        }
        #[cfg(test)]
        mod tests { fn rejects_absent_inputs_before_assertions() {} }
        #[cfg(test)]
        fn prompt_fixture() -> Validator { Validator }
    "#;
    assert_eq!(scan((CORE_PATH, content))?, None);
    assert_eq!(
        scan_file(
            (
                "crates/campaign-core/src/model/receipts/tests/mod.rs",
                "fn defect_severity_assertions() {}"
            ),
            true
        )?,
        None
    );
    Ok(())
}

#[test]
// r[verify campaign.entropy.ambient]
// r[verify campaign.functional_core.effect]
fn rejects_ambient_effects_in_test_code_too() -> Result<(), crate::failure::CheckError> {
    let cases = [
        "use std::fs;",
        "extern crate std;",
        "fn f() { let _ = std::env::var(\"X\"); }",
        "use core::sync::atomic::AtomicU64;",
        "use core::{cell::Cell, sync::atomic};",
        "static mut COUNTER: u64 = 0;",
        "thread_local! { static X: u8 = 0; }",
        "fn f() -> u64 { rand::random() }",
        "fn f() { getrandom::fill(&mut []); }",
        "async fn f() {}",
        "fn f() { println!(\"x\"); }",
        "#[cfg(test)] mod tests { use std::time::Instant; }",
        "const SEED: &str = env!(\"CAMPAIGN_SEED\");",
        "const AMBIENT: Option<&str> = core::option_env!(\"X\");",
        "const UUID: &[u8] = include_bytes!(\"/proc/sys/kernel/random/uuid\");",
        "const TEXT: &str = ::core::include_str!(\"clock.inc\");",
    ];
    assert!(!cases.is_empty());
    for case in cases {
        assert_eq!(
            scan((CORE_PATH, case))?,
            Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT),
            "{case}"
        );
    }
    assert_eq!(
        scan_file(
            ("crates/campaign-core/src/tests.rs", "static mut X: u8 = 0;"),
            true
        )?,
        Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT)
    );
    Ok(())
}

/// Assert that each `(content, rule)` case in a core source violates exactly that rule.
fn assert_core_rules(
    cases: &[(&str, crate::rules::Rule)],
) -> Result<(), crate::failure::CheckError> {
    assert!(!cases.is_empty());
    for (content, expected) in cases {
        assert_eq!(scan((CORE_PATH, content))?, Some(*expected), "{content}");
    }
    Ok(())
}

#[test]
// r[verify campaign.boundary.history_leak]
// r[verify campaign.replay_boundary.invalid]
fn rejects_replay_and_history_authority_in_core() -> Result<(), crate::failure::CheckError> {
    assert_core_rules(&[
        (
            "pub fn minimize_trace() {}",
            crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
        ),
        (
            "pub struct MergeEnvelope;",
            crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
        ),
        (
            "pub fn execute_replay() {}",
            crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
        ),
        (
            "pub struct ReplayCursor;",
            crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
        ),
        (
            "pub fn select_lineage() {}",
            crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
        ),
        (
            "pub fn choose_semantic_parent() {}",
            crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
        ),
        (
            "use choregraph_history::EventId;",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ),
        (
            "pub struct UniversalWeaveTopology;",
            crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ),
    ])
}

#[test]
// r[verify campaign.consumer_pilots.leak]
// r[verify campaign.findings.semantic_leak]
// r[verify campaign.non_vm_candidate_fixture.foreign_types]
fn rejects_finding_foreign_and_consumer_identifiers_in_core()
-> Result<(), crate::failure::CheckError> {
    assert_core_rules(&[
        (
            "pub struct Severity;",
            crate::rules::Rule::FINDINGS_SEMANTIC_LEAK,
        ),
        (
            "pub fn classify_defect() {}",
            crate::rules::Rule::FINDINGS_SEMANTIC_LEAK,
        ),
        (
            "pub struct AssertionOutcome;",
            crate::rules::Rule::FINDINGS_SEMANTIC_LEAK,
        ),
        (
            "pub struct BranchContext;",
            crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        ),
        (
            "pub fn fuse_mount() {}",
            crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        ),
        (
            "pub struct ModelPrompt;",
            crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        ),
        (
            "pub struct ValidatorVerdict;",
            crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        ),
        (
            "pub struct SoftwareWorld;",
            crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        ),
        (
            "pub fn python_callback() {}",
            crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        ),
        (
            "pub struct ChaosControlFault;",
            crate::rules::Rule::CONSUMER_PILOTS_LEAK,
        ),
        (
            "pub struct KvmSnapshot;",
            crate::rules::Rule::CONSUMER_PILOTS_LEAK,
        ),
        (
            "pub struct AnimusSession;",
            crate::rules::Rule::CONSUMER_PILOTS_LEAK,
        ),
    ])
}

#[test]
// r[verify campaign.chaoscontrol_migration.seed_runner]
fn rejects_seed_runner_authority_in_core() -> Result<(), crate::failure::CheckError> {
    assert_core_rules(&[
        (
            "pub fn resume_seed_runner() {}",
            crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        ),
        (
            "pub struct CheckpointFile;",
            crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        ),
        (
            "pub fn aggregate_reports() {}",
            crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        ),
        (
            "pub struct ProgressRecord;",
            crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        ),
        (
            "pub fn dispatch_seeds_to_workers() {}",
            crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        ),
        (
            "pub struct SeedRunner;",
            crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        ),
    ])
}

#[test]
// r[verify campaign.functional_core]
fn requires_unconditional_no_std_in_core_root() {
    let source = |content: &str| crate::model::SourceFile {
        display_path: "crates/campaign-core/src/lib.rs".to_owned(),
        content: content.to_owned(),
    };
    let expected = "crates/campaign-core/src/lib.rs";
    assert!(super::require_no_std(Some(&source("#![no_std]\npub fn f() {}")), expected).is_ok());
    for content in [
        "pub fn f() {}",
        "#![cfg_attr(not(test), no_std)]",
        "#[no_std] mod x {}",
    ] {
        assert_eq!(
            super::require_no_std(Some(&source(content)), expected)
                .err()
                .and_then(|error| error.rule()),
            Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT),
            "{content}"
        );
    }
    assert_eq!(
        super::require_no_std(None, expected)
            .err()
            .and_then(|error| error.rule()),
        Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT)
    );
}
