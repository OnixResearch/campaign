## Phase 1: Baselines and dependency readiness

- [x] [serial] r[campaign.source_inventory] Record exact Choregraph, ChaosControl, Animus, Lattice, and supporting source revisions, paths, licenses, adopted behavior, and retained authority.
- [x] [serial] r[campaign.chaoscontrol_migration.parity] Record the focused ChaosControl frontier, coverage-priority, input-tree, pruning, budget, and stop-policy baseline.
- [x] [serial] r[campaign.choregraph_dependency.blocked] Record the current Choregraph branchable-history publication blocker and the evidence required to clear it.
- [x] [depends:campaign.choregraph_dependency.blocked] r[campaign.choregraph_dependency] Pin the published Choregraph history revision through Cargo and Nix after publication.
- [x] [depends:campaign.choregraph_dependency] r[campaign.boundary] Confirm exact Choregraph event, branch-reference, projection, lineage, and receipt contracts before adapter implementation.

## Phase 2: Repository and pure core

- [x] [serial] r[campaign.functional_core] Create the Rust workspace, `no_std + alloc` core, pinned Nix inputs, and strict Octet deny-all gate.
- [x] [depends:campaign.functional_core] r[campaign.boundary] Add dependency checks that reject Choregraph, Universal Weave, consumer, executor, persistence, VM, framework, and evidence types from `campaign-core`.
- [x] [depends:campaign.functional_core] r[campaign.identity] Implement domain-separated BLAKE3 identities for campaigns, projections, candidates, decisions, guidance, budgets, adapters, state, and receipts.
- [x] [parallel] r[campaign.identity.missing] Add missing, malformed, crossed, unsupported-version, and changed-field identity fixtures.

## Phase 3: Dynamic frontier policy

- [x] [depends:campaign.identity] r[campaign.initial_admission] Implement bounded root admission without a complete future operation inventory.
- [x] [depends:campaign.initial_admission] r[campaign.candidate_admission] Implement canonical reusable candidate admission over opaque operation profiles and cost profiles.
- [x] [depends:campaign.candidate_admission] r[campaign.frontier] Reconstruct retained candidates, guidance generations, selection counts, and outstanding selections from one control lineage.
- [x] [depends:campaign.frontier] r[campaign.guidance] Implement versioned guidance records, bounded integer ranks, updates, and ordering profiles.
- [x] [depends:campaign.guidance] r[campaign.entropy] Implement explicit bounded entropy tickets for deterministic exploratory selection.
- [x] [depends:campaign.entropy] r[campaign.selection] Implement exact candidate eligibility, ranking, and selection decisions.
- [x] [depends:campaign.selection] r[campaign.budgets] Implement checked resource-budget admission, reservation, reconstruction, and exhaustion decisions.
- [x] [parallel] r[campaign.frontier.conflict] Add duplicate, crossed-control, missing-admission, stale-state, ambiguous-lineage, over-depth, overflow, and over-budget fixtures.

## Phase 4: Observations, findings, pruning, and stop

- [x] [depends:campaign.selection] r[campaign.retirement] Implement exact candidate retirement without history deletion.
- [x] [depends:campaign.selection] r[campaign.observations] Implement exact result admission for outstanding selections and bounded child moments.
- [x] [depends:campaign.selection] r[campaign.unobserved] Implement explicit unobserved closure with uncertainty acknowledgement.
- [x] [depends:campaign.observations] r[campaign.findings] Implement bounded opaque finding references and non-terminal finding behavior.
- [x] [depends:campaign.frontier] r[campaign.pruning] Implement deterministic frontier-only pruning and unchanged-state rejection.
- [x] [depends:campaign.budgets] r[campaign.wait] Implement explicit guidance, outstanding-result, and outstanding-capacity wait decisions.
- [x] [depends:campaign.wait] r[campaign.stop] Implement frontier, budget, policy, and consumer stop decisions with explicit incomplete-search claims.
- [x] [depends:campaign.observations] r[campaign.fork] Enforce distinct child event identities for exploration forks.
- [x] [depends:campaign.selection] r[campaign.retry_boundary] Reject infrastructure retries as new exploration work.
- [x] [parallel] r[campaign.observations.crossed] Add crossed, duplicate, stale, oversized, unknown-selection, invalid-child, and conflicting-observation fixtures.
- [x] [parallel] r[campaign.pruning.invalid] Add history rewrite, outstanding-candidate pruning, artifact deletion, and safe-deletion overclaim fixtures.

## Phase 5: Choregraph adapter

- [x] [depends:campaign.choregraph_dependency] r[campaign.history_projection] Implement the one-way adapter from bounded Choregraph history projections to Campaign projection values.
- [x] [depends:campaign.selection] [depends:campaign.history_projection] r[campaign.publication_fence] Map Campaign decisions to immutable Choregraph control events and generation-fenced branch-reference plans.
- [x] [depends:campaign.history_projection] r[campaign.cache] Implement derived cache validation against exact Choregraph projection and Campaign identities.
- [x] [depends:campaign.history_projection] r[campaign.replay_boundary] Add source checks that reject replay, lineage selection, merge, and Universal Weave authority from the Campaign core, adapter, and conformance crates (`campaign-profile` sources are not identifier-scanned; see `risk.md`).
- [x] [parallel] r[campaign.publication_fence.stale] Add stale generation, stale head, duplicate event, conflicting event, missing publication, and execution-before-publication fixtures.

## Phase 6: Configuration, receipts, and conformance

- [x] [depends:campaign.identity] r[campaign.receipts] Implement bounded decision and state-projection receipts with explicit non-claims.
- [x] [serial] r[campaign.configuration] Implement typed Nickel exploration, guidance, entropy, budget, pruning, adapter, and receipt profiles.
- [x] [depends:campaign.publication_fence] r[campaign.conformance] Publish reusable positive and negative core and Choregraph-adapter suites, including entropy paths.
- [x] [depends:campaign.conformance] r[campaign.conformance.state_coverage] Present bounded 4-way state coverage (D97): declared state features and classes, a bounded reachability test over `campaign-core`, canonical recipes replayed through the adapter, and a pinned covering table presented to the load, cache, and claims oracles.
- [x] [parallel] r[campaign.receipts.non_claims] Add exhaustive-search, event-truth, operation-success, replay-success, finding-correctness, authorization, and release overclaim fixtures.

## Phase 7: Consumers and release evidence

- [x] [depends:campaign.conformance] r[campaign.chaoscontrol_migration] Implement a pinned ChaosControl adapter for projections, candidates, guidance, entropy, results, waits, and stops. ChaosControl `b9436098` pins R5 (`e7757a51`) through Cargo and Nix; recorded in `provenance/consumers.ncl` by `ntmtnrtq` (`96948f3f`). See `verification.md`.
- [x] [depends:campaign.chaoscontrol_migration] r[campaign.chaoscontrol_migration.parity] Compare legacy and Campaign-backed policy over the same bounded dynamic-exploration corpus. ChaosControl `b9436098`: parity corpus (26 passed) and the 2026-09-26 KVM smoke parity receipt (0 drifts, `cutover_ready`); recorded in `provenance/consumers.ncl` by `ntmtnrtq`. See `verification.md`.
- [x] [depends:campaign.conformance] r[campaign.consumer_pilots] Make ChaosControl and an Animus-shaped non-VM fixture adapter pass the same conformance suite. Both pilots claim 80 fixtures and digests `28b873e9…`/`37b9ff95…` at R5 in `provenance/consumers.ncl` (`ntmtnrtq`), and the boundary checks admit the record. See `verification.md`.
- [x] [serial] r[campaign.non_vm_candidate_fixture] Record BranchContext source and license boundaries. Build opaque candidate, validator-reference, and workspace-reference fixture inputs.
- [x] [parallel] r[campaign.non_vm_candidate_fixture] Add deterministic selection tests and negative score-as-authority, zero-exit promotion, consumer-type leakage, and foreign-runtime fixtures.
- [x] [serial] r[campaign.chaoscontrol_migration.seed_runner] Confirm that multi-seed execution, progress, resume, aggregation, replay, and minimization remain outside Campaign.
- [x] [serial] r[campaign.choregraph_dependency] Run no-std and Wasm checks, workspace tests, property tests, Clippy, Octet, Nickel, Cairn, conformance, consumer parity, and `nix flake check -L`. Commit `fc05da20` (T0, after the accepted-spec sync): all seven flake checks pass. See `evidence/final-gate.md`.
