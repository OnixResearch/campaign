//! `campaign.consumer_pilots.valid`: both healthy Campaign pilots, the Animus-shaped fixture and
//! the float-scoring fixture, produce exactly the pinned fixture count and transcript digests.
//!
//! `provenance/consumers.ncl` records these values for every consumer pilot, and
//! `campaign-infra-checker boundaries` reads the three constants below as exact literals and
//! refuses a record whose claims differ. A suite or planner change therefore fails this test until
//! the constants are re-pinned, and a record that is not refreshed with them fails the checker.

/// Fixtures in the shared conformance suite.
const PINNED_FIXTURE_COUNT: usize = 80;
/// Transcript digest of a healthy pilot on the default profile.
const PINNED_TRANSCRIPT_DIGEST: &str =
    "28b873e9873c97b5b57194bbb499503e05fae0dc16633b9fe0ce844e0328b51b";
/// Transcript digest of a healthy pilot under the record's `reordered_profile`:
/// `exploration_order = [1]`, `retention_rank_order = [1]`.
const PINNED_REORDERED_DIGEST: &str =
    "37b9ff95d8c8d5b1b175be8e3e8445261d68daadddb3d903530b0186976c5bae";

const HEX_RADIX: u32 = 16;
const NIBBLE_BITS: u8 = 4;
const LOW_NIBBLE: u8 = 0x0f;

/// Lowercase hex of a transcript digest, the form the consumer record uses.
fn lower_hex(digest: &[u8]) -> alloc::string::String {
    digest
        .iter()
        .flat_map(|byte| [byte >> NIBBLE_BITS, byte & LOW_NIBBLE])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), HEX_RADIX))
        .collect()
}

/// Each healthy pilot's transcript digest on `profile`, after it passes every pinned fixture.
fn pilot_digests(
    profile: &crate::inputs::SuiteProfile,
) -> Result<alloc::vec::Vec<alloc::string::String>, campaign_core::CampaignError> {
    let reports = [
        crate::catalog::run_suite(
            &mut crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
            profile,
        ),
        crate::catalog::run_suite(
            &mut crate::fixtures::float_scores::FixedPointAdapter::new(),
            profile,
        ),
    ];
    reports
        .iter()
        .map(|report| {
            assert_eq!(super::failures(report), alloc::vec::Vec::new());
            assert_eq!(report.outcomes.len(), PINNED_FIXTURE_COUNT);
            Ok(lower_hex(&report.transcript.digest()?))
        })
        .collect()
}

#[test]
// r[verify campaign.consumer_pilots.valid]
fn both_healthy_pilots_produce_the_pinned_fixture_count_and_digests()
-> Result<(), campaign_core::CampaignError> {
    let mut profile =
        super::default_profile().map_err(|_| campaign_core::CampaignError::InvalidConfig)?;
    assert_eq!(crate::catalog::fixture_names().len(), PINNED_FIXTURE_COUNT);
    assert_eq!(pilot_digests(&profile)?, [PINNED_TRANSCRIPT_DIGEST; 2]);
    profile.exploration_order = alloc::vec![1];
    profile.retention_rank_order = alloc::vec![1];
    assert_eq!(pilot_digests(&profile)?, [PINNED_REORDERED_DIGEST; 2]);
    Ok(())
}
