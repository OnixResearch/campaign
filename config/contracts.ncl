# Typed contracts for human-authored Campaign configuration.
#
# Every violation message starts with `campaign contract:` so negative fixtures can prove that export failed
# because of a contract, not because of a syntax error or a merge conflict.
let U64Max = 18446744073709551615 in
let U32Max = 4294967295 in
let I64Max = 9223372036854775807 in
# Magnitude of the lowest i64 rank, 2^63.
let RankFloorMagnitude = 9223372036854775808 in
let EntropyScale = 1000000 in
let CampaignSchemaVersion = 1 in
let IdentityVersion = 1 in
let SchemaRepository = "campaign.repository.v4" in
let SchemaProfile = "campaign.profile.v2" in
let CoreMode = "no_std + alloc" in
let PublishedChoregraphHistoryRevision = "b3e08e19750f53bdbcae970cdf58a47a791ed20b" in
let ChoregraphGitUrl = "https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git" in
let ChoregraphHistorySchema = "choregraph-history.branchable.v1" in
let PinnedDependencyStatus = "published-pinned" in
let ReceiptNonClaims = [
  'exhaustive_search,
  'event_truth,
  'operation_success,
  'replay_success,
  'finding_correctness,
  'authorization,
  'release_readiness,
  'safe_deletion,
]
in

let Check = fun description predicate =>
  std.contract.from_validator (fun value =>
    if predicate value then
      'Ok
    else
      'Error { message = "campaign contract: %{description}" }
  )
in
let is_integer_in = fun low high value =>
  std.is_number value
  && std.number.is_integer value
  && value >= low
  && value <= high
in
let is_unique = fun values =>
  let step = fun acc value =>
    if acc.duplicate || std.array.elem value acc.seen then
      { seen = acc.seen, duplicate = true }
    else
      { seen = acc.seen @ [value], duplicate = acc.duplicate }
  in
  !(std.array.fold_left step { seen = [], duplicate = false } values).duplicate
in
let Pinned = fun name expected =>
  Check "%{name} must equal the pinned value %{expected}" (fun value => value == expected)
in
let is_tag_among = fun allowed value => std.is_enum value && std.array.elem value allowed in

let NonEmpty = Check "value must be a non-empty string" (fun value =>
  std.is_string value && std.string.length value > 0
) in
let Bound = Check "bound must be an integer in 1..=18446744073709551615" (is_integer_in 1 U64Max) in
let RankMagnitude = Check "bounds.maximum_rank_magnitude must be an integer in 1..=9223372036854775807" (is_integer_in 1 I64Max) in
let Penalty = Check "decay penalty_per_selection must be an integer in 1..=18446744073709551615" (is_integer_in 1 U64Max) in
let FindingLimit = Check "stopping finding_limit must be an integer in 1..=18446744073709551615" (is_integer_in 1 U64Max) in
let Threshold = Check "exploratory_threshold_parts_per_million must be an integer in 0..=1000000" (is_integer_in 0 EntropyScale) in
let RankIndex = Check "rank index must be an integer in 0..=4294967295" (is_integer_in 0 U32Max) in
let RankOrder = Check "rank order must be a non-empty array of unique rank indices" (fun value =>
  std.is_array value && std.array.length value > 0 && is_unique value
) in
let Tag = fun name allowed =>
  Check "%{name} must be one of the declared enum tags" (is_tag_among allowed)
in
let SelectionMode = Tag "selection.mode" ['ranked, 'exploratory] in
let DecayRule = Tag "selection.decay.rule" ['none, 'subtract_from_first_rank] in
let TieBreak = Tag "selection.tie_break" ['candidate_identity] in
let DuplicateCandidateRule = Tag "admission.duplicate_candidate" ['reject, 'coalesce] in
let Implementation = Tag "implementation" ['production] in
let NonClaim = Tag "receipt non-claim" ReceiptNonClaims in
let AllNonClaims = Check "receipt_non_claims must list every receipt non-claim exactly once" (fun value =>
  std.is_array value
  && std.array.length value == std.array.length ReceiptNonClaims
  && is_unique value
) in
let CampaignSchema = Check "versions.campaign_schema must equal the supported Campaign schema version 1" (fun value =>
  value == CampaignSchemaVersion
) in
let IdentitySchema = Check "versions.identity_version must equal the supported identity version 1" (fun value =>
  value == IdentityVersion
) in

let DecayContract = std.contract.Sequence [
  {
    rule | DecayRule,
    penalty_per_selection | Penalty | optional,
  },
  Check "selection.decay declares penalty_per_selection iff rule is 'subtract_from_first_rank" (fun decay =>
    std.record.has_field "penalty_per_selection" decay == (decay.rule == 'subtract_from_first_rank)
  ),
]
in
let BoundsContract = {
  maximum_roots | Bound,
  maximum_candidates | Bound,
  maximum_candidate_batch | Bound,
  maximum_retained_candidates | Bound,
  maximum_selections | Bound,
  maximum_guidance_records | Bound,
  maximum_outstanding_selections | Bound,
  maximum_depth | Bound,
  maximum_children_per_result | Bound,
  maximum_derived_records | Bound,
  maximum_rank_dimensions | Bound,
  maximum_rank_magnitude | RankMagnitude,
  maximum_finding_records | Bound,
  maximum_control_events | Bound,
  maximum_budget_dimensions | Bound,
  maximum_operation_schemas | Bound,
  maximum_prune_batch | Bound,
}
in
let SelectionContract = {
  mode | SelectionMode,
  exploratory_threshold_parts_per_million | Threshold | optional,
  exploration_order | RankOrder | Array RankIndex | optional,
  decay | DecayContract,
  tie_break | TieBreak,
}
in
let ProfileShape = {
  schema | Pinned "profile schema" SchemaProfile,
  implementation | Implementation,
  owners | {
    bounds | NonEmpty,
    selection | NonEmpty,
    stopping | NonEmpty,
    retention | NonEmpty,
    admission | NonEmpty,
    implementation | NonEmpty,
    versions | NonEmpty,
    receipt_non_claims | NonEmpty,
  },
  versions | {
    campaign_schema | CampaignSchema,
    identity_version | IdentitySchema,
  },
  bounds | BoundsContract,
  selection | SelectionContract,
  stopping | {
    finding_limit | FindingLimit | optional,
  },
  retention | {
    rank_order | RankOrder | Array RankIndex,
  },
  admission | {
    duplicate_candidate | DuplicateCandidateRule,
  },
  receipt_non_claims | AllNonClaims | Array NonClaim,
}
in
let all_below = fun limit values => std.array.all (fun index => index < limit) values in
let exploratory = fun profile => profile.selection.mode == 'exploratory in
let ProfileRelations = std.contract.Sequence [
  Check "bounds.maximum_candidate_batch must not exceed bounds.maximum_candidates" (fun profile =>
    profile.bounds.maximum_candidate_batch <= profile.bounds.maximum_candidates
  ),
  Check "bounds.maximum_retained_candidates must not exceed bounds.maximum_candidates" (fun profile =>
    profile.bounds.maximum_retained_candidates <= profile.bounds.maximum_candidates
  ),
  Check "bounds.maximum_outstanding_selections must not exceed bounds.maximum_selections" (fun profile =>
    profile.bounds.maximum_outstanding_selections <= profile.bounds.maximum_selections
  ),
  Check "bounds.maximum_prune_batch must not exceed bounds.maximum_candidates" (fun profile =>
    profile.bounds.maximum_prune_batch <= profile.bounds.maximum_candidates
  ),
  Check "retention.rank_order indices must be below bounds.maximum_rank_dimensions" (fun profile =>
    all_below profile.bounds.maximum_rank_dimensions profile.retention.rank_order
  ),
  Check "bounds.maximum_control_events must leave room for the admission event and a stop" (fun profile =>
    profile.bounds.maximum_control_events >= 2
  ),
  Check "stopping.finding_limit must not exceed bounds.maximum_finding_records" (fun profile =>
    !(std.record.has_field "finding_limit" profile.stopping)
    || profile.stopping.finding_limit <= profile.bounds.maximum_finding_records
  ),
  Check "selection.exploratory_threshold_parts_per_million must be declared iff selection.mode is 'exploratory" (fun profile =>
    std.record.has_field "exploratory_threshold_parts_per_million" profile.selection == exploratory profile
  ),
  Check "selection.exploration_order must be declared iff selection.mode is 'exploratory" (fun profile =>
    std.record.has_field "exploration_order" profile.selection == exploratory profile
  ),
  Check "selection.exploration_order indices must be below bounds.maximum_rank_dimensions" (fun profile =>
    !(exploratory profile)
    || all_below profile.bounds.maximum_rank_dimensions profile.selection.exploration_order
  ),
  Check "selection.decay penalty_per_selection * (bounds.maximum_selections - 1) + min(bounds.maximum_rank_magnitude, 2^63) must not exceed 2^63" (fun profile =>
    profile.selection.decay.rule != 'subtract_from_first_rank
    || profile.selection.decay.penalty_per_selection * (profile.bounds.maximum_selections - 1)
      + std.number.min profile.bounds.maximum_rank_magnitude RankFloorMagnitude
      <= RankFloorMagnitude
  ),
]
in
let DependencyContract = {
  git_url | Pinned "choregraph.git_url" ChoregraphGitUrl,
  history_revision | Pinned "choregraph.history_revision" PublishedChoregraphHistoryRevision,
  selected_full_revision | Pinned "choregraph.selected_full_revision" PublishedChoregraphHistoryRevision,
  source_store_path | NonEmpty,
  required_history_schema | Pinned "choregraph.required_history_schema" ChoregraphHistorySchema,
  dependency_status | Pinned "choregraph.dependency_status" PinnedDependencyStatus,
}
in
let RepositoryContract = {
  schema | Pinned "repository schema" SchemaRepository,
  name | Pinned "repository name" "campaign",
  license | Pinned "repository license" "AGPL-3.0-or-later",
  packages | Array NonEmpty,
  core_mode | Pinned "core_mode" CoreMode,
  primary_exploration_source_revision | NonEmpty,
  choregraph | DependencyContract,
  consumers | Array NonEmpty,
  non_claims | Array NonEmpty,
}
in
{
  constants = {
    schema_repository = SchemaRepository,
    schema_profile = SchemaProfile,
    core_mode = CoreMode,
    choregraph_git_url = ChoregraphGitUrl,
    choregraph_history_revision = PublishedChoregraphHistoryRevision,
    choregraph_history_schema = ChoregraphHistorySchema,
    dependency_status = PinnedDependencyStatus,
    campaign_schema_version = CampaignSchemaVersion,
    identity_version = IdentityVersion,
    receipt_non_claims = ReceiptNonClaims,
  },
  Repository = RepositoryContract,
  Profile = std.contract.Sequence [ProfileShape, ProfileRelations],
}
