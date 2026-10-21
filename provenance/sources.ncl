let Contracts = import "contracts.ncl" in
let Constants = Contracts.constants in
{
  schema = Constants.schema_sources,
  sources = [
    {
      id = "choregraph-history",
      repository = "https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git",
      revision = "b3e08e19750f53bdbcae970cdf58a47a791ed20b",
      license = "AGPL-3.0-or-later",
      owner = "OnixResearch (Choregraph workspace repository, rad:zL2ncTUeASVYwcoGkEXv9JKgGbAF)",
      adoption = 'dependency,
      reviewed_files = [
        "crates/choregraph-history/Cargo.toml",
        "crates/choregraph-history/src/lib.rs",
        "crates/choregraph-history/src/model.rs",
        "crates/choregraph-history/src/dag.rs",
        "crates/choregraph-history/src/identity.rs",
        "crates/choregraph-history/src/refs.rs",
        "crates/choregraph-history/src/traversal.rs",
      ],
      adopted_mechanisms = [
        "HistoryGraph and EventDraft immutable application-event admission",
        "EventId and Digest exact source identities",
        "Projection with ProjectionKind::LinearLineage bounded control history",
        "BranchRef and ReferenceMutationPlan explicit head and generation facts",
        "plan_branch_advance generation-fenced branch update plans",
      ],
      campaign_paths = [
        "Cargo.toml",
        "Cargo.lock",
        "flake.nix",
        "flake.lock",
        "config/repository.ncl",
        "crates/campaign-choregraph",
        "provenance/CHOREGRAPH-PUBLICATION.md",
      ],
      retained_authority = "Choregraph owns immutable history, generic projection, identity, branch references, lineage selection, replay, merge envelopes, graph persistence, and effect authority. Campaign consumes only the published history contract.",
      non_adoption = [
        "No Choregraph source is copied into Campaign or read from an ambient sibling worktree.",
        "Campaign does not import the machine-history or Universal Weave adapters.",
      ],
      notes = [
        "Pinned Git dependency under the workspace owner grant; the public AGPL-3.0-or-later license and notices are unchanged.",
      ],
    },
    {
      id = "choregraph-machine",
      repository = "https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git",
      revision = "0809c9f10ba48b8ae0192ab3a29ff4c5b335e56e",
      license = "AGPL-3.0-or-later",
      owner = "OnixResearch (Choregraph workspace repository, rad:zL2ncTUeASVYwcoGkEXv9JKgGbAF)",
      adoption = 'reference_only,
      reviewed_files = [
        "crates/choregraph-core/src/lib.rs",
        "crates/choregraph-core/src/model.rs",
        "crates/choregraph-core/src/transition.rs",
        "crates/choregraph-core/src/admission.rs",
        "crates/choregraph-core/src/boundary.rs",
      ],
      adopted_mechanisms = [
        "Pure machine transitions over owned state",
        "Typed effect intents instead of performed effects",
        "Production replay boundary kept outside policy code",
      ],
      campaign_paths = [
        "crates/campaign-core/src/frontier",
        "crates/campaign-core/src/policy",
      ],
      retained_authority = "Machine admission, execution-graph semantics, proof claims, and replay remain in Choregraph.",
      non_adoption = [
        "Reference only; no machine crate is a Campaign dependency and no source is copied.",
      ],
      notes = [
        "The original review recorded mechanisms but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "choregraph-branchable-history-design",
      repository = "https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git",
      revision = "dc986f60ed99866df5509aff2d1ac0bec12fa940",
      license = "AGPL-3.0-or-later",
      owner = "OnixResearch (Choregraph workspace repository, rad:zL2ncTUeASVYwcoGkEXv9JKgGbAF)",
      adoption = 'reference_only,
      reviewed_files = [
        "cairn/changes/add-branchable-event-history/proposal.md",
        "cairn/changes/add-branchable-event-history/design.md",
        "cairn/changes/add-branchable-event-history/specs/branchable-event-history/spec.md",
      ],
      adopted_mechanisms = [
        "Ownership split: immutable event DAGs, branch references, lineage projection, merge envelopes, and optional Universal Weave topology stay in Choregraph",
      ],
      campaign_paths = [
        "AGENTS.md",
        "README.md",
      ],
      retained_authority = "This revision is historical design provenance only; Choregraph owns every mechanism it describes.",
      non_adoption = [
        "Not a dependency; the ambient branchable-history worktree is never imported.",
      ],
      notes = [
        "The original review recorded the design scope but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "chaoscontrol",
      repository = "https://github.com/brittonr/chaoscontrol",
      revision = "e1dcf2552132221085967250cbc5cb00559e8757",
      license = "AGPL-3.0-or-later",
      owner = "brittonr GitHub account, owner-authorized for OnixResearch extraction",
      adoption = 'reference_only,
      reviewed_files = [
        "crates/chaoscontrol-explore/Cargo.toml",
        "crates/chaoscontrol-explore/src/frontier.rs",
        "crates/chaoscontrol-explore/src/coverage.rs",
        "crates/chaoscontrol-explore/src/explorer.rs",
        "crates/chaoscontrol-explore/src/input_tree.rs",
        "crates/chaoscontrol-explore/src/profile/campaign.rs",
      ],
      adopted_mechanisms = [
        "Adaptive frontier selection with selection-count decay",
        "Coverage-derived guidance supplied as consumer ranks",
        "Input-tree branching with bounded alternatives per selection",
        "Rank-ordered pruning of derived un-outstanding work",
        "Stale-search policy and bounded search stop classes",
      ],
      campaign_paths = [
        "crates/campaign-core/src/policy",
        "crates/campaign-core/src/frontier",
        "provenance/CHAOSCONTROL-BASELINE.md",
      ],
      retained_authority = "Faults, VMs, snapshots, coverage meaning, assertions, bug meaning, replay, minimization, execution, and evidence remain in ChaosControl.",
      non_adoption = [
        "Behavior description only; no ChaosControl source text is copied.",
        "The ambient RNG and f64 scores are replaced by explicit entropy tickets and bounded integer ranks.",
      ],
      notes = [
        "The repository mixes Apache-2.0 and AGPL-3.0-or-later packages under license-policy.tsv; license names the reviewed chaoscontrol-explore package.",
      ],
    },
    {
      id = "animus",
      repository = "https://github.com/OnixResearch/animus",
      revision = "c85fdb35e4e546edcafb368fcb32e0353bf65fc4",
      license = "MIT OR Apache-2.0",
      owner = "OnixResearch",
      adoption = 'reference_only,
      reviewed_files = [
        "crates/animus-adapter-weave/Cargo.toml",
        "crates/animus-adapter-weave/src/lib.rs",
        "crates/animus-adapter-weave/src/adapter.rs",
        "crates/animus-adapter-weave/src/policy.rs",
        "crates/animus-adapter-weave/src/receipt.rs",
      ],
      adopted_mechanisms = [
        "Bounded Universal Weave adapter shape",
        "Authoritative-ledger separation from derived projections",
      ],
      campaign_paths = [
        "crates/campaign-choregraph",
        "crates/campaign-conformance",
      ],
      retained_authority = "Sessions, agents, prompts, model services, compaction, repair, tool authority, and persistence remain in Animus. Universal Weave ownership moves to Choregraph, not Campaign.",
      non_adoption = [
        "Reference only; no Animus crate is a dependency and no source is copied.",
      ],
      notes = [
        "Workspace license is MIT OR Apache-2.0; animus-std-basalt-authority, animus-std-secretspec, and animus-std-workspace-edit are AGPL-3.0-or-later and were not reviewed.",
        "The original review recorded mechanisms but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "lattice",
      repository = "https://github.com/OnixResearch/lattice",
      revision = "9004439b6973d36ddf34475d6b9f6735d34a0857",
      license = "AGPL-3.0-or-later AND MIT",
      owner = "OnixResearch",
      adoption = 'reference_only,
      reviewed_files = [
        "README.md",
        "src/orchestration/mod.rs",
        "src/orchestration/core.rs",
        "src/orchestration/shell.rs",
      ],
      adopted_mechanisms = [
        "Durable orchestration, retry, cancellation, and recovery kept outside policy",
      ],
      campaign_paths = [
        "crates/campaign-core/src/frontier",
      ],
      retained_authority = "Workflow meaning, effect execution, persistence, credentials, triggers, and operator authority remain in Lattice.",
      non_adoption = [
        "Reference only; no Lattice crate is a dependency and no source is copied.",
      ],
      notes = [
        "At this revision the root lattice package declares AGPL-3.0-or-later and [workspace.package] (budget-ledger-core) declares MIT.",
        "The original review recorded mechanisms but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "vm-cohort",
      repository = "rad:z2QJLUqyAZnnHPiZQ1BFjLsX9ush3",
      revision = "ac279d14afa586521cfe2f5a2b229416d2de7ce9",
      license = "MIT",
      owner = "OnixResearch",
      adoption = 'reference_only,
      reviewed_files = [
        "README.md",
        ".cairn/changes/build-vm-cohort/design.md",
        ".cairn/changes/build-vm-cohort/specs/vm-cohort/spec.md",
      ],
      adopted_mechanisms = [
        "Bounded worker cohort composition expressed as an outstanding-selection bound",
      ],
      campaign_paths = [
        "crates/campaign-core/src/policy",
      ],
      retained_authority = "VM checkpoints, clone mechanics, KVM effects, and host cleanup remain in VM Cohort.",
      non_adoption = [
        "Reference only; the revision is planning documents and no source is copied.",
      ],
      notes = [
        "The original review recorded mechanisms but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "transactional-reconciliation-core",
      repository = "rad:z4Tky6zvC8w4Y6c4YBzNxVbq5n752",
      revision = "ff8e41f076805a76f5eeef6fd7e4a4cdec76bfd2",
      license = "MIT",
      owner = "Onix Research",
      adoption = 'reference_only,
      reviewed_files = [
        "README.md",
        ".cairn/changes/extract-transactional-reconciliation-core/design.md",
        ".cairn/changes/extract-transactional-reconciliation-core/specs/transactional-reconciliation-core/spec.md",
      ],
      adopted_mechanisms = [
        "Uncertain outcome acknowledgement",
        "Compare-and-set generation fencing reference pattern",
      ],
      campaign_paths = [
        "crates/campaign-core/src/model",
        "crates/campaign-core/src/policy",
      ],
      retained_authority = "Reconciliation plans and persistence semantics remain in that component.",
      non_adoption = [
        "Reference only; the revision is planning documents and no source is copied.",
      ],
      notes = [
        "The original review recorded mechanisms but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "differential-execution-harness",
      repository = "rad:z2pbwQ4Zd52M4oC1W9bLT3nQem9ay",
      revision = "e6c56cbec76898f1c9ea03f87014fe6977046edc",
      license = "MIT",
      owner = "Onix Research",
      adoption = 'reference_only,
      reviewed_files = [
        "README.md",
        ".cairn/changes/build-differential-execution-harness/design.md",
        ".cairn/changes/build-differential-execution-harness/specs/differential-execution-harness/spec.md",
      ],
      adopted_mechanisms = [
        "Bounded observation adapter patterns",
      ],
      campaign_paths = [
        "crates/campaign-core/src/model",
      ],
      retained_authority = "Differential comparison, normalization, and parity claims remain in that harness.",
      non_adoption = [
        "Reference only; the revision is planning documents and no source is copied.",
      ],
      notes = [
        "The original review recorded mechanisms but not files; reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "luv-frontier-study",
      repository = "https://github.com/mbrock/luv",
      revision = "638390086d236a383acd8f9eb98dea452d91ea2c",
      license = Constants.no_assertion,
      owner = "Mikael Brockman (mbrock GitHub account)",
      adoption = 'reference_only,
      reviewed_files = [
        "wiki/frontier-traversals.org",
      ],
      adopted_mechanisms = [
        "Distinct frontier roles: unfold, reserve, plan, admit, schedule, commit",
        "Reservation before adjacency and commitment after naming",
        "Discovery, monotone relaxation, and invalidation as separate dynamics",
        "Reference implementations isolated to differential evidence with separate identity, work, time, and memory questions",
      ],
      campaign_paths = [
        "crates/campaign-core/src/discipline",
      ],
      retained_authority = "No source is copied. Campaign owns its normative contracts and establishes their behavior through its own fixtures.",
      non_adoption = [
        "Reference only; no Luv code, text, or runtime is imported.",
      ],
      notes = [
        "No license file exists at the repository root at this revision and GitHub reports no license; recorded as SPDX NOASSERTION, which the contract admits only for reference-only sources.",
      ],
    },
    {
      id = "branchcontext",
      repository = "https://github.com/multikernel/branching",
      revision = "b0778564fbc75eb82a61687d211ee6050f9750bf",
      license = "Apache-2.0",
      owner = "multikernel (Cong Wang)",
      adoption = 'reference_only,
      reviewed_files = [
        "LICENSE",
        "README.md",
        "src/branching/agent/patterns.py",
        "src/branching/agent/speculate.py",
      ],
      adopted_mechanisms = [
        "Bounded exploration patterns (best-of-N, beam, tree, tournament, speculation) as opaque candidate shapes for the non-VM conformance fixture",
      ],
      campaign_paths = [
        "crates/campaign-conformance",
      ],
      retained_authority = "BranchContext keeps its Python runtime, FUSE and workspace effects, validator execution, and automatic commit or acceptance policy.",
      non_adoption = [
        "Exploration-pattern reference only; no Python runtime, workspace effect, or automatic acceptance policy is imported.",
      ],
      notes = [
        "reviewed_files were confirmed at this revision on 2026-09-25.",
      ],
    },
    {
      id = "cairn-policy",
      repository = "https://github.com/OnixResearch/cairn",
      revision = "5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b",
      license = "MIT OR Apache-2.0",
      owner = "OnixResearch",
      adoption = 'vendored_copy,
      reviewed_files = [
        "cairn-policy/contracts.ncl",
        "cairn-policy/default.ncl",
        "cairn-policy/generated/cairn-policy.json",
        "cairn-policy/lifecycle-store-contracts.ncl",
        "cairn-policy/nominal-evidence-identities.ncl",
      ],
      adopted_mechanisms = [
        "Vendored Cairn lifecycle policy contracts and generated policy",
      ],
      campaign_paths = [
        "cairn-policy",
      ],
      retained_authority = "Cairn owns the lifecycle policy; copies must stay byte-identical to the pinned source per cairn-policy/source-manifest.ncl.",
      non_adoption = [
        "No Cairn runtime code is copied; only the five policy files listed in the source manifest.",
      ],
      notes = [],
    },
  ],
} | Contracts.Inventory
