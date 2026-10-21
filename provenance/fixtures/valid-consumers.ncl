# Plain (uncontracted) consumer pilot entries reused by the consumer record contract fixtures.
let Conformance = {
  fixtures = 3,
  transcript_digest = "0000000000000000000000000000000000000000000000000000000000000001",
  reordered_profile_digest = "0000000000000000000000000000000000000000000000000000000000000002",
  command = "fixture conformance command",
} in
let Artifact = {
  path = "evidence/fixture.json",
  blake3 = "0000000000000000000000000000000000000000000000000000000000000003",
} in
{
  reordered_profile = {
    exploration_order = [1],
    retention_rank_order = [1],
  },
  fixture = {
    id = "fixture-pilot",
    kind = 'in_repo_fixture,
    adapter = "crates/fixture/src/adapter",
    tests = ["crates/fixture/src/tests.rs"],
    conformance = Conformance,
    retained_authority = ['execution, 'evidence],
    non_claims = ["fixture non-claim"],
  },
  migrated = {
    id = "fixture-consumer",
    kind = 'migrated_consumer,
    repository = "https://example.invalid/consumer.git",
    adoption = {
      change = "adopt-fixture",
      branch = "fixture-branch",
      commit = "0123456789abcdef0123456789abcdef01234567",
    },
    pins = {
      cargo_campaign_revision = "1111111111111111111111111111111111111111",
      nix_campaign_revision = "1111111111111111111111111111111111111111",
      choregraph_revision = "2222222222222222222222222222222222222222",
    },
    conformance = Conformance,
    parity = {
      corpus_tests = "src/parity/tests.rs",
      receipt = Artifact,
      cutover_ready = true,
      agreeing_decision_classes = [
        { class = 'candidate_order, compared_fields = ["SelectedEntry"] },
        { class = 'selected_parents, compared_fields = ["ParentMoment"] },
        { class = 'priority_results, compared_fields = ["RetainedScores"] },
        { class = 'pruning, compared_fields = ["Pruned"] },
        { class = 'budget_use, compared_fields = ["SelectionUse"] },
        { class = 'stop_classes, compared_fields = ["StopClass"] },
      ],
    },
    kvm_smoke = {
      record = Artifact,
      receipt = Artifact,
      bounds = {
        vms = 1,
        rounds = 5,
        branches = 4,
        ticks = 500,
        seed = 0,
        mode = "fixture-mode",
        memory_mb = 128,
      },
    },
    retained_authority = [
      'execution,
      'snapshot,
      'coverage,
      'fault,
      'assertion,
      'finding,
      'replay,
      'minimization,
      'evidence,
    ],
    non_claims = ["fixture non-claim"],
  },
}
