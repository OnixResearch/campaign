# Agent Notes

## Product boundary

Campaign owns product-neutral adaptive exploration policy over Choregraph history projections.

The core owns reusable candidates, guidance updates, selection instances, outstanding-work bounds, explicit entropy, budgets, pruning, stop decisions, and receipts.

Choregraph owns immutable history, branch references, lineage projection, replay selection, merge envelopes, and optional Universal Weave topology.

Consumers retain action meaning, execution, scheduling, persistence, clocks, retries, snapshots, payloads, domain findings, evidence, and release authority.

Do not add generic event DAGs, branch-reference mechanics, replay cursors, execution lifecycle, persistence, or Universal Weave wrappers to Campaign.

Do not add ChaosControl faults, assertions, coverage formats, VM snapshots, replay, or minimization to shared crates. Do not add Animus model, prompt, role, or proof semantics.

## Architecture

Use a functional core and imperative shell.

- `campaign-core` owns deterministic frontier policy over explicit values.
- `campaign-choregraph` maps Campaign events and projections to a published, pinned Choregraph history contract.
- Consumer shells own durable branch updates and all action effects.
- `campaign-conformance` owns reusable positive and negative fixtures.

Keep `campaign-core` independent from Choregraph types. Keep framework, provider, VM, database, and product types outside all shared contracts.

Do not depend on an ambient Choregraph worktree. Pin the history contract at `b3e08e19750f53bdbcae970cdf58a47a791ed20b` in Cargo and Nix. Keep the provenance and conformance source identities equal.

## Configuration and identity

Use typed Nickel for human-authored profiles. Runtime Rust consumes checked generated values.

Use BLAKE3 for repository-owned identities and receipts. Preserve another algorithm only when an external contract requires it.

Give every bound, capacity, threshold, schema version, and budget dimension a named owner.

## Rust quality gate

When Rust implementation starts, pin one reviewed immutable `OnixResearch/octet` revision through Nix. Enable its complete lint catalog and official deny-all hook.

Run positive and negative tests across the workspace, all targets, and compatible features. Run Clippy with `-D warnings`. Make `nix flake check -L` enforce the complete gate.

## Lifecycle and version control

Use native Cairn under `.cairn/` for accepted specs, active changes, and archives. Write normative EARS requirements with positive and negative scenarios.

Use `jj` for version control. Do not open pull requests or force-push.
