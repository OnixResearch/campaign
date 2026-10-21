# Final gate for build-campaign-core

This record covers Campaign's final `nix flake check -L`, which the final-gate task in `tasks.md` requires.

## Revision

T0 is change `vsorlzln`, commit `fc05da20b8c019d96141a26716815c96e3af001d`. It consists of:

- change `vsukstxw` (`3aaa8b2c`), which records the approved assurance plan review of `build-campaign-core`;
- the accepted-spec sync of both changes. `vsorlzln` copies the delta specs of `build-campaign-core` and `frontier-operational-discipline` into `.cairn/specs/campaign/spec.md` and `.cairn/specs/frontier-operational-discipline/spec.md`.

T0 has the same code as R5 (`e7757a51`). Between R5 and T0, only files under `.cairn/`, `README.md`, `provenance/consumers.ncl`, and `provenance/generated/consumers.json` changed.

## Command

```sh
nix flake check -L --keep-going --option builders '' path:/home/brittonr/git/OnixResearch/.worktrees/campaign-complete-20260906
```

The command ran in a workspace whose working copy was an empty change on T0. It exited 0, and the log ends with `all checks passed!`.

## Per-check results

Seven checks were built for `x86_64-linux`. Each passed:

| Check | Result in the log |
| --- | --- |
| `cairn-policy` | `cairn validate` reports `"valid": true`. |
| `cargo-format` | `cargo fmt --all -- --check` passes. |
| `dependency-boundaries` | The 98 infra-checker tests pass. The checker prints "campaign provenance, dependency, and boundary checks passed". |
| `local-checker` | `scripts/check.sh` passes. Configuration: 6 positive and 42 negative fixtures. The boundaries check passes. `cairn validate` reports `"valid": true`. The Rust gates pass (670 tests, conformance library 165 in 782.77 s). Octet reports `Status: clean` and `Findings: 0`. The script prints "campaign infrastructure checks passed". |
| `nickel-config` | Configuration contracts and freshness pass: 6 positive and 42 negative fixtures. |
| `octet-deny-all` | `Status: clean`, `Findings: 0`. |
| `rust-workspace` | 670 tests passed and 0 failed: core 234, core unit 35, properties 10, adversarial 5, choregraph repair 87, choregraph unit 8, choregraph published 2, conformance 165 (472.12 s), profile 26, and infra checker 98. The post-check Clippy (`-D warnings`) and the `wasm32-unknown-unknown` checks of `campaign-core`, `campaign-choregraph`, and `campaign-conformance` pass. |

`aarch64-linux` was not checked. Nix reports "The check omitted these incompatible systems: aarch64-linux", because this host cannot check that system.

The log also contains these messages. None of them fails a check:

- The Choregraph forge (`git.onix.computer`) answered HTTP 530, so the pinned git inputs came from the local caches.
- Narinfo lookups at an unreachable binary cache timed out.
- Nixpkgs printed two `stdenv.isLinux`/`isDarwin` deprecation warnings during evaluation.

## Accepted-spec sync at T0

- `.cairn/specs/campaign/spec.md` and `.cairn/specs/frontier-operational-discipline/spec.md` hold the synced specs.
- `cairn traceability coverage`: verdict `pass`, with 180 requirements (159 Campaign and 21 frontier-discipline IDs); `missing` and `dangling` are both empty.
- `cairn validate`: valid, with no issues. The proposal, design, and tasks gates pass.

## Log

- Path: `/tmp/campaign-flake/check-T0-fc05da20b8c019d96141a26716815c96e3af001d.log` (276284 bytes).
- BLAKE3: `b46a87715f9c0895070a886f82ed5c6162ccf0dda5be066cb68360d99e4a8619`.

The log is kept locally. It is not part of the repository.

## What this proves

At T0, every `x86_64-linux` check of the flake passes. These checks cover:

- format;
- the workspace build and tests;
- Clippy;
- the Wasm builds;
- Nickel configuration;
- the dependency, source, and provenance boundaries;
- Octet deny-all;
- Cairn validation with the synced specs and the approved plan review.

## What this does not prove

- It proves nothing about `aarch64-linux`.
- It does not run the `compile_fail` doctests, because the Rust checks use `--all-targets`. The doctests last ran at R4 (see `verification.md`).
- It does not re-acquire the pinned inputs from their public sources.
- It makes none of the claims that `risk.md` lists under **Non-claims**, including release readiness and test sufficiency.
- It does not re-validate documents changed after T0. Such changes make the assurance plan review stale until it is refreshed.
- The archive and stable publication are still to come.
