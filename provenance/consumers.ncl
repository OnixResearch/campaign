let Contracts = import "contracts.ncl" in
let Constants = Contracts.constants in
# Consumer pilot evidence: every pilot that runs the shared conformance suite, the transcript
# digests it produced, and, for the migrated ChaosControl consumer, its immutable Campaign pins,
# legacy-versus-Campaign parity, adaptive KVM smoke, and retained product authority.
# `campaign-infra-checker boundaries` enforces the structure and requires every pilot's fixture
# count and both digests to equal the constants pinned in campaign-conformance's `tests::consumers`,
# which both healthy Campaign pilots must reproduce exactly.
{
  schema = Constants.schema_consumers,
  # The changed profile under which `reordered_profile_digest` is computed.
  reordered_profile = {
    exploration_order = [1],
    retention_rank_order = [1],
  },
  consumers = [
    {
      id = "animus-shaped-fixture",
      kind = 'in_repo_fixture,
      adapter = "crates/campaign-conformance/src/fixtures/animus_shaped",
      tests = [
        "crates/campaign-conformance/src/tests/pilots/mod.rs",
        "crates/campaign-conformance/src/tests/consumers/mod.rs",
      ],
      conformance = {
        fixtures = 80,
        transcript_digest = "28b873e9873c97b5b57194bbb499503e05fae0dc16633b9fe0ce844e0328b51b",
        reordered_profile_digest = "37b9ff95d8c8d5b1b175be8e3e8445261d68daadddb3d903530b0186976c5bae",
        command = "cargo test --locked -p campaign-conformance --lib -- tests::pilots tests::consumers",
      },
      retained_authority = ['execution, 'acceptance, 'promotion, 'evidence],
      non_claims = [
        "An in-repo non-VM fixture adapter over opaque candidate, workspace, and validator references; it models no Animus runtime.",
        "No rank, selection, exit status, or validator reference becomes an acceptance or promotion claim.",
      ],
    },
    {
      id = "chaoscontrol",
      kind = 'migrated_consumer,
      repository = "https://github.com/brittonr/chaoscontrol",
      adoption = {
        change = "adopt-campaign-core",
        branch = "cairn/adopt-campaign-core-20260906",
        commit = "b94360982280cab0a4e87edd6b2e9ada6c3458b0",
      },
      pins = {
        cargo_campaign_revision = "e7757a5134f631e1ed410df3cae57e57f887c468",
        nix_campaign_revision = "e7757a5134f631e1ed410df3cae57e57f887c468",
        choregraph_revision = "b3e08e19750f53bdbcae970cdf58a47a791ed20b",
      },
      conformance = {
        fixtures = 80,
        transcript_digest = "28b873e9873c97b5b57194bbb499503e05fae0dc16633b9fe0ce844e0328b51b",
        reordered_profile_digest = "37b9ff95d8c8d5b1b175be8e3e8445261d68daadddb3d903530b0186976c5bae",
        command = "cargo test --offline --locked -p chaoscontrol-explore --lib campaign_adapter::tests::conformance",
      },
      parity = {
        corpus_tests = "crates/chaoscontrol-explore/src/explorer/frontier_backend/parity/tests.rs",
        receipt = {
          path = ".cairn/changes/adopt-campaign-core/evidence/kvm-smoke-2026-09-26-frontier-parity-receipt.json",
          blake3 = "13abf29e4d717aaabeffd8a61eb8aca4ed89884ec08a4a7e901c265adfe91c03",
        },
        cutover_ready = true,
        agreeing_decision_classes = [
          { class = 'candidate_order, compared_fields = ["SelectedEntry", "ExploreFlag", "RetainedSet"] },
          { class = 'selected_parents, compared_fields = ["ParentMoment"] },
          { class = 'priority_results, compared_fields = ["RetainedScores", "SelectionCounts"] },
          { class = 'pruning, compared_fields = ["Admitted", "Pruned"] },
          { class = 'budget_use, compared_fields = ["SelectionUse"] },
          { class = 'stop_classes, compared_fields = ["StopClass", "RoundsCompleted"] },
        ],
      },
      kvm_smoke = {
        record = {
          path = ".cairn/changes/adopt-campaign-core/kvm-smoke-2026-09-26.md",
          blake3 = "65d63af8138a71db8349a1f0fc7a972aa45b10bf17ee56e0056e6e0c3261503a",
        },
        receipt = {
          path = ".cairn/changes/adopt-campaign-core/evidence/kvm-smoke-2026-09-26-frontier-parity-receipt.json",
          blake3 = "13abf29e4d717aaabeffd8a61eb8aca4ed89884ec08a4a7e901c265adfe91c03",
        },
        bounds = {
          vms = 1,
          rounds = 5,
          branches = 4,
          ticks = 500,
          seed = 42,
          mode = "fault-schedule",
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
      non_claims = [
        "Parity proves only the recorded corpus and profiles, not exhaustive search, all entropy streams, snapshot durability, KVM correctness, finding truth, or release readiness.",
        "Live KVM coverage, child-moment, and finding agreement between the policies is not claimed; each KVM run is compared by replaying its recorded observations through the other policy.",
        "Campaign, Choregraph, and parity receipts are structural decision evidence only, never VM, fault, assertion, finding, replay, minimization, or release evidence.",
        "The Campaign revision is a development pin, not a published release revision.",
      ],
    },
  ],
} | Contracts.ConsumerRecord
