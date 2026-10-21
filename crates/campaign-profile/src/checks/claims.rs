//! `receipt_non_claims`: every `campaign_core::NonClaim` exactly once.

/// Every tag in `campaign_core::NonClaim::ALL` order, so position pairs a tag with its non-claim.
const TAGS: [crate::wire::NonClaimTag; campaign_core::NON_CLAIM_COUNT] = [
    crate::wire::NonClaimTag::ExhaustiveSearch,
    crate::wire::NonClaimTag::EventTruth,
    crate::wire::NonClaimTag::OperationSuccess,
    crate::wire::NonClaimTag::ReplaySuccess,
    crate::wire::NonClaimTag::FindingCorrectness,
    crate::wire::NonClaimTag::Authorization,
    crate::wire::NonClaimTag::ReleaseReadiness,
    crate::wire::NonClaimTag::SafeDeletion,
];

/// Accepts the declared list when it names every non-claim exactly once, in any order, and returns the
/// non-claims in canonical `NonClaim::ALL` order.
pub(crate) fn validate(
    declared: &[crate::wire::NonClaimTag],
) -> crate::failure::ProfileResult<[campaign_core::NonClaim; campaign_core::NON_CLAIM_COUNT]> {
    for (tag, non_claim) in TAGS.into_iter().zip(campaign_core::NonClaim::ALL) {
        match declared
            .iter()
            .filter(|candidate| **candidate == tag)
            .count()
        {
            0 => return Err(crate::failure::ProfileError::MissingNonClaim(non_claim)),
            1 => {}
            _ => return Err(crate::failure::ProfileError::DuplicateNonClaim(non_claim)),
        }
    }
    debug_assert_eq!(declared.len(), campaign_core::NON_CLAIM_COUNT);
    Ok(campaign_core::NonClaim::ALL)
}
