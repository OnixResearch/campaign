# ChaosControl exploration-policy baseline

This record pins the focused ChaosControl frontier, coverage-priority, input-tree, pruning, budget, and stop-policy behavior that the future Campaign-backed adapter must preserve for parity (`r[campaign.chaoscontrol_migration.parity]`, Phase 1 baseline task). It is behavior description only: `chaoscontrol-explore` is AGPL-3.0-or-later, so no source text is copied into Campaign; the adopted-mechanism row and license boundary live in [`SOURCE.md`](SOURCE.md).

## Reviewed revision

| Field | Value |
|---|---|
| Repository | `github.com/brittonr/chaoscontrol`; workspace sibling `../chaoscontrol/` |
| Reviewed revision | `e1dcf2552132221085967250cbc5cb00559e8757` (2026-08-10, "Restore immutable SpaceWasm cohort admission") |
| Crate | `crates/chaoscontrol-explore` ("Coverage-guided exploration engine for ChaosControl", AGPL-3.0-or-later per its `Cargo.toml`) |
| Read method | `git show <revision>:<path>` at the exact revision; no moving ref was consulted |

Line anchors below are line numbers in the file at that revision.

## Frontier structure and selection

Source: `crates/chaoscontrol-explore/src/frontier.rs`.

- A frontier entry binds a monotonic `u64` id, a VM `SimulationSnapshot`, a `CoverageBitmap`, an `f64` score (higher is more interesting), `times_selected: u32`, `depth: u32`, the `FaultSchedule` that produced the state, and an optional parent entry id (`frontier.rs:10`).
- `push` assigns the next monotonic id, inserts, re-sorts the whole vector by score descending, and prunes when over capacity (`frontier.rs:49`).
- Selection is epsilon-greedy with an ambient RNG: with probability `0.1` pick a uniform random entry, otherwise pick the best effective score `score / (1 + times_selected)` (`frontier.rs:77`, `frontier.rs:85`). Selection increments `times_selected` and keeps the entry, so one entry can be selected repeatedly.
- Entries stay selectable until pruned; there is no retirement state, and the parent link is plain `Option<u64>` without generation fencing.

Parity dimensions to preserve: reusable candidates, selection-count decay of priority, monotonic entry identities, parent/depth tracking.

Deliberate Campaign divergences (already normative in the change spec): the ambient RNG draw becomes an explicit bounded entropy ticket under a versioned selection profile (`r[campaign.entropy]`), and `f64` scores become bounded integer ranks converted at the adapter boundary (`r[campaign.guidance]`).

## Coverage guidance and priority scoring

Source: `crates/chaoscontrol-explore/src/coverage.rs` and `score_branch` in `crates/chaoscontrol-explore/src/explorer.rs:1273`.

- Coverage is an AFL-style 64 KiB bitmap of 8-bit saturating counters (`coverage.rs:9`). Region layout: code edges `[0, 32 KiB)`, assertion-state edges `[32 KiB, 48 KiB)`, schedule-fingerprint edges `[48 KiB, 64 KiB)` (`coverage.rs:17`).
- Hit counts are bucketed AFL-style (1, 2, 3, 4-7, 8-15, 16-31, 32-127, 128+) by `classify` (`coverage.rs:76`). Global coverage is a per-position maximum merge; new coverage means any position nonzero locally and zero globally (`coverage.rs:66`).
- Branch score (`explorer.rs:1273`): `10 × new_edges + rare_edge_weight × rare_edges + 0.1 × total_edges − 0.5 × parent_depth`, floored at `0.1`, plus `+1` per assertion in the oracle catalog. An edge is rare when its global hit count is nonzero and `≤ rare_edge_threshold` (default `3`); `rare_edge_weight` defaults to `5.0` (`explorer.rs:186-187`).
- Blind mode: a coverage guest address of `0` makes every run score only through assertion counts and depth, with an empty bitmap.

Parity dimensions to preserve: new-edge dominance, rare-edge boost, depth penalty, assertion-count bonus, and the "rare means globally infrequent" definition. The bitmap, buckets, and region layout stay ChaosControl-owned evidence; the adapter only supplies derived integer ranks.

## Input-tree branching

Source: `crates/chaoscontrol-explore/src/input_tree.rs`.

- Exploration can branch at guest `random_choice(n)` decision points instead of mutating fault schedules. Modes are `FaultSchedule`, `InputTree`, and `Hybrid` (alternating rounds, shared frontier) (`explorer.rs:44`).
- `select_alternatives` (`input_tree.rs:60`) skips choices with `n_options < 2`, scores each point by `(1 / n_options) × (1 / (1 + sequence_id × 0.01))` — small option counts and early decisions first (`input_tree.rs:77-78`) — sorts descending, and generates alternatives until an explicit budget is reached.
- Small option counts (`n ≤ 10`) enumerate every alternative; larger ones sample up to `3` random alternatives with one retry to avoid the original value (`input_tree.rs:103`, `input_tree.rs:118`). The per-round alternative budget is `branch_factor − 1` via checked `saturating_sub` (`explorer.rs:861`).

Parity dimensions to preserve: small-n priority, depth weighting, bounded alternatives per selection, and enumeration-vs-sampling split. Guest random semantics stay consumer-owned.

## Pruning and stale-search recycling

Source: `frontier.rs` and `recycle_frontier_from_corpus` in `explorer.rs:1455`.

- Over-capacity pruning keeps the top `max_frontier` entries by score and truncates the rest (`frontier.rs:117`); pruning is pure derived-state removal with no history notion at all.
- When the frontier empties, the corpus is recycled: entries covering at least one rare edge are sorted by rare-edge count and up to `max_frontier / 2` (at least 1) are re-added as frontier entries with fresh `times_selected = 0` and a score of `rare_edge_weight × rare_edges + new_edges` (`explorer.rs:1455`).
- Stale-search escalation: after `havoc_after_stale` consecutive stale rounds (defaulting to `stale_round_limit / 2` when unset), rounds switch to havoc mutation ranges `[4, 16]` (`explorer.rs:616-621`, defaults `explorer.rs:188-189`).

Parity dimensions to preserve: prune only derived, un-outstanding work; rank-order retention; recycling as re-admission with reset selection counts rather than history edit. Campaign adds the rule that candidates with outstanding selections are not pruneable (`r[campaign.pruning]`).

## Budgets and bounds

Source: `ExplorerConfig` defaults in `crates/chaoscontrol-explore/src/explorer.rs:168-189` and `CampaignProfile` in `crates/chaoscontrol-explore/src/profile/campaign.rs`.

- Exploration budgets: `branch_factor = 8` branches per snapshot, `ticks_per_branch = 1000`, `max_rounds = 100`, `max_frontier = 50`, scheduling `quantum = 100`, `bootstrap_budget = 10_000` ticks.
- Campaign profile bounds (`profile/campaign.rs:14-15`, `:73`): at most `1024` seeds and `256` workers, plus a `maximum_total_memory_mib` ceiling checked with `checked_mul`; the four mutation probabilities must be finite, in `[0, 1]`, and sum to `1.0` within `f64::EPSILON × 8`; worker count, seed count, and memory must fit the declared bounds before the campaign starts.
- Workers run whole seeds; there is no per-selection cost reservation and no outstanding-work bound, because branches are synchronous within a round.

Parity dimensions to preserve: bounded rounds, bounded frontier, bounded fan-out per selection, and validated profile admission. Campaign replaces implicit round/tick budgets with named checked dimensions (`r[campaign.budgets]`) and adds per-selection cost reservation and the outstanding-selection bound.

## Stop policy

Source: round loop in `crates/chaoscontrol-explore/src/explorer.rs:524-575`.

- Stale-round tracking: a round with zero new coverage edges and zero new bugs is stale (`explorer.rs:526`).
- Stop reasons emitted, in check order: `frontier_exhausted` when the frontier is empty and no corpus entry can be recycled (`explorer.rs:542`); `coverage_plateau` after `stale_round_limit` (default `10`) consecutive stale rounds (`explorer.rs:554`); `bug_found` when a bug appears in a short run (`max_rounds < 10`) (`explorer.rs:561`); `interrupted` on SIGINT/SIGTERM (`explorer.rs:568`); `completed` when `max_rounds` finish without an early break (`explorer.rs:575`).
- A stop ends the process run; there is no wait class, no outstanding-work accounting, and no terminal-stop persistence across restarts. Any stop is implicitly "search ended", but nothing claims exhaustive coverage.

Parity dimension to preserve: the distinct stop classes and the stale-round plateau definition. Campaign maps these onto explicit `FrontierExhausted`, `BudgetExhausted`, `ConsumerRequested`, and wait decisions, and requires that frontier or budget stops are denied while published selections remain outstanding (`r[campaign.wait]`, `r[campaign.stop]`).

## Identity notes observed in the baseline

- Bug deduplication is domain-separated BLAKE3: `"chaoscontrol.bug-dedup.v1\0"` over the assertion fingerprint plus sorted fault type names (`explorer.rs:1305`). Campaign receipts use BLAKE3 with their own domain separation (`r[campaign.receipts]`), so parity must not conflate the two identity spaces.

## Seed-runner boundary confirmation

ChaosControl's multi-seed runner (`crates/chaoscontrol-explore/src/campaign.rs` and `profile/campaign.rs`) owns seed dispatch, worker placement, progress files, resume, aggregation, and report projection. Replay verdicts and minimization stay in ChaosControl's replay and minimizer modules. None of these mechanisms moves into Campaign (`r[campaign.chaoscontrol_migration.seed_runner]`).

Campaign enforces the boundary mechanically:

- The infra checker rejects any ChaosControl, VM, runtime, or storage dependency family in every Campaign crate manifest.
- Its source scans reject replay execution, replay comparison, merge, and minimization identifiers in `campaign-core`, `campaign-choregraph`, and `campaign-conformance`, and seed-runner identifiers (seed dispatch, resume, aggregation, checkpoints) in the same crates.
- `reconstruct` and `apply_control_event` fold Campaign control events into derived policy state. They restore no VM state and resume no execution.

ChaosControl's adoption change records the same boundary from the consumer side (`r[chaoscontrol.campaign_policy.boundary]`): the Campaign adapter replaces frontier policy inside one Explorer run and leaves the multi-seed runner unchanged.

## Non-claims

This baseline records observed source facts at one immutable revision. It does not prove parity, behavior agreement, migration correctness, or that ChaosControl and Campaign decisions match; the comparison itself is the later Phase 7 task over a bounded corpus. It transfers no code, no coverage semantics, no assertion meaning, and no execution or evidence authority; those remain ChaosControl-owned per [`SOURCE.md`](SOURCE.md).

## Verification record

2026-09-04: every cited path and line anchor was read from `../chaoscontrol` at `e1dcf2552132221085967250cbc5cb00559e8757` via `git show <revision>:<path>`, and each anchor was confirmed to contain the quoted symbol or constant at the cited line. The positive check script and its output are recorded in the change's validation evidence for task `r[campaign.chaoscontrol_migration.parity]`. The document contains behavior descriptions and numeric constants only; no ChaosControl source text was copied, keeping the `campaign.source_inventory` planning-stage no-import posture intact for this AGPL-3.0-or-later source.
