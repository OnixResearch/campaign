# Campaign

Campaign is a product-neutral Rust component for bounded adaptive exploration over Choregraph history.

Its pure core owns frontier policy, candidate selection, guidance binding, resource budgets, pruning plans, stop decisions, and structural receipts. Choregraph owns immutable event history, branch references, lineage projection, replay selection, and optional Universal Weave topology.

## Status

This repository is incubating. The core, history adapter, conformance port, profile loader, and repository gates are implemented under the Cairn changes `build-campaign-core` and `frontier-operational-discipline`, archived on 2026-09-26 under `.cairn/archive/`; their requirements are accepted in `.cairn/specs/campaign/spec.md` and `.cairn/specs/frontier-operational-discipline/spec.md`. Independent adversarial audit rounds attack the integrated head. Closeout requires a round with no P1 and no P2 counterexample: P1 and P2 findings are repaired, kept as named regression tests, and audited again, and P3 sampling gaps found in that final clean round are recorded as residual risks with owners. Twelve rounds have run. Each found at least one P1 or P2 finding or candidate, and each was repaired or, where a binding decision accepts it, recorded as a residual risk. Binding decision D99 (user) amends the criterion for round 12, the final round: its three P2 conformance coverage gaps are repaired and verified by regression tests without a further round, and its P3 findings and every dimension the conformance suite does not declare are recorded in `risk.md` with owners. Round 11's three P2 findings were conformance coverage gaps keyed on campaign-state facts. Binding decision D97 repairs them with bounded 4-way state coverage: the negative fixture `history.state_coverage` presents every history shape and receipt kind through a pinned table of campaign states that carries every 4-way combination of declared state features within declared bounds, which brings the shared conformance suite to 80 fixtures. Round 12 found that the history presented against those states was still fixed (forks at most one event ahead, fork heads of six kinds) and that planner-stop plan receipts were missing from consumer- and policy-stopped states; D99 declares the fork depth and the fork head's event sub-kind, pairs each with every 3-way combination of held features, and presents planner-stop plan receipts at states of every class. From round 11 on, each round is a specification-compliance and coverage review: P1 and P2 findings need an executed test, and the infra checker is reviewed only for ordinary repository changes. The evidence record is [`.cairn/archive/2026-09-26-build-campaign-core/verification.md`](.cairn/archive/2026-09-26-build-campaign-core/verification.md).

ChaosControl adoption is complete on a ChaosControl branch (commit `b9436098`, pinning Campaign `e7757a51`). It supplies the second consumer pilot, legacy-policy parity, and a Campaign-backed KVM smoke. That Campaign pin is a development pin, and its fresh acquisition from the live forge endpoint was not observed.

The published Choregraph history revision is `b3e08e19750f53bdbcae970cdf58a47a791ed20b`. See [`provenance/CHOREGRAPH-PUBLICATION.md`](provenance/CHOREGRAPH-PUBLICATION.md) for the source and owner-grant boundaries. Every adopted source is recorded in the typed inventory [`provenance/sources.ncl`](provenance/sources.ncl).

Campaign's final `nix flake check -L` passed at `fc05da20`, after the accepted-spec sync. Both lifecycle changes are archived (2026-09-26). Consumer pilot evidence is recorded in the typed record [`provenance/consumers.ncl`](provenance/consumers.ncl). The boundary checks require every pilot's claims to equal the fixture count and transcript digests pinned in `crates/campaign-conformance/src/tests/consumers/mod.rs`, and count those pins only while the library's `#[cfg(test)] mod tests;` and `mod consumers;` register a top-level `#[test]` that is not ignored, expected to panic, or configured away, names all three constants in its body, and calls `run_suite` in its body or through a same-module function its body calls, and while no inner `#![cfg]` configures away the library root, the tests module, or the pinned module. The ChaosControl entry records ChaosControl's final adoption at Campaign R5, and the boundary checks admit it. Nothing in this repository claims promotion or release readiness.

The [Octet acceptance record](provenance/OCTET-ACCEPTANCE.md) reports zero findings across the full workspace, all targets, and all features.

## Scope

Campaign owns:

- campaign, policy, guidance, candidate, decision, budget, and adapter identities;
- admission from bounded Choregraph history projections without a complete future work inventory;
- deterministic frontier reconstruction from Campaign events in Choregraph history;
- reusable candidate ranking, selection counts, and unique selection instances;
- explicit bounded entropy for deterministic exploratory selection;
- checked exploration budgets, outstanding-selection bounds, and consumer resource counters;
- bounded finding references, retention decisions, pruning plans, and stop reasons;
- BLAKE3 decision and state-projection receipts;
- reusable positive and negative conformance fixtures.

Choregraph owns:

- immutable event DAGs and canonical history identities;
- generation-fenced branch references;
- ancestor and explicit-lineage projections;
- machine replay through the production kernel;
- merge envelopes and optional Universal Weave topology.

Consumers retain:

- action generation and domain meaning;
- execution, scheduling, placement, retries, clocks, persistence, and artifact storage;
- snapshots, payload bytes, action transcripts, result truth, and infrastructure retries;
- coverage, novelty, assertion, finding, replay, and minimization semantics;
- authorization, cleanup, evidence, publication, and release decisions.

## Architecture

The implementation uses these crates:

- `campaign-core`: a `no_std + alloc` exploration-policy core;
- `campaign-choregraph`: a one-way adapter to a published Choregraph history contract;
- `campaign-conformance`: the reusable conformance port. A consumer implements `ConformanceAdapter` (identity, record, and shell mapping) and runs `run_suite` against real Choregraph history; the report names one verdict per positive and negative fixture and carries an adapter-independent `DecisionTranscript` whose BLAKE3 digest two consumers compare. The crate also ships the Animus-shaped non-VM fixture adapter, a float-scoring adapter, and a readable reference planner reachable only through `campaign_conformance::reference`;
- `campaign-profile`: a strict loader that turns the exported `campaign.profile.v2` Nickel profile into checked core values (bounds, selection, stopping, retention, and admission rules).

`tools/campaign-infra-checker` enforces dependency boundaries, the source inventory, immutable Choregraph pins, and source-pattern rules for ambient effects, replay authority, finding semantics, foreign runtime types, and seed-runner authority (seed dispatch, progress, resume, aggregation, checkpoints). It also refuses compile-time file and environment reads and source indirection in any token tree (macro bodies, macro arguments, attributes, renamed imports), any `.cargo/config` or `.cargo/config.toml` in the repository and any toolchain-file `path`, features any member sets on a core-allowlisted dependency, and compiled targets outside their member. It exempts a module from the history, consumer, and replay scans only when every declaration naming it is gated by exactly `#[cfg(test)]`. The checker is a lexical guard over ordinary repository changes, not a proof against deliberate evasion, and Cargo configuration outside the repository (ancestor directories, `CARGO_HOME`, `--config`, the environment) is not checked.

Hosts that keep a live frontier advance it with `campaign_core::apply_control_event`, the exact per-event step of `reconstruct`.

`campaign_choregraph::validate_control_projection` is the cold-start entry only. A host that holds a validated projection advances it with `validate_control_advance` and must pass it to publication (`ControlPublicationRequest::previous`) and to cache validation (`CacheValidationRequest::previous`), which reject rewinds, renames, fork jumps, foreign envelope identities, and a graph older than the held projection's (`StaleGraph`). `plan_next_from_published_projection` also respects the host's projection bound, the branch generation range, and the Choregraph graph's event limit. When no room is left for another selection, its settlement, and the stop, it waits for outstanding results. With nothing outstanding, it plans a consumer stop under the adapter-owned `control_capacity_reason_id()`. Once the recorded reserve no longer fits, every plan returns the typed capacity error. Other writers of the graph must leave that reserve free. It waits for guidance only while the guidance, a selection, its settlement, and the stop still fit. The reserve accounts for one campaign per graph and envelope: `admit_control_root` refuses a root on a graph that holds another Campaign control event under its envelope, and validation refuses a graph holding a second control root, both with `SharedControlGraph`, so hosts give each campaign its own graph. `admit_control_root` admits a root only with room for its own stop, and retrying an identical root that the graph already holds returns the same admission while the graph holds no other control event under the envelope. A publication whose draft is admitted but whose branch update has not committed leaves a pending draft on the branch head. Planning and every other publication on that head are then refused with `PendingControlDraft`, so a stale publisher or a compare-and-set loser cannot consume the winner's reserve. The host decodes the draft with `pending_control_draft` and republishes its control event, which yields the same publication and counts the draft once. Hosts serialize graph appends per control branch.

Hosts publish a control event only on a durably committed branch value: they never plan or publish through a publication's planned branch value before its compare-and-set is durable. A validated projection counts every pending draft that descends from the branch head as a selection and its settlement, and it exposes the room that other writers may still append as `ValidatedControlProjection::graph_room`. Hosts republish pending drafts before they append anything else, leave that room free, and re-validate after appending. The recovery guarantee covers the drafts such hosts leave, a partial commit and racing siblings; a host that pipelines publications below an uncommitted draft is counted conservatively and may be refused at the bound, which can block its branch head. A draft that the host cannot complete blocks its branch head, and no abandonment API exists.

Human-authored profiles live in `config/` as typed Nickel. `scripts/check-config.sh` checks their contracts, negative fixtures, and export freshness, including the copy embedded in `campaign-profile`.

## Decision flow

```text
consumer observations, candidates, and entropy
  -> campaign-choregraph projection
  -> campaign-core frontier decision
  -> Choregraph event and branch-reference plan
  -> consumer-owned durable publication
  -> consumer-owned action execution
```

A consumer must not execute a selected operation until the exact decision event and branch-reference change are durably accepted.

## Claim boundary

A selection decision does not prove that an action started or completed. A finding reference does not prove a product defect.

A Campaign receipt records bounded supplied facts. It does not prove exhaustive search, event truth, replay success, authorization, or release readiness.

## Building and checking

The Rust workspace and two repository gates run without credentials, a sibling checkout, or the Nix dev shell.

[rustup](https://rustup.rs/) installs the toolchain that `rust-toolchain.toml` pins: Rust 1.96.0 with Clippy, rustfmt, and the `wasm32-unknown-unknown` target. Cargo fetches registry crates from crates.io and the pinned `choregraph-history` revision from the read-only Git route that `Cargo.toml` names.

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo check -p campaign-core --no-default-features --target wasm32-unknown-unknown --locked
cargo check -p campaign-choregraph --no-default-features --target wasm32-unknown-unknown --locked
cargo check -p campaign-conformance --no-default-features --target wasm32-unknown-unknown --locked
```

The conformance suite can take about ten minutes in a debug build.

The configuration and boundary gates compare `nickel export` output byte for byte. They therefore need Nickel 1.17.0 from the repository's own nixpkgs input in `flake.lock`. This command needs `jq` and Nix with flakes enabled:

```sh
nix shell "github:NixOS/nixpkgs/$(jq -r '.nodes[.nodes.root.inputs.nixpkgs].locked.rev' flake.lock)#nickel" \
  -c sh -c 'sh scripts/check-config.sh && sh scripts/check-boundaries.sh'
```

If `CHOREGRAPH_HISTORY_SOURCE` is set, `scripts/check-boundaries.sh` also checks that it contains the `choregraph-history` package manifest. It does not verify the checkout's revision.

The complete gate is `nix flake check -L`. It adds the Cairn lifecycle checks and the Octet deny-all lint catalog. The gate, the dev shell (`nix develop`), and `scripts/check.sh` need both the pinned Cairn CLI and the Octet hook. The pre-commit hook and `scripts/check-rust-gates.sh` need the Octet hook, and `scripts/check-cairn.sh` needs the Cairn CLI. Together, these entry points need read access to inputs that refuse anonymous fetches:

- private GitHub repositories `OnixResearch/cairn` (the `cairn` flake input and three nested revisions), `OnixResearch/octet` (the `octet` flake input and the pre-commit hook), `OnixResearch/slotcar`, and `OnixResearch/valence`;
- SSH-only Git sources `OnixResearch/onix-artifact` and `OnixResearch/mantle`, selected in `flake.lock` and in the pinned Cairn `Cargo.lock`. `OnixResearch/mantle` is readable over HTTPS, but these lockfiles select it through SSH.
- the SSH-only Git source `OnixResearch/bounded-exec` at `44eb74eb6391`, which only the pinned Octet `Cargo.lock` (Octet `890f424cd7b1`) selects. The `bounded-exec` node in `flake.lock` is a separate public route.

`flake.lock` records the flake-input revisions; the pinned Cairn and Octet `Cargo.lock` files record their Cargo Git sources. The public [`cairn-public`](https://github.com/OnixResearch/cairn-public) and [`octet-public`](https://github.com/OnixResearch/octet-public) source exports do not contain the pinned revisions. Passing the credential-free checks does not mean the complete gate passes.

## Source

The canonical repository is the Radicle project `rad:z2scC9MCm3pxk9mX4FEidRKabQ5LN`. Public Radicle seeds serve read-only clones:

```sh
git clone https://seed.radicle.garden/z2scC9MCm3pxk9mX4FEidRKabQ5LN.git campaign
```

The `repository` field in `Cargo.toml` and ChaosControl's Campaign pins name the `git.onix.computer` adapter for this RID. That adapter does not currently serve anonymous clones, so use a seed route such as the one above.

## License

Campaign is licensed under [`AGPL-3.0-or-later`](LICENSE). [`NOTICE`](NOTICE) states the grant and the material it does not relicense: the pinned Choregraph dependency, the copied Cairn policy files, and registry crates.

Earlier revisions were offered under `MIT OR Apache-2.0`. [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE) preserve those texts. They cover only content already granted under them, not new contributions.

## References

- [Choregraph](https://github.com/OnixResearch/choregraph/tree/b3e08e19750f53bdbcae970cdf58a47a791ed20b) at `b3e08e19750f53bdbcae970cdf58a47a791ed20b` — pinned immutable history and branch-plan contract. Cargo and Nix fetch it from `https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git`.
- Choregraph branchable-history design at `dc986f60ed99866df5509aff2d1ac0bec12fa940` — planned immutable history and optional Universal Weave ownership.
- [ChaosControl](https://github.com/brittonr/chaoscontrol) at `e1dcf2552132221085967250cbc5cb00559e8757` — adaptive frontier, coverage guidance, input-tree branching, and first consumer source.
- Animus (`OnixResearch/animus`, not publicly hosted) at `c85fdb35e4e546edcafb368fcb32e0353bf65fc4` — bounded Universal Weave adapter and consumer-owned session meaning.
- [Lattice](https://github.com/OnixResearch/lattice) at `9004439b6973d36ddf34475d6b9f6735d34a0857` — durable orchestration and consumer-owned effect authority.
- VM Cohort (`rad:z2QJLUqyAZnnHPiZQ1BFjLsX9ush3`) at `ac279d14afa586521cfe2f5a2b229416d2de7ce9` — VM worker mechanics that remain outside Campaign.
- Transactional Reconciliation Core (`rad:z4Tky6zvC8w4Y6c4YBzNxVbq5n752`, clone `https://seed.radicle.garden/z4Tky6zvC8w4Y6c4YBzNxVbq5n752.git`) at `ff8e41f076805a76f5eeef6fd7e4a4cdec76bfd2` — uncertain-effect and compare-and-set reference.
- [Luv frontier study](https://github.com/mbrock/luv/blob/638390086d236a383acd8f9eb98dea452d91ea2c/wiki/frontier-traversals.org) at `638390086d236a383acd8f9eb98dea452d91ea2c` — reference for explicit frontier roles and separate evidence questions. Campaign implements its own normative contracts and does not copy this source.
- [BranchContext](https://github.com/multikernel/branching/tree/b0778564fbc75eb82a61687d211ee6050f9750bf) — Apache-2.0 exploration-pattern reference for the non-VM conformance fixture. Campaign imports no Python runtime, workspace effects, or automatic acceptance policy.
