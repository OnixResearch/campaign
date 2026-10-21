//! Typed profile loading failures.

/// Profile loader result type.
pub type ProfileResult<T> = Result<T, ProfileError>;

/// Category of a JSON decoding failure, mirroring `serde_json::error::Category`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeCategory {
    /// The text is not syntactically valid JSON.
    Syntax,
    /// The JSON is valid but does not match the closed profile shape: unknown or missing field,
    /// duplicate field, wrong type (including a float where an integer is required), or unknown enum tag.
    Data,
    /// The text ended before a complete JSON value.
    Eof,
    /// The reader failed; unreachable for in-memory text but kept so the mapping is total.
    Io,
}

/// Fail-closed profile loading errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileError {
    /// The JSON did not decode into the closed `campaign.profile.v2` shape.
    Decode {
        /// Decoding failure category.
        category: DecodeCategory,
        /// Decoder message naming the offending field, tag, or type and its position.
        message: String,
    },
    /// `schema` is not exactly `campaign.profile.v2`.
    UnsupportedProfileSchema,
    /// `versions.campaign_schema` differs from `campaign_core::CAMPAIGN_SCHEMA_VERSION`.
    UnsupportedCampaignSchema,
    /// `versions.identity_version` differs from `campaign_core::identity::ID_VERSION_ONE`.
    UnsupportedIdentityVersion,
    /// `implementation` selects reference evidence; runtime profiles must select the production policy.
    ReferenceImplementation,
    /// An owner is empty.
    EmptyOwner {
        /// Owner group key.
        group: &'static str,
    },
    /// A bound is zero.
    ZeroBound {
        /// Bound key.
        bound: &'static str,
    },
    /// A bound exceeds the bound that contains it.
    BoundExceedsLimit {
        /// Bound key that is too large.
        bound: &'static str,
        /// Containing bound key.
        limit: &'static str,
    },
    /// `maximum_rank_magnitude` exceeds `campaign_core::MAX_RANK_MAGNITUDE` (`i64::MAX`).
    RankMagnitudeOutOfRange,
    /// The exploratory threshold exceeds `campaign_core::ENTROPY_SCALE` parts per million.
    ThresholdOverScale,
    /// The exploratory threshold and exploration order must be present exactly when the mode is exploratory.
    ExploratoryFieldsMismatch,
    /// The decay penalty must be present exactly when the rule is `subtract_from_first_rank`.
    DecayPenaltyMismatch,
    /// The decay penalty is zero.
    ZeroDecayPenalty,
    /// `subtract_from_first_rank` is not representable under the bounds:
    /// `penalty_per_selection * (maximum_selections - 1) + min(maximum_rank_magnitude, 2^63)` exceeds `2^63`.
    DecayExceedsRankDomain,
    /// The stopping finding limit is zero.
    ZeroFindingLimit,
    /// The stopping finding limit exceeds `maximum_finding_records`; settlement caps recorded findings at that
    /// bound, so the declared stop could never fire.
    FindingLimitExceedsFindingBound,
    /// A rank-dimension order is empty.
    EmptyOrder {
        /// Order key.
        order: &'static str,
    },
    /// A rank-dimension order repeats an index.
    DuplicateOrderIndex {
        /// Order key.
        order: &'static str,
        /// Repeated rank-dimension index.
        index: u32,
    },
    /// A rank-dimension order names an index at or above `maximum_rank_dimensions`.
    OrderIndexOutOfRange {
        /// Order key.
        order: &'static str,
        /// Out-of-range rank-dimension index.
        index: u32,
    },
    /// `receipt_non_claims` omits a non-claim.
    MissingNonClaim(campaign_core::NonClaim),
    /// `receipt_non_claims` repeats a non-claim.
    DuplicateNonClaim(campaign_core::NonClaim),
    /// An exploratory profile was built without an entropy source identity.
    MissingEntropySource,
    /// A ranked profile was built with an entropy source identity it would ignore.
    UnexpectedEntropySource,
    /// A Campaign core constructor rejected a profile or runtime value.
    Campaign(campaign_core::CampaignError),
}

impl From<campaign_core::CampaignError> for ProfileError {
    fn from(error: campaign_core::CampaignError) -> Self {
        Self::Campaign(error)
    }
}

impl From<serde_json::Error> for ProfileError {
    fn from(error: serde_json::Error) -> Self {
        Self::Decode {
            category: decode_category(error.classify()),
            message: error.to_string(),
        }
    }
}

const fn decode_category(category: serde_json::error::Category) -> DecodeCategory {
    match category {
        serde_json::error::Category::Syntax => DecodeCategory::Syntax,
        serde_json::error::Category::Data => DecodeCategory::Data,
        serde_json::error::Category::Eof => DecodeCategory::Eof,
        serde_json::error::Category::Io => DecodeCategory::Io,
    }
}

impl core::fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode { category, message } => {
                write!(formatter, "profile JSON {category:?} error: {message}")
            }
            Self::UnsupportedProfileSchema => formatter.write_str("unsupported profile schema"),
            Self::UnsupportedCampaignSchema => formatter.write_str("unsupported campaign schema"),
            Self::UnsupportedIdentityVersion => formatter.write_str("unsupported identity version"),
            Self::ReferenceImplementation => {
                formatter.write_str("reference implementation selected by a runtime profile")
            }
            Self::EmptyOwner { group } => write!(formatter, "owner `{group}` is empty"),
            Self::ZeroBound { bound } => write!(formatter, "bound `{bound}` is zero"),
            Self::BoundExceedsLimit { bound, limit } => {
                write!(formatter, "bound `{bound}` exceeds `{limit}`")
            }
            Self::RankMagnitudeOutOfRange => {
                formatter.write_str("bound `maximum_rank_magnitude` exceeds i64::MAX")
            }
            Self::ThresholdOverScale => formatter.write_str("exploratory threshold exceeds scale"),
            Self::ExploratoryFieldsMismatch => formatter
                .write_str("exploratory fields must be present exactly in exploratory mode"),
            Self::DecayPenaltyMismatch => formatter
                .write_str("decay penalty must be present exactly for subtract_from_first_rank"),
            Self::ZeroDecayPenalty => formatter.write_str("decay penalty is zero"),
            Self::DecayExceedsRankDomain => formatter.write_str(
                "decay penalty over `maximum_selections - 1` selections leaves the i64 rank domain",
            ),
            Self::ZeroFindingLimit => formatter.write_str("finding limit is zero"),
            Self::FindingLimitExceedsFindingBound => {
                formatter.write_str("finding limit exceeds `maximum_finding_records`")
            }
            Self::EmptyOrder { order } => write!(formatter, "order `{order}` is empty"),
            Self::DuplicateOrderIndex { order, index } => {
                write!(formatter, "order `{order}` repeats index {index}")
            }
            Self::OrderIndexOutOfRange { order, index } => {
                write!(formatter, "order `{order}` index {index} is out of range")
            }
            Self::MissingNonClaim(non_claim) => {
                write!(formatter, "missing non-claim {non_claim:?}")
            }
            Self::DuplicateNonClaim(non_claim) => {
                write!(formatter, "duplicate non-claim {non_claim:?}")
            }
            Self::MissingEntropySource => formatter.write_str("missing entropy source identity"),
            Self::UnexpectedEntropySource => {
                formatter.write_str("entropy source identity supplied for a ranked profile")
            }
            Self::Campaign(error) => write!(formatter, "campaign core rejected profile: {error:?}"),
        }
    }
}

impl std::error::Error for ProfileError {}
