let Contracts = import "contracts.ncl" in
let Constants = Contracts.constants in
{
  schema = Constants.schema_repository,
  name = "campaign",
  license = "AGPL-3.0-or-later",
  packages = [
    "campaign-core",
    "campaign-choregraph",
    "campaign-conformance",
  ],
  core_mode = Constants.core_mode,
  primary_exploration_source_revision = "e1dcf2552132221085967250cbc5cb00559e8757",
  choregraph = {
    git_url = Constants.choregraph_git_url,
    history_revision = Constants.choregraph_history_revision,
    selected_full_revision = Constants.choregraph_history_revision,
    source_store_path = "/nix/store/v8h6nyhc04rl657z1l8mdanw42x26dlc-source",
    required_history_schema = Constants.choregraph_history_schema,
    dependency_status = Constants.dependency_status,
  },
  consumers = ["chaoscontrol", "fixture-adapter"],
  non_claims = [
    "A selection decision does not prove that an action started or completed.",
    "A Campaign receipt does not prove exhaustive search, event truth, operation success, replay success, finding correctness, authorization, or release readiness.",
    "Campaign does not own generic history, action execution, persistence, retries, snapshots, domain guidance, or consumer evidence.",
  ],
} | Contracts.Repository
