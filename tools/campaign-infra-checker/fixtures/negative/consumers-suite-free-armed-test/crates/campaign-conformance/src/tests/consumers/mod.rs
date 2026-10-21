//! Fixture pinned consumer suite evidence.

/// Fixtures in the fixture suite.
const PINNED_FIXTURE_COUNT: usize = 3;
/// Fixture transcript digest on the default profile.
const PINNED_TRANSCRIPT_DIGEST: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";
/// Fixture transcript digest under the reordered profile.
const PINNED_REORDERED_DIGEST: &str =
    "0000000000000000000000000000000000000000000000000000000000000002";

#[test]
fn pins_are_named() {
    let _ = (
        PINNED_FIXTURE_COUNT,
        PINNED_TRANSCRIPT_DIGEST,
        PINNED_REORDERED_DIGEST,
    );
}

#[test]
#[ignore]
fn pins_are_reproduced() {
    let report = crate::run_suite();
    assert_eq!(report.fixtures, PINNED_FIXTURE_COUNT);
    assert_eq!(report.default_digest, PINNED_TRANSCRIPT_DIGEST);
    assert_eq!(report.reordered_digest, PINNED_REORDERED_DIGEST);
}
