# Implementation checkpoint (historical)

> **Historical record.** This checkpoint records the state observed on 2026-09-06, before the Octet repairs and the closeout-audit repairs. Its counts, gaps, and blockers are not current and must not be cited as current evidence.
> The current evidence record is [`.cairn/archive/2026-09-26-build-campaign-core/verification.md`](../.cairn/archive/2026-09-26-build-campaign-core/verification.md).
> See [Octet acceptance](OCTET-ACCEPTANCE.md) for the later verified source checks.

At the time of this checkpoint, Campaign was incubating and neither active Cairn change was complete.
This checkpoint does not authorize stable publication or consumer cutover.

## Outcome and ownership

The implementation preserves the published-history selection loop and its denial controls.
The infrastructure checker now rejects known ways to exclude handwritten source from checks.
Campaign maintains both capabilities in this repository.
Repository tests and the pinned checks provide the repeatable verification path.

## Observed checks

The integrated workspace passed these checks on 2026-09-06:

- Workspace tests across all targets and features: 64 passed, none ignored.
- Workspace Clippy across all targets and features with `-D warnings`: passed.
- `campaign-core` and `campaign-choregraph` checks for `wasm32-unknown-unknown`: passed.
- The infrastructure checker passed the official pinned Octet hook with zero findings, warnings, and errors.

The workspace Octet gate remains red.
Its latest run reported 394 errors in `campaign-core`, across library and library-test compilations.
This result does not establish that the adapter crates are Octet-clean.
The remaining categories include oversized files, concrete imports, assertion density, and mutable borrowed inputs.

## Identity and regression controls

`FrameWriter` consumes and returns an owned incremental BLAKE3 hasher.
It does not retain the complete byte transcript in a growing buffer.
The public append methods now return the writer.
`derive_id_with` accepts and returns this owned writer through its closure.

Two fixed digest controls retain the framing from pre-refactor revision `72fd3b251066a36edbecb5a1c63cae1e50d74d3f`.
They exercise both values of a Boolean field.
The domain, field tags, order, and digest bytes remain unchanged for those controls.
This is bounded compatibility evidence, not a proof for every possible input.
A separate negative control requires a failed field encoder to return its error without an identity.

The published-loop tests still require the current history head and generation.
The adversarial tests still reject crossed policy facts, stale publication, forged state, and incorrect resource reservation.

## Infrastructure controls

The checker now covers:

- Direct, renamed, inherited, and target-specific dependency declarations.
- Empty CLI roots, malformed command bytes, and literal native paths.
- Source traversal bounds, deterministic path order, observed symlinks, and invalid source bytes.
- File-wide and module-wide lint suppressions, including conditional attributes.
- Known excluded Rust paths and `include!` source indirection.
- Every explicit Cargo workspace member, including the checker and integration tests.

These are bounded syntax and filesystem observations.
They do not prove whole-program purity or protection against concurrent filesystem substitution.
The actual repository scan still rejects broad suppressions in the existing core integration tests.
Those tests must retain their assertions during the remaining repairs.

Two item-level Octet allowances remain in the checker:

- `process::capture_request` owns explicit CLI argument and environment capture.
- `rustscope::is_excluded` uses native path methods that are not const on the pinned toolchain.

Each item also permits unknown lint names during plain Rust builds.
A negative compiler control returned `E0015` for `Path::components` in a const function.
No crate-wide or module-wide Octet allowance is present in the checker.

## Nix and lifecycle

The Nix checks now use the Cargo lockfile vendor directory in offline Cargo mode.
A frozen-source Cairn check compiled the checker, verified policy freshness, and validated both active changes.
It then failed traceability coverage because the accepted requirement set is empty.
No spec sync or archive was performed to hide this failure.

The full Nix check ran against frozen revision `c1b29da120bf9cf5cacbb0b849b918ea567d0754` with `--keep-going`.
Its Nickel, formatting, and Rust workspace checks passed, including 63 tests, Clippy, and Wasm checks.
Its Octet, dependency-boundary, local-checker, and Cairn-policy checks failed on the recorded blockers.
The subsequent field-encoder negative control increased the local test total to 64.

During concurrent source edits, a live-path Nix evaluation rejected a changed source hash.
The subsequent check used a frozen Jujutsu snapshot through a Git archive.
Jujutsu has no equivalent archive command in the installed version.

## Remaining acceptance work

- Complete real source repairs for the full workspace Octet gate.
- Remove broad test suppressions without weakening the test assertions.
- Complete requirement-linked verification, accepted-spec sync, and lifecycle gates.
- Make generated configuration values feed the real consumer policy mapping.
- Implement the ChaosControl adapter and run the shared conformance cases through it.
- Establish policy parity, including selection-count decay after repeated selections.
- Run a Campaign-backed KVM smoke and preserve its bounded evidence.
- Run the complete Nix checks, then complete publication and Cairn archive steps.

The legacy ChaosControl smoke is not Campaign adoption evidence.
The legacy division-based priority rule differs from the current subtractive Campaign decay rule.
Stable publication remains withheld until the consumer and repository acceptance work is complete.
