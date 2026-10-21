//! Closed serde shape of the exported `campaign.profile.v2` JSON.
//!
//! Every record denies unknown fields and decodes only from a JSON object, and every tag is a closed enum
//! decoded only from a JSON string (`crate::closed`), so the decoder rejects any key, tag, or value shape the
//! Nickel contracts do not declare. The top-level document is itself decoded through `crate::closed::record`. Optional keys must be absent rather than `null`, matching the
//! Nickel export, which omits unset optional fields.

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileDocument {
    pub(crate) schema: String,
    #[serde(deserialize_with = "crate::closed::tag")]
    pub(crate) implementation: ImplementationTag,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) owners: crate::load::ProfileOwners,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) versions: VersionsDocument,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) bounds: BoundsDocument,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) selection: SelectionDocument,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) stopping: StoppingDocument,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) retention: RetentionDocument,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) admission: AdmissionDocument,
    #[serde(deserialize_with = "receipt_non_claims")]
    pub(crate) receipt_non_claims: Vec<NonClaimTag>,
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImplementationTag {
    Production,
    ReferenceEvidence,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionsDocument {
    pub(crate) campaign_schema: u64,
    pub(crate) identity_version: u64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(
    clippy::struct_field_names,
    reason = "Campaign PROFILE owner: keys mirror the exported maximum_* bound names one to one"
)]
pub(crate) struct BoundsDocument {
    pub(crate) maximum_roots: u64,
    pub(crate) maximum_candidates: u64,
    pub(crate) maximum_candidate_batch: u64,
    pub(crate) maximum_retained_candidates: u64,
    pub(crate) maximum_selections: u64,
    pub(crate) maximum_guidance_records: u64,
    pub(crate) maximum_outstanding_selections: u64,
    pub(crate) maximum_depth: u64,
    pub(crate) maximum_children_per_result: u64,
    pub(crate) maximum_derived_records: u64,
    pub(crate) maximum_rank_dimensions: u64,
    pub(crate) maximum_rank_magnitude: u64,
    pub(crate) maximum_finding_records: u64,
    pub(crate) maximum_control_events: u64,
    pub(crate) maximum_budget_dimensions: u64,
    pub(crate) maximum_operation_schemas: u64,
    pub(crate) maximum_prune_batch: u64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectionDocument {
    #[serde(deserialize_with = "crate::closed::tag")]
    pub(crate) mode: SelectionModeTag,
    #[serde(default = "absent", deserialize_with = "present")]
    pub(crate) exploratory_threshold_parts_per_million: Option<u64>,
    #[serde(default = "absent", deserialize_with = "present")]
    pub(crate) exploration_order: Option<Vec<u32>>,
    #[serde(deserialize_with = "crate::closed::record")]
    pub(crate) decay: DecayDocument,
    #[serde(deserialize_with = "crate::closed::tag")]
    pub(crate) tie_break: TieBreakTag,
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SelectionModeTag {
    Ranked,
    Exploratory,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecayDocument {
    #[serde(deserialize_with = "crate::closed::tag")]
    pub(crate) rule: DecayRuleTag,
    #[serde(default = "absent", deserialize_with = "present")]
    pub(crate) penalty_per_selection: Option<u64>,
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DecayRuleTag {
    None,
    SubtractFromFirstRank,
}

/// The only declared tie-break; it is fixed by the core planner and only checked here.
#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TieBreakTag {
    CandidateIdentity,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoppingDocument {
    #[serde(default = "absent", deserialize_with = "present")]
    pub(crate) finding_limit: Option<u64>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetentionDocument {
    pub(crate) rank_order: Vec<u32>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdmissionDocument {
    #[serde(deserialize_with = "crate::closed::tag")]
    pub(crate) duplicate_candidate: DuplicateCandidateTag,
}

#[derive(Clone, Copy, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DuplicateCandidateTag {
    Reject,
    Coalesce,
}

#[derive(Clone, Copy, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NonClaimTag {
    ExhaustiveSearch,
    EventTruth,
    OperationSuccess,
    ReplaySuccess,
    FindingCorrectness,
    Authorization,
    ReleaseReadiness,
    SafeDeletion,
}

/// Value of an optional key that the export omits.
const fn absent<T>() -> Option<T> {
    None
}

/// Decodes an optional key that, when present, must hold a value; `null` is rejected as a type error.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Decodes `receipt_non_claims`: at most one string tag per receipt non-claim, so the loader can then name
/// the first missing or repeated non-claim.
fn receipt_non_claims<'de, D>(deserializer: D) -> Result<Vec<NonClaimTag>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::closed::tags(deserializer, campaign_core::NON_CLAIM_COUNT)
}
