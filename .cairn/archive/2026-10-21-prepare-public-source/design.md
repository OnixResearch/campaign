## Context

Campaign `main` at `d5bf962017a2` is already readable from public Radicle seeds under `MIT OR Apache-2.0`. The name `OnixResearch/campaign` is free on GitHub.

A credential-isolated run on that revision passed Cargo fetch, format, tests, Clippy, the `wasm32` checks, `scripts/check-config.sh`, and `scripts/check-boundaries.sh`. Cargo fetched `choregraph-history` anonymously from `git.onix.computer`. The run used:

- a tree exported to `/var/tmp`, with no ancestor `.cargo` directory;
- `env -i` with an empty `HOME` and Cargo home;
- `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`;
- no SSH agent and no dev shell;
- Rust 1.96.0 and Nickel 1.17.0 from the locked nixpkgs and rust-overlay;
- `CHOREGRAPH_HISTORY_SOURCE` pointing only at an anonymous clone.

The complete gate (`nix flake check -L`), the dev shell, and the pre-commit hook build the pinned Cairn and Octet flakes. Anonymous probes of the locked closure (`flake.lock` and the pinned Cairn and Octet `Cargo.lock` Git sources) refuse these inputs:

- private GitHub `OnixResearch/cairn` at four revisions, including the root pin `5d5aa7dfc02e`;
- private GitHub `OnixResearch/octet` at `890f424cd7b1`;
- private GitHub `OnixResearch/slotcar` and `OnixResearch/valence`;
- SSH-only `OnixResearch/mantle`, `OnixResearch/onix-artifact`, and `OnixResearch/bounded-exec` Git sources.

The public `cairn-public` and `octet-public` exports are historyless and do not contain the pinned commits. Cairn's export states that its own dependency closure is not yet anonymously fetchable.

The README and `provenance/SOURCE.md` are among the documents that `campaign-infra-checker boundaries` scans. It refuses any 40-hex revision in them that no inventory entry records.

Cairn's licensing map at the pinned revision (`docs/licensing.md`) assigns `MIT OR Apache-2.0` to `cairn-core`, the gate contract, and "`templates/**` and reusable lifecycle examples". It assigns `AGPL-3.0-or-later` to "repository documentation and lifecycle material not carrying another notice". The five copied policy files carry no notice. Cairn's later `cairn-policy/LICENSING.md` (Cairn `d2c1dd786735`, also on public `cairn-public` main) offers `AGPL-3.0-or-later OR MIT OR Apache-2.0` for those policy files as they exist in that repository and as they appeared at an earlier historical commit. All five Campaign copies are byte-identical to the files in that repository:

| File | Blob |
|---|---|
| `contracts.ncl` | `34bb94a5b04c` |
| `default.ncl` | `13ce49606f5f` |
| `generated/cairn-policy.json` | `b8bd7044eb1d` |
| `lifecycle-store-contracts.ncl` | `ea7becf0212d` |
| `nominal-evidence-identities.ncl` | `ba2c825f194b` |

The last two also match the historical blobs.

## No-spec classification

The accepted `campaign` and `frontier-operational-discipline` specs have no requirement about the repository license, README content, or public acquisition. The source-inventory requirement and its records stay unchanged. No requirement is added, removed, or reworded.

The classification fails if any edit changes accepted requirement text, a Rust code path, a dependency revision, a lockfile, an inventory record, or a consumer pin. In that case the change moves to the spec-driven profile with a delta spec before any further edits. Two comparisons in Verification detect this. One requires every path outside the Impact list to be unchanged. The other requires every line that changes in `Cargo.toml` and the repository-configuration files to be a license line.

## Decisions

### Decision: Prospective AGPL grant with retained historical texts

**Choice:** `LICENSE` holds the complete GNU AGPL version 3 text, byte-identical to the FSF text and to the sibling copies. `NOTICE` carries the copyright line from `LICENSE-MIT`, the current grant, the AGPL no-warranty statement, the retained historical grants, and the material it does not relicense. `LICENSE-MIT` and `LICENSE-APACHE` stay byte-identical. These all name `AGPL-3.0-or-later`:

- the Cargo workspace license;
- `config/repository.ncl`;
- the `Pinned "repository license"` contract value;
- the exported projection;
- the README License section;
- one sentence in `provenance/SOURCE.md`. It states that Campaign's own material is offered under that license per `NOTICE`, and that the owner grant changes neither that license nor the licenses of reused sources.

A permanent negative fixture, `config/fixtures/negative/historical-license.ncl`, fails on the historical value. It follows the existing pinned-repository fixtures and runs in `scripts/check-config.sh`.

**Rationale:** This follows the operator selection and the layout of the AGPL siblings published on GitHub. Earlier grants cannot be revoked, so the historical texts stay and `NOTICE` limits them to the content they already cover.

**Rejected alternatives:** Keeping `MIT OR Apache-2.0` with a license map was rejected because the operator chose AGPL. Per-crate `LICENSE` and `NOTICE` copies were rejected: every member sets `publish = false` and inherits the workspace license, and no package archive is distributed, so the copies would only drift. A one-off refusal check was rejected in favor of the permanent fixture, so the refusal repeats on every configuration check.

### Decision: State the copied Cairn policy terms in NOTICE and keep the inventory record

**Choice:** `NOTICE` states that the copied policy files are byte-identical to files that Cairn offers under `AGPL-3.0-or-later OR MIT OR Apache-2.0`, and that this grant does not relicense them. The `cairn-policy` inventory entry keeps its `MIT OR Apache-2.0` record.

**Rationale:** Cairn's later grant offers the permissive choice for these exact bytes, so the recorded value is available to recipients. The inventory field means the expression read at the pinned revision, and a later grant does not belong in it.

**Rejected alternatives:** Recording `AGPL-3.0-or-later` was rejected: the reading at the pinned revision is ambiguous between the lifecycle-material and reusable-example rows, and the later grant covers the same bytes. Recording the three-way expression was rejected because it would put a later grant into a field defined by the pinned revision.

### Decision: Document credential-free checks, their tools, and every private prerequisite; keep pins

**Choice:** The README lists the Cargo and Nickel commands with their tool sources. Rust comes from rustup, which installs the toolchain that `rust-toolchain.toml` pins. Nickel 1.17.0 comes from the repository's own nixpkgs input. The README command resolves that input's revision from `flake.lock` with `jq`, through the root node, instead of writing it out. Verification exercises exactly that route. Setting `CHOREGRAPH_HISTORY_SOURCE` is documented as optional. When set, it only adds a check that the path holds the `choregraph-history` package manifest; it does not verify the checkout's revision.

The README names the private and SSH-only inputs listed in Context as prerequisites for the complete gate, the dev shell, and the pre-commit hook. It describes `OnixResearch/mantle` as readable over HTTPS but locked through SSH. It states that `scripts/check.sh`, `scripts/check-cairn.sh`, and `scripts/check-rust-gates.sh` need the private Cairn CLI or Octet hook. The pins do not change.

**Rejected alternatives:**

- Repinning to `cairn-public` or `octet-public` was rejected. Those exports carry different commits, the Octet export head is a newer lint cohort, and the Cairn export lacks an anonymous dependency closure. Adopting either is a separate reviewed cohort change.
- Naming only the two root pins was rejected. A contributor granted access to them would still fail on the rest of the private closure.
- Making checks skip when inputs are missing was rejected because it would weaken the gate.

### Decision: Public references by stable identity

**Choice:** The README names the canonical Radicle RID and a public seed clone route that was observed to serve `main`. Git clone routes appear as code, not hyperlinks, because a browser receives 404 from them. Sources with an anonymous web page link to it. Sources without a public route are named by identity and revision without a link.

Revisions that no inventory entry records appear in the README and `provenance/SOURCE.md` only abbreviated to 12 hex digits. Each abbreviated revision is cited with the file that pins it in full: a Campaign file where one exists, otherwise the upstream file, such as Octet's `Cargo.lock`. Runnable commands derive full revisions from `flake.lock` instead of writing them out. Text-only mentions of sibling paths stay in historical provenance records, where they describe how a review was performed. The archived `final-gate.md` keeps its operator path as historical evidence.

Cargo `repository` and the checker's `CANONICAL_CAMPAIGN_URL` stay unchanged. `cairn-policy/UPSTREAM.md` names the pinned flake reference, drops the claim that the revision is published, and points to the README prerequisites.

**Rejected alternatives:** Moving the canonical URL to a public seed was rejected. Consumer pins and checker policy name the `git.onix.computer` adapter, so moving it is a separate decision. Adding inventory entries for the Octet pin or the Cairn grant commits was rejected because the inventory records adopted sources, not tooling.

## Pure decisions and shell effects

No Rust code path changes. The decisions above are pure facts: one license expression, one contract value, one negative fixture, and documentation text. The shell effects are local file edits, `nickel export` regeneration, local checks, and read-only anonymous network probes. Nothing writes to a remote.

## Failure behavior

Checks run in a fixed order:

1. credential-free checks on the exported tree;
2. the complete gate and Cairn gates;
3. negative probes and byte comparisons;
4. scans and link resolution.

Any failure stops the handoff until it is fixed and the affected step re-runs.

- A Nickel projection that differs from a fresh export fails `check-config.sh` or `check-boundaries.sh`. The fix is to regenerate it, never to hand-edit it.
- `check-boundaries.sh` refuses a 40-hex revision in the README or `provenance/SOURCE.md` that no inventory entry records. The fix is to abbreviate the citation, not to add an inventory entry.
- If the historical-license fixture exports successfully, `check-config.sh` fails.
- A documented credential-free command that needs credentials or a sibling checkout makes the README wrong. The change stays open until the command or the README is corrected.
- If a path outside the Impact list differs from the base revision, or a non-license line changes in `Cargo.toml` or the repository-configuration files, the no-spec classification fails.
- An unclassified gitleaks finding stops the handoff.
- The complete gate fails at input acquisition without access to the private closure. That failure is the documented prerequisite, not a skipped check.

## Verification

- **Positive checks:**
  - Run the credential-free checks with every isolation condition in Context, on a tree exported from the prepared revision. Use the documented tool route: rustup installs the pinned toolchain into the empty home, and Nickel comes from the documented `nix shell` command.
  - Run the complete `nix flake check -L` from a clean `path:` export of the prepared revision.
  - Run Cairn validation and traceability, plus the proposal, design, and tasks gates.
  - Confirm the `LICENSE` SHA-256 equals the FSF AGPL text.
  - Review `NOTICE` against the license and retained-terms criteria.
- **Negative checks:**
  - The historical-license fixture must fail with the pinned repository-license message.
  - Anonymous `nix flake prefetch` of the private root pins must fail in an isolated environment where a public control succeeds.
  - Every private or SSH-only input listed in Context must be refused at its exact locked URL. HTTPS probes run with `GIT_TERMINAL_PROMPT=0` and disabled Git configuration. SSH probes run with `GIT_SSH_COMMAND` set to `ssh -F /dev/null` with `IdentityFile=/dev/null`, `IdentitiesOnly=yes`, `IdentityAgent=none`, and `BatchMode=yes`. OpenSSH reads its configuration and identities from the passwd home, not from `$HOME`.
  - `git diff --exit-code` against `d5bf962017a2` must be empty for every path outside the Impact list.
  - `git diff -U0` against `d5bf962017a2` for `Cargo.toml`, `config/contracts.ncl`, `config/repository.ncl`, and `config/generated/repository.json` must change only license lines.
- **Scans:**
  - Run gitleaks with default rules over three scopes: the exported tree, the publication commit set (`main` plus this change), and all local refs. Record the version and commit counts.
  - Resolve links by type: `git ls-remote` for clone routes, anonymous HTTP GET for web links, and in-tree existence for relative links.
  - Scan tracked Markdown for `](../` links leaving the repository, `/home/` paths, and links to hosts that refuse anonymous access, with the stated exceptions.

## Semantic review inputs

The no-spec classification review binds the current BLAKE3 hashes of `proposal.md`, `design.md`, `acceptance.md`, and `tasks.md` in `reviews/no-spec-classification.json`.

## Risks / Trade-offs

- Earlier permissive grants stay effective for `d5bf962017a2` and earlier content, which public Radicle seeds already serve.
- AGPL changes downstream obligations for anyone linking new Campaign material. ChaosControl, the migrated consumer, is already `AGPL-3.0-or-later`.
- Outside contributors cannot run the complete gate, the dev shell, or the pre-commit hook until the private closure becomes publicly fetchable.
- `git.onix.computer` does not serve `rad:z2scC9MCm3pxk9mX4FEidRKabQ5LN` today. Cargo `repository` metadata and ChaosControl's pins therefore point at a route that refuses anonymous clones.
- Archived evidence keeps one operator-local path in `.cairn/archive/2026-09-26-build-campaign-core/evidence/final-gate.md`. Historical evidence stays byte-identical.

## Claim boundaries

This change does not create, push, mirror, or announce a repository. It is not legal advice or a determination about combined works. It does not establish release readiness, and it does not show that the complete gate can be reproduced publicly.
