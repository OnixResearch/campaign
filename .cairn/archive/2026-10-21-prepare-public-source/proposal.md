## Why

Campaign will be offered as a public GitHub source repository. Readers there have no workspace access, so the tree must state its own license terms and build prerequisites correctly.

Three facts block that today:

- The repository declares `MIT OR Apache-2.0`. The operator selected `AGPL-3.0-or-later` for current and new owned source, and the AGPL siblings already published on GitHub carry that grant.
- The README links to workspace-relative sibling paths and to a private repository. It does not say which checks run without operator credentials, how to obtain their tools, or which private inputs the complete gate needs.
- `cairn-policy/UPSTREAM.md` gives its refresh command with an operator-local path, and it calls the pinned Cairn revision published although it cannot be fetched anonymously.

Accepted specifications do not change. No requirement in `.cairn/specs/campaign/spec.md` or `.cairn/specs/frontier-operational-discipline/spec.md` concerns the repository license, README content, or public acquisition. The accepted source-inventory requirement (`campaign.source_inventory`) and its records stay as they are.

## What Changes

- Offer current and new owned material under `AGPL-3.0-or-later`. `LICENSE` becomes the complete GNU AGPL version 3 text, and a new `NOTICE` states the copyright holder, the grant, and the no-warranty terms.
- Change the Cargo workspace license, `config/repository.ncl`, the pinned contract value in `config/contracts.ncl`, and the exported `config/generated/repository.json` together. Add a negative configuration fixture that the historical value must fail.
- Keep `LICENSE-MIT` and `LICENSE-APACHE` byte-identical as historical grant texts. Earlier grants stay effective for the content they cover, including byte-identical retained content.
- Name the material this grant does not relicense: the pinned AGPL Choregraph dependency, the copied Cairn policy files (Cairn offers those exact bytes under `AGPL-3.0-or-later OR MIT OR Apache-2.0`), and registry crates.
- Update the license prose in the README and `provenance/SOURCE.md`.
- In the README, document the credential-free Cargo and Nickel checks and their tool sources. Name the private and SSH-only inputs that the complete gate, dev shell, and pre-commit hook need, and state the canonical Radicle source. Replace workspace-relative and private links, and show Git clone routes as code instead of hyperlinks.
- In `cairn-policy/UPSTREAM.md`, replace the operator-local path with the pinned flake reference, and state that the refresh needs access to the private Cairn closure.

## Impact

- **Files**: `LICENSE`, `NOTICE`, `Cargo.toml`, `config/contracts.ncl`, `config/repository.ncl`, `config/generated/repository.json`, `config/fixtures/negative/historical-license.ncl`, `config/fixtures/negative/historical-license.expect`, `provenance/SOURCE.md`, `README.md`, `cairn-policy/UPSTREAM.md`, and this change package.
- **Behavior**: Rust code, dependency revisions, lockfiles, source inventory records, and consumer pins are unchanged. The repository-configuration contract now refuses the historical license value. Recipients take on AGPL obligations for new owned material.
- **Testing**: Run the credential-free checks from a tree exported outside the operator's home, with the documented tools: rustup and Nickel from the locked nixpkgs. That run uses an empty `HOME` and Cargo home, disabled global and system Git configuration, and no SSH agent. No sibling checkout variable points at an operator checkout, and no dev shell is active. It covers Cargo fetch, format, tests, Clippy, the `wasm32` checks, `scripts/check-config.sh` (including the historical-license fixture), and `scripts/check-boundaries.sh`. Also run the complete `nix flake check -L` and the Cairn validation and gates. The negative checks are anonymous fetch refusals for the private closure at their exact locked URLs, with a public control. Further checks: byte comparisons against the base revision for every path outside the Impact list; license-only diffs within `Cargo.toml` and the repository-configuration files; gitleaks over the exported tree, the publication commit set, and all local refs; link resolution by link type; and a scan of the tracked documentation.
- **Owner and capability**: Campaign maintainers own the grant text, `NOTICE`, and the public build instructions. Once the tree is published, an outside contributor can read its license terms and run the documented credential-free subset of checks. The complete gate still needs the named private prerequisites.

## Out of Scope

- Creating the GitHub repository, pushing, mirroring, changing visibility, announcing, or pushing or announcing this change on Radicle. Publication remains a separate operator action.
- Publishing Cairn, Octet, or their private locked inputs, or repinning Campaign to the public `cairn-public` or `octet-public` exports.
- Changing the canonical repository URL, source inventory records, consumer pins, dependency revisions, APIs, algorithms, or accepted requirements.
- Legal advice, or any determination about combined works.
