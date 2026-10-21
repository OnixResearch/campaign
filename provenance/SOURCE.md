# Source inventory

The typed inventory [`sources.ncl`](sources.ncl) is the source of truth for every source Campaign adopts, depends on, vendors, or uses as a reference.
[`generated/sources.json`](generated/sources.json) is its exported form. Do not edit the export by hand.

Campaign implements its own normative contracts. It does not copy external implementation files, except the Cairn policy files listed in `cairn-policy/source-manifest.ncl`. References retain their original scope and authority.

The Choregraph history package is a pinned dependency, not an ambient sibling import. Any later source adaptation must add its exact files, revision, rights basis, and retained authority to the inventory before stable publication.

## Inventory fields

Each entry in [`sources.ncl`](sources.ncl) records:

| Field | Meaning |
|---|---|
| `id` | Kebab-case identity used by `// campaign-source: <id>` declarations |
| `repository` | Immutable repository location: an HTTPS clone URL or a Radicle identity, never a branch, tree, or release URL |
| `revision` | Full lowercase 40-hex revision that was reviewed |
| `license` | SPDX expression read at that revision; `NOASSERTION` only for reference-only sources |
| `owner` | Contributor or organization that owns the source |
| `adoption` | `dependency`, `vendored_copy`, or `reference_only` |
| `reviewed_files` | Files read at the revision |
| `adopted_mechanisms` | Mechanisms Campaign adopts |
| `campaign_paths` | Campaign paths that carry the adopted mechanisms |
| `retained_authority` | What the source keeps and Campaign does not own |
| `non_adoption` | What Campaign explicitly does not import |
| `notes` | Review notes, including license detail |

[`contracts.ncl`](contracts.ncl) enforces closed records, full revisions, immutable locations, contained paths, unique identities, and the `NOASSERTION` rule. `fixtures/` holds positive and negative contract controls.

## Validation

`scripts/check-boundaries.sh` typechecks the inventory, compares a fresh export with `generated/sources.json`, and runs the contract fixtures.

`campaign-infra-checker boundaries` then rejects the repository when:

- an entry is incomplete, moving, or maps a Campaign path that does not exist (`campaign.source_inventory.complete`);
- README or a provenance record cites a full revision that no entry records (`campaign.source_inventory.complete`);
- a pinned Git workspace dependency lacks a `dependency` entry at its pinned revision, or a `dependency` entry lacks its pin (`campaign.source_inventory.complete`);
- a Rust file declares `// campaign-source: <id>` for an unknown identity or outside that entry's `campaign_paths` (`campaign.source_inventory.missing`);
- Cargo, the Cargo lockfile, `flake.nix`, `flake.lock`, `config/generated/repository.json`, `CHOREGRAPH-PUBLICATION.md`, and the `choregraph-history` entry disagree on the Choregraph revision (`campaign.choregraph_dependency.valid`), or any of them selects Choregraph by branch, tag, path, patch, or replacement (`campaign.choregraph_dependency.moving`).

## Consumer pilot record

The typed record [`consumers.ncl`](consumers.ncl), exported to [`generated/consumers.json`](generated/consumers.json), lists every consumer pilot that runs the shared conformance suite: the in-repo Animus-shaped fixture (`in_repo_fixture`, with its adapter and test paths) and ChaosControl (`migrated_consumer`). A migrated consumer also records its repository, adoption change, branch, and commit; its Cargo and Nix Campaign pins and its Choregraph revision; its legacy-versus-Campaign parity (corpus tests, receipt BLAKE3, `cutover_ready`, and the fields compared for each decision class); its adaptive KVM smoke (record and receipt BLAKE3, bounds); the authority it retains; and its non-claims. Every pilot records its conformance fixture count, the transcript digest under the default profile, the transcript digest under `reordered_profile`, and the command that produced them.

[`contracts.ncl`](contracts.ncl) and `campaign-infra-checker boundaries` reject a record when:

- a pilot is incomplete or malformed, a digest is not 64-hex BLAKE3, an in-repo path is absent, or both pilot kinds are not present (`campaign.consumer_pilots.valid`);
- the Cargo and Nix Campaign pins differ, a revision is not a full 40-hex revision, the consumer repository is not an inventoried source, or its Choregraph revision differs from Campaign's pinned `choregraph-history` revision (`campaign.chaoscontrol_migration`);
- parity is not `cutover_ready`, or it does not compare exactly the candidate-order, selected-parent, priority-result, pruning, budget-use, and stop-class decision classes (`campaign.chaoscontrol_migration.parity`);
- a migrated consumer does not retain execution, snapshot, coverage, fault, assertion, finding, replay, minimization, and evidence authority (`campaign.chaoscontrol_migration.authority`).

The `campaign-conformance` test `tests::consumers` pins the suite evidence as three constants, `PINNED_FIXTURE_COUNT`, `PINNED_TRANSCRIPT_DIGEST`, and `PINNED_REORDERED_DIGEST` (under `exploration_order = [1]`, `retention_rank_order = [1]`), and requires both healthy Campaign pilots, the Animus-shaped and the float-scoring fixtures, to produce exactly those values. `campaign-infra-checker boundaries` reads the three constants from that module as exact literals, refuses the module when one is missing or inexact, and refuses a record whose `reordered_profile` is another profile or whose pilot claims a different fixture count or digest (`campaign.consumer_pilots.valid`). A suite or planner change therefore fails the conformance test until the constants are re-pinned, and a record not refreshed with them fails the checker. After editing the record, regenerate the export with `nickel export --format json provenance/consumers.ncl > provenance/generated/consumers.json`; `scripts/check-boundaries.sh` compares the two.

## Owner grant and licenses

Campaign's own material is offered under `AGPL-3.0-or-later`; [`NOTICE`](../NOTICE) states the grant, the retained historical `MIT OR Apache-2.0` grants, and the material it does not relicense. The owner grant below changes neither that license nor the licenses of reused sources.

OnixResearch-owned source is available for this owner-authorized extraction. The implementation must preserve third-party notices and identify contributor-owned material separately.

The public source license does not change because of this grant. The grant does not apply to unrelated recipients.

The selected Choregraph package retains its public `AGPL-3.0-or-later` license. The workspace owner grant separately permits this use of exclusively OnixResearch-owned source. `CHOREGRAPH-PUBLICATION.md` records the selected contract and grant boundary.

Third-party material retains its own licenses and notices. Luv and BranchContext remain reference-only, with no copied implementation.

## Adopted and rejected scope

Campaign adopts product-neutral frontier policy, opaque guidance binding, checked budgets, pruning plans, and stop decisions.

Campaign rejects generic history, execution lifecycle, replay cursors, persistence, Universal Weave integration, and consumer result reduction.

## Verification record

2026-09-04 review: every listed revision was resolved as a present commit in its workspace sibling clone with `git cat-file -e <revision>^{commit}`, and each license entry was read from the source tree at that revision (`LICENSE`, `LICENSES/`, `license-policy.tsv`, and workspace or per-crate `Cargo.toml` license fields). The planning-stage no-import claim was checked against the repository tree, which contains lifecycle, configuration, and provenance documents only.

2026-09-25 review for the typed inventory:

- Every sibling-clone revision and every `reviewed_files` path was confirmed with `git cat-file -e <revision>:<path>`.
- Lattice: at the reviewed revision the root `lattice` package declares `AGPL-3.0-or-later` and `[workspace.package]` declares `MIT`, so the inventory records `AGPL-3.0-or-later AND MIT`. The earlier table recorded only the workspace `MIT` value.
- BranchContext (`github.com/multikernel/branching`, no sibling clone): the GitHub API resolved commit `b0778564fbc75eb82a61687d211ee6050f9750bf` and reported SPDX `Apache-2.0`; each reviewed file was fetched at that revision.
- Luv (`github.com/mbrock/luv`, no sibling clone): the GitHub API resolved commit `638390086d236a383acd8f9eb98dea452d91ea2c` and reported no license. No root `LICENSE`, `LICENSE.md`, or `COPYING` exists at that revision, so the inventory records `NOASSERTION` for this reference-only source.
