let Contracts = import "../contracts.ncl" in
let Constants = Contracts.constants in
{
  schema = Constants.schema_profile,
  implementation = 'production,
  owners = {
    bounds = "campaign-core",
    selection = "campaign-core",
    stopping = "consumer-shell",
    retention = "consumer-adapter",
    admission = "campaign-core",
    implementation = "campaign-core",
    versions = "campaign-core",
    receipt_non_claims = "campaign-core",
  },
  versions = {
    campaign_schema = Constants.campaign_schema_version,
    identity_version = Constants.identity_version,
  },
  bounds = {
    maximum_roots = 256,
    maximum_candidates = 65536,
    maximum_candidate_batch = 256,
    maximum_retained_candidates = 4096,
    maximum_selections = 65536,
    maximum_guidance_records = 65536,
    maximum_outstanding_selections = 256,
    maximum_depth = 4096,
    maximum_children_per_result = 256,
    maximum_derived_records = 256,
    maximum_rank_dimensions = 16,
    maximum_rank_magnitude = 4294967295,
    maximum_finding_records = 4096,
    maximum_control_events = 1048576,
    maximum_budget_dimensions = 32,
    maximum_operation_schemas = 64,
    maximum_prune_batch = 256,
  },
  selection = {
    mode = 'exploratory,
    exploratory_threshold_parts_per_million = 100000,
    exploration_order = [0],
    decay = { rule = 'none },
    tie_break = 'candidate_identity,
  },
  stopping = {},
  retention = {
    rank_order = [0],
  },
  admission = {
    duplicate_candidate = 'reject,
  },
  receipt_non_claims = Constants.receipt_non_claims,
} | Contracts.Profile
