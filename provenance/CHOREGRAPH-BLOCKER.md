# Choregraph branchable-history publication blocker

This record satisfies the Phase 1 task `r[campaign.choregraph_dependency.blocked]`: it records the current state of the Choregraph branchable-history publication blocker, the exact publication evidence required to clear it, and the affected work that stays blocked. All facts were verified on 2026-09-04 by read-only git inspection of the workspace sibling `../choregraph/` (object reads, branch containment, worktree listing, and manifest reads at the named revisions). No sibling repository, ref, or worktree was modified.

## What changed since the design was written

The design revision cited by this repository's design and README, `dc986f60ed99866df5509aff2d1ac0bec12fa940` ("Keep branchable history on Choregraph's graph boundary", 2026-08-10), was abandoned. It is reachable from no branch; the sibling clone retains it only under a `jj` keep ref. It remains design provenance only, exactly as `provenance/SOURCE.md` records.

The capability landed through reworked commits instead:

| Commit | Date | Subject | Reachability |
|---|---|---|---|
| `efa4e705d782c1f0fc1e96b789a9b47010e45401` | 2026-08-13 | Keep branchable history on Choregraph's graph boundary | Choregraph `main` and `rad/main` |
| `1dc26fe0a90336ee6a6774ae1af458f25bd8b36f` | 2026-08-13 | Preserve runtime authority while adding branchable history | Choregraph `main` and `rad/main`; sibling checkout HEAD |
| `b3e08e19750f53bdbcae970cdf58a47a791ed20b` | 2026-08-20 | prove readiness completeness before parallel dispatch | current Choregraph `main` and `rad/main` tip |

Choregraph accepted the capability in its own lifecycle as archive `2026-08-13-add-branchable-event-history`, with the accepted contract at `.cairn/specs/branchable-event-history/spec.md` (the `r[choregraph.history.*]` requirements). The pure contract crates at `1dc26fe` are `choregraph-history` (DAG admission, identity, model, branch references, traversal), `choregraph-history-machine`, and `choregraph-weave`, all at workspace version `0.1.0`. `git diff --stat 1dc26fe b3e08e1 -- crates/choregraph-history crates/choregraph-history-machine crates/choregraph-weave` is empty, so the contract surface is unchanged through the current mainline tip.

The ambient planning worktree `choregraph-wt-branchable-history/` referenced by this repository's AGENTS.md no longer exists; the sibling now has only its main checkout (detached at `1dc26fe`) and an unrelated `choregraph-suspension` worktree.

## Why the dependency stays blocked

`r[campaign.choregraph_dependency]` requires Campaign to consume one published immutable Choregraph history revision, and `r[campaign.choregraph_dependency.moving]` fails stable publication for branch, unresolved-tag, or sibling-path selections. Today:

- The contract crates are not published. The Choregraph workspace declares `publish = false`, and the repository has no git tags; the only crates.io citation in the contract docs is the external `universal-weave` `0.4.1` cohort that Choregraph itself consumes. The contract exists only through reachability from the moving `main`/`rad/main` branches.
- No revision has been designated as the published contract revision for consumers, and no single consumer-facing history schema identity has been declared outside the per-event `version` and `schema identity` fields bound by `r[choregraph.history.event_dag]`.
- Campaign has no Cargo, Nix, or conformance declarations yet; the workspace lands with `r[campaign.functional_core]` (Phase 2), so there is nothing to validate a pin against (`r[campaign.choregraph_dependency.valid]`).
- The implementation-stage license re-evaluation required by `provenance/SOURCE.md` before any import from AGPL-3.0 sources is outstanding: depending on the AGPL-3.0 Choregraph crates from Campaign's `MIT OR Apache-2.0` crates is a dependency decision that must be recorded before code links them.

## Evidence required to clear the blocker

1. One exact immutable Choregraph revision, identified by full commit hash and reachable from the published `rad/main` line, designated as the history-contract revision — durably named (for example by a Choregraph tag) so that Campaign's selection is not a moving branch or unresolved tag.
2. The consumed contract surface enumerated as accepted requirement IDs (`r[choregraph.history.*]`) together with the crate versions and the event `version` and `schema identity` values at that revision.
3. Campaign-side declarations pinning that same full revision in Cargo and Nix, an updated `provenance/SOURCE.md` row moving Choregraph from design provenance to pinned dependency, and conformance fixtures bound to the same revision — validated by `r[campaign.choregraph_dependency.valid]`.
4. The recorded license-boundary re-evaluation from `provenance/SOURCE.md` covering dependency on the AGPL-3.0 Choregraph crates.

## Work that stays blocked

- `r[campaign.choregraph_dependency]` — pin the published Choregraph history revision through Cargo and Nix (explicitly sequenced after publication).
- `r[campaign.boundary]` — confirming the exact Choregraph event, branch-reference, projection, lineage, and receipt contracts before adapter implementation.
- Phase 5 adapter tasks: `r[campaign.history_projection]`, `r[campaign.publication_fence]` with the `r[campaign.publication_fence.stale]` fixtures, `r[campaign.cache]`, and `r[campaign.replay_boundary]`.
- Stable publication of Campaign.

All remaining work stays sequenced by `tasks.md`; nothing in this record unblocks or reorders other phases.
