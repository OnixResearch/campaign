# Verification of the prepared source

Recorded 2026-10-21. This record states local and anonymous observations. It does not authorize or perform publication.

## Verified revision

| Field | Value |
|---|---|
| Prepared revision | `6dc0c59a0c5a8cbffbad68956eb12c816e4fd197` (jj change `nkruronz`) |
| Tree | `5d4c17b0bf1f472699f2954775259fb64710562e` |
| Base revision | `d5bf962017a2096edda232fbef8bf39b33af7854` (`main`) |
| Host | `x86_64-linux`, Nix 2.36.0 |

Every check below ran on a `git archive` export of the prepared revision unless stated otherwise. This record and the files in `raw/` belong to a later evidence-only revision. That revision changes only lifecycle files under `.cairn/`. See "Final candidate" for how it is checked.

Retained raw outputs are in [`raw/`](raw/):

- `public-route-check.sh`, `closure-probe.py`, and `reference-scan.py`: the scripts that ran.
- Their outputs, plus the Nix prefetch, gitleaks, and Cairn gate outputs.

The two long logs are not retained in the repository; only their BLAKE3 identities are recorded here.

## Credential-free public route (positive)

`raw/public-route-check.sh` configures the isolation; the script is the record of these settings.

- The tree is exported to `/var/tmp`.
- Every command runs under `env -i` with an empty `HOME`, Cargo home, `RUSTUP_HOME`, `XDG_CACHE_HOME`, and `XDG_CONFIG_HOME`.
- `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, and `GIT_TERMINAL_PROMPT=0`.
- `GIT_SSH_COMMAND` is `ssh -F /dev/null` with `IdentityFile=/dev/null`, `IdentitiesOnly=yes`, `IdentityAgent=none`, and `BatchMode=yes`.
- `NIX_CONFIG` holds empty `access-tokens`.
- `PATH` holds only the tool bundle and the Nix CLI (Nix 2.36.0). The bundle is `/nix/store/7bvxi1jkvv82lkrq685372k3da7my5v7-campaign-public-route-tools`, built by `/nix/store/f8ad1gj57fvhhir9fvmcsaxiwxlpnhjy-campaign-public-route-tools.drv` as a `symlinkJoin` over the repository's root nixpkgs input `c043004d1c6985732bcc1cbc5a9c9aecbbb4e0f0`. It holds rustup 1.29.0, gcc-wrapper 15.3.0, Git 2.55.0, OpenSSH 10.5p1, jq 1.8.2, curl 8.21.0, and coreutils.

The log, BLAKE3 `1fac6f0d94f8f86971c538cbe16367fc72568478626ae48bef1945b9629739a0`, confirms the following:

- the exported revision at its first and last lines;
- environment variable names only, with no `SSH_AUTH_SOCK`, `CAIRN_POLICY_SOURCE_ROOT`, or `CHOREGRAPH_HISTORY_SOURCE`;
- no `.cargo` directory in the tree or any ancestor;
- rustup syncing and downloading the 1.96.0 toolchain that `rust-toolchain.toml` pins (six components);
- Cargo downloading every crate, and `choregraph-history` from `https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git`.

The invoking shell measured 914 s of wall time.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo test --workspace --all-targets --all-features --locked` | pass: 10 harnesses, 670 tests; the conformance suite (165 tests) took 853.09 s |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo check -p campaign-core --no-default-features --target wasm32-unknown-unknown --locked` | pass |
| `cargo check -p campaign-choregraph --no-default-features --target wasm32-unknown-unknown --locked` | pass |
| `cargo check -p campaign-conformance --no-default-features --target wasm32-unknown-unknown --locked` | pass |
| README Nickel command (variant below) | Nickel 1.17.0; configuration passed (6 positive, 43 negative fixtures); checker tests 98 passed; boundary checks passed |
| `check-boundaries.sh` with `CHOREGRAPH_HISTORY_SOURCE` at an anonymous clone of Choregraph `b3e08e19750f` | pass |

The Cargo commands ran exactly as the README documents them. The Nickel command differed from the README in two ways. It ran inside `sh -c '…'`, so its inner quotes became double quotes. It also had `nickel --version &&` inserted before the two scripts to record the version.

## Complete gate (positive, operator credentials)

`nix flake check -L --keep-going --option builders ''` ran from a `path:` export of the prepared revision.

- **Exit status:** the invoking shell observed exit 0. The log ends with "all checks passed!". Its BLAKE3 is `ef565edcccdee5c66de252a87790b7ff5d71b282de24bd16a5686bd19274014b`.
- **Source:** the log unpacks the flake source as `/nix/store/p9dxgz556iqrl8i5aq104admgwj67a20-source`. All 896 files under that path match the blobs of the prepared tree under `git hash-object`.
- **Checks:** all seven `x86_64-linux` checks were built: `nickel-config`, `dependency-boundaries`, `cairn-policy`, `local-checker`, `cargo-format`, `rust-workspace`, and `octet-deny-all`. `devShells.x86_64-linux.default` was evaluated.
- **Octet deny-all:** reported `Errors: 0` with cargo-octet 0.1.0. Its config hash is `b3:5d3d676972d9dfa047acbfc53d7e21ddc056fa075b127de628b9f7e3b07cf413`.
- **Cairn:** `cairn-policy` and `local-checker` reported "campaign Cairn policy and lifecycle validation passed".
- **Not claimed:** `aarch64-linux` builds.

Cairn also ran directly on the export, using the dev shell's binary `/nix/store/1cdyx7zv4mjp5rhkhb9ahnly56774ijg-cairn-0.1.0/bin/cairn` (BLAKE3 `1c13f865963662a7a1dd96d0920e20e4975e628b5adf7890ad2952f77bc3dd09`). Its policy was `cairn-policy/generated/cairn-policy.json`, policy hash `c8276321e0e82c524e09ad9357fa48a9d38ac8590263ee6f79c51a328dc6037d`. The output is in `raw/cairn-gates.jsonl`:

- the proposal, design, and tasks gates all reported PASS, with receipt hashes `26ceebe666f931a19f94786b41260bf5e97f747c5d054e3cde633bd44b88b3a2`, `cfa26b6ca646cd9283dd88db7408b361ec380232ca761aa02ccdd1700c16c15b`, and `b1b7d0d611ef597adcb734b7b8ee613342079d417dfa324eabc11a45449c4677`;
- `cairn validate --strict` exited 0;
- traceability coverage referenced 180 of 180 requirements, none missing or dangling.

## Negative checks

**Historical license.** `config/fixtures/negative/historical-license.ncl` failed export with exit 1. The two messages were "contract broken by the value of `license`" and "campaign contract: repository license must equal the pinned value AGPL-3.0-or-later". `scripts/check-config.sh` counts it among its 43 negative fixtures.

**Private root pins.** In an isolated environment with no tokens, `nix flake prefetch` returned the following (`raw/nix-prefetch.txt`):

| Reference | Exit | Observation |
|---|---|---|
| `github:OnixResearch/cairn/5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b` | 1 | archive download returned HTTP 404 |
| `github:OnixResearch/octet/890f424cd7b1b2015f39c0eb5747f1bd1e576028` | 1 | archive download returned HTTP 404 |
| `github:NixOS/nixpkgs/c043004d1c6985732bcc1cbc5a9c9aecbbb4e0f0`, the public control | 0 | `sha256-7QYjT2vHLuX9Z1pdxHXDKCbh1CR3D/2rywB9Tx0MPRg=` |

**Locked closure at exact URLs.** `raw/closure-probe.py` collects two sets of URLs. The first is every `flake.lock` node reachable from the root. The second is every Git source in the pinned Cairn and Octet `Cargo.lock` files, which it reads from local clones because those repositories refuse anonymous access. It runs `git ls-remote` on each URL with Git configuration, prompts, SSH configuration, identities, and agent disabled. Output is in `raw/closure-probes.txt`. These URLs were refused:

| URL | Revisions | Selected by |
|---|---|---|
| `https://github.com/OnixResearch/cairn` | `5d5aa7dfc02e`, `695124d45957`, `a22ea2bff65f`, `3c470b947807` | `flake.lock` |
| `https://github.com/OnixResearch/octet` | `890f424cd7b1` | `flake.lock` |
| `https://github.com/OnixResearch/slotcar` | `2f8213a88781`, `b18a547801df` | `flake.lock` |
| `https://github.com/OnixResearch/valence` | `5f1c2ba5072c` | `flake.lock` and Octet `Cargo.lock` |
| `ssh://git@github.com/OnixResearch/onix-artifact.git` | `e41340bec587` | `flake.lock` and Cairn `Cargo.lock` |
| `ssh://git@github.com/OnixResearch/mantle.git` | `0bbab77e23ab` | `flake.lock` and Cairn `Cargo.lock` |
| `ssh://git@github.com/OnixResearch/bounded-exec.git` | `44eb74eb6391` | Octet `Cargo.lock` |

HTTPS refusals report "could not read Username for 'https://github.com': terminal prompts disabled". SSH refusals report "Permission denied (publickey)". Every other URL answered, and the answering URLs serve as public controls.

## Retained terms and classification

- `LICENSE` SHA-256 is `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`. That equals `https://www.gnu.org/licenses/agpl-3.0.txt`.
- `LICENSE-MIT` blob `ba7fd84f18ad` and `LICENSE-APACHE` blob `261eeb9e9f8b` are unchanged from the base.
- The changed paths are exactly the Impact list plus this change package.
- `git diff --exit-code` against the base, excluding the Impact files and this change package, exited 0.
- `git diff -U0` against the base changes one license line in each of `Cargo.toml`, `config/contracts.ncl`, `config/repository.ncl`, and `config/generated/repository.json`, and nothing else.
- The final no-spec review on 2026-10-21 (reviewer `omp-reviewer-prepare-public-source-plan`) checked `NOTICE` blob `e041d253a43b` at the prepared revision against `a[prepare-public-source.license]` and `a[prepare-public-source.retained-terms]` and found nothing. The plan receipt in `reviews/` does not record this review; this entry does.

## Scans

gitleaks 8.30.1 ran with default rules, an empty ignore set, and full redaction. Output is in `raw/gitleaks.txt`.

| Scope | Size | Findings |
|---|---|---|
| exported tree of the prepared revision | 4.51 MB | 0 |
| ancestors of the prepared revision | 98 commits by `git rev-list`, 95 scanned (commits with patches) | 0 |
| all local refs (`--all`, including jj keep refs) | 1064 commits by `git rev-list`, 644 scanned | 0 |

`raw/reference-scan.py` produced `raw/references.txt`. It resolves README links by type:

- all eight web links returned HTTP 200;
- all nine relative links exist in the tree;
- all three clone routes shown as code answered `git ls-remote`.

Across the 33 tracked Markdown files, it found no `](../` link leaving the repository and no link to a refused host. It found `/home/` text twice. One is the declared exception, `.cairn/archive/2026-09-26-build-campaign-core/evidence/final-gate.md:17`. The other is this change's `design.md`, which names the scan pattern.

Only gitleaks covered history. References and operator paths were scanned in the tree only. History still contains operator-local paths. `cairn-policy/UPSTREAM.md` carried one from `569f98f` through the base revision; this change removes it. The archived `final-gate.md` path is present from `52c0b46` on. Commit metadata also carries the author's name and email. Choosing between a full-history push and a historyless source projection is the operator's decision.

## Final candidate

The evidence-only revision, together with any review-receipt refresh and archive move, is checked after it is frozen. Those checks are:

- Cairn validation and the `cairn-policy` flake check;
- gitleaks over the commits after the base revision and over the exported tree;
- `git diff --name-status` against the base revision, which must list only the Impact paths plus this change package at its final location, and `git diff -U0` changing only the license line in the four configuration files;
- the tracked-Markdown scan.

In that scan, `/home/` text is expected at `final-gate.md:17` (the declared exception) and as scan-pattern text in this change's `design.md` and in this record.

A revision cannot record checks of its own final bytes. The results are therefore reported with that revision's commit ID in the handoff.

## Not performed

- creating the GitHub repository;
- pushing or mirroring any ref;
- changing visibility;
- pushing or announcing this change on Radicle;
- setting `github_url` for `campaign` in the workspace catalog;
- serving `rad:z2scC9MCm3pxk9mX4FEidRKabQ5LN` on `git.onix.computer`.

## Non-claims

The credential-free checks do not show that outside contributors can run the complete gate, the dev shell, or the pre-commit hook. Those still need the refused inputs above. These observations do not establish legal compliance, release readiness, or the continuous availability of any route.
