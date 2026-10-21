# Campaign Octet acceptance

## Scope and result

The official pinned hook passed for the complete workspace, all targets, and all features on 2026-09-06.
It reported zero findings, warnings, errors, and autofixable findings.
The check includes the core, history adapter, conformance library, infrastructure checker, and integration tests.

Octet revision: `890f424cd7b1b2015f39c0eb5747f1bd1e576028`.
Driver toolchain: `nightly-2026-03-21-x86_64-unknown-linux-gnu`.
Rust toolchain: `1.96.0`.

```text
Status: clean
Findings: 0
Warnings: 0
Errors: 0
Autofixable: 0
```

The final local run recorded these identities:

- Config: `b3:41533bc1ba5c6e3d9d26c75863daf20ab6df4285c9954ecf8addc4002f390316`.
- Profile: `b3:c8b6227f7433b0ba2889f34f57a6f16cdc52b74feb1346b4107ff3400451177d`.

## Repeatable checks

Run these commands from the Campaign repository:

```sh
nix develop path:$PWD -c sh -c '"$OCTET_DENY_ALL_HOOK" --workspace -- --all-targets --all-features'
nix develop path:$PWD -c cargo test --workspace --all-targets --all-features --locked
nix develop path:$PWD -c cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
nix develop path:$PWD -c cargo check -p campaign-core -p campaign-choregraph --target wasm32-unknown-unknown --no-default-features --locked
nix develop path:$PWD -c cargo run -p campaign-infra-checker --locked -- boundaries --root .
```

The regression run passed 83 tests, with none ignored.
All 27 original integration scenarios remain present and pass.
The Clippy, Wasm, formatting, and actual-source boundary checks also passed.

New controls cover fixed identity codes, ordering, missing and crossed identities, length bounds, receipt claims, framing errors, and history metadata.
The two fixed framing digests remain unchanged.
The test fixtures now return explicit errors instead of relying on file-wide panic or unwrap allowances.

## Durable source repairs

Campaign owns these changes and their maintenance:

- Capability modules replace the oversized model, frontier, policy, and fixture files.
- Qualified paths expose concrete type and function owners.
- Internal frontier and fixture transitions consume owned state and return the next state.
- Assertions check validated identities, canonical order, counts, budgets, and publication relationships.
- Checked arithmetic retains typed failure paths.
- Fixed identity discriminants retain the original wire codes.

The full lint catalog and package scope remain unchanged.
No generated-source exclusion, disabled lint, finding baseline, ignored test, or module-wide suppression contributes to this result.
The source-boundary checker also rejects file-wide and module-wide suppressions in integration tests.

## Frozen Nix verification

The full Nix check ran with `--keep-going` against frozen revision `cb4d2823d3fb4524eb3f5b39f2cfd19aed60855f`.

These checks passed:

- `octet-deny-all`: zero findings, warnings, and errors.
- `rust-workspace`: 83 tests, Clippy, and Wasm checks.
- `dependency-boundaries`: fixture tests and the actual source scan.
- `nickel-config`: configuration contracts and export freshness.
- `cargo-format`: formatting.

`cairn-policy` and `local-checker` failed traceability coverage because the accepted requirement set remains empty.
Thus, the full flake remains red on lifecycle acceptance, not on Octet.
No lifecycle requirement was removed to change that result.

## Narrow exceptions

The result includes these item-level Octet exceptions.
Each source attribute names its owner and specific reason.
No exception applies to a complete module or crate.

| Owner and item | Lint | Reason |
|---|---|---|
| MODEL: `FrameWriter::new` | `concrete_construction_in_core` | BLAKE3 construction is deterministic local computation, not an external capability. |
| MODEL: `derive_state_id_from_parts` | `too_many_parameters` | Preserve the existing seven-part public signature and ordered transcript. |
| MODEL: `check_len` | `usize_in_public_api` | Accept native collection lengths with checked conversion to protocol `u64`. |
| MODEL: `check_len_at_least_one` | `usize_in_public_api` | Preserve native-length admission and its typed empty-input error. |
| POLICY: `FrontierState::recompute_identity` | `mutating_input_in_pure` | Preserve the public wrapper while an owned internal transition computes the replacement. |
| POLICY: `FrontierState::sort_all` | `mutating_input_in_pure` | Preserve the public wrapper while an owned internal transition sorts the replacement. |
| POLICY: `role_code` | `fragile_exhaustive_enum_match` | A new role requires a distinct, explicitly reviewed wire code. |
| POLICY: `ensure_candidate_selectable` | `fragile_exhaustive_enum_match` | Each closed candidate status requires its exact typed denial. |
| ADAPTER: `envelope::payload::encode_body` | `missing_const_fn` | The frame writer updates a runtime BLAKE3 transcript and cannot run as a const function. |
| Checker: `capture_request` | `ambient_env` | The CLI shell owns argument and environment capture. |
| Checker: `is_excluded` | `missing_const_fn` | The pinned compiler rejects native path iteration in a const function. |

Existing item-level Clippy exceptions preserve consumed public requests and independent receipt claim flags.
The checker permits unknown Octet names only on its two documented items during plain Rust builds.

Library attributes register the `tigerstyle` tool only under `dylint_lib = "octet"`.
Cargo denies unexpected configuration values and declares this one selector.
Compiler controls accept the selector and reject an unreviewed library name.
Tool registration does not suppress a lint or exclude source.

## Claim boundary

This result establishes the listed source and regression checks under the pinned toolchains.
It does not establish exhaustive correctness, consumer adoption, release readiness, or lifecycle completion.
Both Campaign changes remain active.
Real ChaosControl adoption and accepted-spec traceability remain separate acceptance work.
