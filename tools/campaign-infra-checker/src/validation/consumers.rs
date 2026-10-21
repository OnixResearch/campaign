//! Consumer pilot overlays: each breaks one consumer record or pinned-suite fact the boundary check
//! must read, and is refused under its rule with its exact denial.

/// Every consumer overlay. Messages are asserted exactly below.
pub(super) const REFUSALS: &[super::scope::overlays::Refusal] = &[
    (
        "consumers-absent-test-path",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "consumer record names absent Campaign path: crates/campaign-conformance/src/tests/absent.rs",
    ),
    (
        "consumers-malformed-digest",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "transcript_digest is not a lowercase 64-hex BLAKE3 digest: \
         000000000000000000000000000000000000000000000000000000000000000",
    ),
    (
        "consumers-missing-fixture-count",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "consumer record object lacks field fixtures",
    ),
    (
        "consumers-stale-transcript-digest",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "consumer pilot fixture-consumer claims conformance evidence \
         crates/campaign-conformance/src/tests/consumers/mod.rs does not pin: transcript_digest \
         9999999999999999999999999999999999999999999999999999999999999999 but \
         PINNED_TRANSCRIPT_DIGEST = 0000000000000000000000000000000000000000000000000000000000000001",
    ),
    (
        "consumers-unpinned-suite",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/tests/consumers/mod.rs declares no top-level const \
         PINNED_REORDERED_DIGEST",
    ),
    (
        "consumers-malformed-pin",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/tests/consumers/mod.rs: const PINNED_FIXTURE_COUNT must be \
         `usize = <positive decimal literal>`",
    ),
    (
        "consumers-other-reordered-profile",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "reordered_profile must be exploration_order [1] and retention_rank_order [1], the profile \
         crates/campaign-conformance/src/tests/consumers/mod.rs computes PINNED_REORDERED_DIGEST under",
    ),
    (
        "consumers-zero-prefixed-count",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/tests/consumers/mod.rs: const PINNED_FIXTURE_COUNT must be \
         `usize = <positive decimal literal>`",
    ),
    (
        "consumers-unregistered-suite",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/lib.rs does not declare `#[cfg(test)] mod tests;`; the \
         pinned consumer suite test in crates/campaign-conformance/src/tests/consumers/mod.rs \
         never runs",
    ),
    (
        "consumers-unregistered-module",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/tests/mod.rs does not declare `mod consumers;`; the \
         pinned consumer suite test in crates/campaign-conformance/src/tests/consumers/mod.rs \
         never runs",
    ),
    (
        "consumers-ignored-pinned-test",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/tests/consumers/mod.rs holds no top-level #[test] \
         without ignore, should_panic, or cfg attributes that names PINNED_FIXTURE_COUNT, \
         PINNED_TRANSCRIPT_DIGEST, PINNED_REORDERED_DIGEST; the pinned consumer suite test in \
         crates/campaign-conformance/src/tests/consumers/mod.rs never runs",
    ),
    (
        "consumers-disabled-test-harness",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/Cargo.toml sets [lib] test = false; the pinned consumer suite \
         test in crates/campaign-conformance/src/tests/consumers/mod.rs never runs",
    ),
    (
        "consumers-disabled-library-tests",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/lib.rs is configured by an inner #![cfg]; the pinned \
         consumer suite test in crates/campaign-conformance/src/tests/consumers/mod.rs never runs",
    ),
    (
        "consumers-suite-free-armed-test",
        crate::rules::Rule::CONSUMER_PILOTS_VALID,
        "crates/campaign-conformance/src/tests/consumers/mod.rs holds no top-level armed #[test] \
         naming the pinned constants that calls run_suite in its body or through a same-module \
         function its body calls, so no suite run reproduces its constants; the pinned consumer \
         suite test in crates/campaign-conformance/src/tests/consumers/mod.rs never runs",
    ),
    (
        "consumers-pin-mismatch",
        crate::rules::Rule::CHAOSCONTROL_MIGRATION,
        "cargo Campaign pin 1111111111111111111111111111111111111111 differs from nix Campaign pin \
         3333333333333333333333333333333333333333",
    ),
    (
        "consumers-uninventoried-repository",
        crate::rules::Rule::CHAOSCONTROL_MIGRATION,
        "fixture-consumer.repository https://example.invalid/unrecorded.git is not an inventoried \
         source repository",
    ),
    (
        "consumers-choregraph-mismatch",
        crate::rules::Rule::CHAOSCONTROL_MIGRATION,
        "fixture-consumer.pins.choregraph_revision 2222222222222222222222222222222222222222 differs \
         from Campaign's pinned choregraph-history revision",
    ),
    (
        "consumers-cutover-not-ready",
        crate::rules::Rule::CHAOSCONTROL_MIGRATION_PARITY,
        "parity must be cutover_ready",
    ),
    (
        "consumers-missing-decision-class",
        crate::rules::Rule::CHAOSCONTROL_MIGRATION_PARITY,
        "agreeing_decision_classes lacks budget_use",
    ),
    (
        "consumers-missing-authority",
        crate::rules::Rule::CHAOSCONTROL_MIGRATION_AUTHORITY,
        "retained_authority lacks replay",
    ),
];

#[test]
// r[verify campaign.consumer_pilots.valid]
// r[verify campaign.chaoscontrol_migration]
// r[verify campaign.chaoscontrol_migration.parity]
// r[verify campaign.chaoscontrol_migration.authority]
// r[verify campaign.boundary.lexical_guard]
fn refuses_each_consumer_overlay_with_its_exact_denial() -> Result<(), Box<dyn std::error::Error>> {
    for (fixture, rule, message) in REFUSALS {
        let root = super::fixtures::materialize(Some(fixture))?;
        let error = super::fixtures::check_boundaries(root.path()).err();
        assert_eq!(
            error.as_ref().and_then(crate::failure::CheckError::rule),
            Some(*rule),
            "{fixture}: {error:?}"
        );
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some(*message),
            "{fixture}"
        );
    }
    Ok(())
}
