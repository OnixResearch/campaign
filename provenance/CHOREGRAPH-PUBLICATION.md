# Published Choregraph history contract

## Selected source

Campaign selects the published Choregraph source at this full immutable revision:

- Repository: `rad:zL2ncTUeASVYwcoGkEXv9JKgGbAF`
- Read-only Git adapter: `https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git`
- Revision: `b3e08e19750f53bdbcae970cdf58a47a791ed20b`
- Package: `choregraph-history`, version `0.1.0`
- Canonical event version: `HISTORY_EVENT_VERSION_V1`, value `1`
- Accepted specification: `.cairn/specs/branchable-event-history/spec.md`

Campaign pins this full revision, not a branch or tag. The package does not need a crates.io release for this Git dependency.

## Publication observation

On 2026-09-06, a fresh Radicle fetch reported this revision as Choregraph `main@rad`.
Its ancestors include `1dc26fe0a90336ee6a6774ae1af458f25bd8b36f`, which contains the history implementation.

The following public-source command also fetched the exact revision:

```text
nix flake metadata 'git+https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git?rev=b3e08e19750f53bdbcae970cdf58a47a791ed20b' --no-write-lock-file
```

The result named the requested revision and supplied a Nix store source.
This observation does not establish continuous availability or consumer conformance.

`CHOREGRAPH-BLOCKER.md` remains a historical observation. This record supersedes its publication prerequisite.
A full published Git revision is an immutable source selection. A new tag does not strengthen that selection.

## Consumed contract

The adapter consumes only these published history capabilities:

- `HistoryGraph` and `EventDraft`: immutable application-event admission.
- `EventId` and `Digest`: exact source identities.
- `Projection` and `ProjectionKind::LinearLineage`: caller-selected bounded control history.
- `BranchRef` and `ReferenceMutationPlan`: explicit head and generation facts.
- `plan_branch_advance`: pure generation-fenced update plans.

Relevant requirement families are `choregraph.history.event_dag`, `choregraph.history.branch_refs`, `choregraph.history.lineage`, and `choregraph.history.boundary`.
Campaign declares its own payload schema identity. Choregraph binds that opaque identity and does not interpret Campaign policy.

Campaign does not import the machine-history or Universal Weave adapters. It does not own lineage selection, graph persistence, replay, or effect authority.

## Owner-authorized reuse

The user authorized completion of Campaign. The workspace owner grant permits reuse of exclusively OnixResearch-owned source across consumer license boundaries.
Campaign consumes the selected Choregraph contract under that separate owner grant. Choregraph retains its public `AGPL-3.0-or-later` license and notices.
The grant does not change that public license or apply to other recipients.

The consumed package declares BLAKE3 as its external dependency. BLAKE3 and all other third-party material retain their licenses and notices.
No Choregraph source is copied into Campaign or read from an ambient sibling at runtime.

## Remaining acceptance

Published source availability clears only dependency selection. Cargo, Nix, provenance, and adapter conformance must select the same revision.
Positive and negative Campaign checks must establish adapter behavior before stable publication.
