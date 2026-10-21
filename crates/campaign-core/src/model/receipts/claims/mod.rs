#[derive(Clone, Debug, Eq, PartialEq)]
/// One claim a Campaign receipt explicitly does not make.
// r[impl campaign.receipts.non_claims]
pub enum NonClaim {
    /// Public Campaign core API variant.
    ExhaustiveSearch,
    /// Public Campaign core API variant.
    EventTruth,
    /// Public Campaign core API variant.
    OperationSuccess,
    /// Public Campaign core API variant.
    ReplaySuccess,
    /// Public Campaign core API variant.
    FindingCorrectness,
    /// Public Campaign core API variant.
    Authorization,
    /// Public Campaign core API variant.
    ReleaseReadiness,
    /// Public Campaign core API variant.
    SafeDeletion,
}

impl NonClaim {
    /// Every non-claim in canonical receipt order; every receipt lists exactly these.
    pub const ALL: [Self; NON_CLAIM_COUNT] = [
        Self::ExhaustiveSearch,
        Self::EventTruth,
        Self::OperationSuccess,
        Self::ReplaySuccess,
        Self::FindingCorrectness,
        Self::Authorization,
        Self::ReleaseReadiness,
        Self::SafeDeletion,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Campaign MODEL owner: the stable receipt API enumerates independent forbidden claims"
)]
/// Public Campaign core API value.
pub struct ReceiptClaims {
    /// Public Campaign core API value.
    pub exhaustive_search: bool,
    /// Public Campaign core API value.
    pub event_truth: bool,
    /// Public Campaign core API value.
    pub operation_success: bool,
    /// Public Campaign core API value.
    pub replay_success: bool,
    /// Public Campaign core API value.
    pub finding_correctness: bool,
    /// Public Campaign core API value.
    pub authorization: bool,
    /// Public Campaign core API value.
    pub release_readiness: bool,
    /// Public Campaign core API value.
    pub safe_deletion: bool,
}

impl ReceiptClaims {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    #[must_use]
    pub fn no_claims() -> Self {
        Self {
            exhaustive_search: false,
            event_truth: false,
            operation_success: false,
            replay_success: false,
            finding_correctness: false,
            authorization: false,
            release_readiness: false,
            safe_deletion: false,
        }
    }

    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn validate_no_claims(&self) -> crate::error::CampaignResult<()> {
        let forbidden_claims = [
            self.exhaustive_search,
            self.event_truth,
            self.operation_success,
            self.replay_success,
            self.finding_correctness,
            self.authorization,
            self.release_readiness,
            self.safe_deletion,
        ];
        if forbidden_claims.contains(&true) {
            return Err(crate::error::CampaignError::InvalidReceiptClaim);
        }
        Ok(())
    }
}

/// Number of non-claims every Campaign receipt lists.
pub const NON_CLAIM_COUNT: usize = 8;

const NON_CLAIM_EXHAUSTIVE: u32 = 1;

const NON_CLAIM_EVENT_TRUTH: u32 = 2;

const NON_CLAIM_OPERATION: u32 = 3;

const NON_CLAIM_REPLAY: u32 = 4;

const NON_CLAIM_FINDING: u32 = 5;

const NON_CLAIM_AUTH: u32 = 6;

const NON_CLAIM_RELEASE: u32 = 7;

const NON_CLAIM_SAFE_DELETE: u32 = 8;

pub(super) const fn non_claim_code(non_claim: &NonClaim) -> u32 {
    match non_claim {
        NonClaim::ExhaustiveSearch => NON_CLAIM_EXHAUSTIVE,
        NonClaim::EventTruth => NON_CLAIM_EVENT_TRUTH,
        NonClaim::OperationSuccess => NON_CLAIM_OPERATION,
        NonClaim::ReplaySuccess => NON_CLAIM_REPLAY,
        NonClaim::FindingCorrectness => NON_CLAIM_FINDING,
        NonClaim::Authorization => NON_CLAIM_AUTH,
        NonClaim::ReleaseReadiness => NON_CLAIM_RELEASE,
        NonClaim::SafeDeletion => NON_CLAIM_SAFE_DELETE,
    }
}
