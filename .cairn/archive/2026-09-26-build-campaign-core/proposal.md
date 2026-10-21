## Why

ChaosControl already performs adaptive frontier search, coverage guidance, fault-schedule mutation, and input-tree branching. Those mechanics remain coupled to VM execution and product evidence.

Choregraph owns deterministic execution graphs, replay, and optional Universal Weave topology. Its published branchable-history crate (`choregraph-history`) owns immutable event DAGs, generation-fenced branch references, and linear-lineage projections.

Campaign must therefore own only the missing product-neutral exploration policy. It must not create another history engine or execution lifecycle.

## What Changes

- Create a pure `no_std + alloc` core for dynamic frontier reconstruction, candidate selection, guidance binding, budgets, pruning, and stop decisions.
- Admit bounded initial history projections without requiring a complete future operation inventory.
- Add a one-way adapter to a published, immutable Choregraph history contract.
- Record Campaign control events in Choregraph history and use generation-fenced branch updates before external action execution.
- Keep generic event DAGs, branch references, lineage, replay, merge, and Universal Weave behavior in Choregraph.
- Keep operation generation, execution, retries, snapshots, payloads, results, findings, evidence, and release decisions with consumers.
- Distinguish exploration forks from infrastructure retries, replay, and minimization.
- Use typed Nickel profiles, BLAKE3 identities, reusable conformance fixtures, and explicit incomplete-search claims.
- Require ChaosControl and an Animus-shaped non-VM candidate fixture before stable publication.
- Adapt bounded exploration patterns from BranchContext without importing its Python runtime, workspace operations, or automatic commit policy.

## Impact

- **New repository**: `campaign`
- **Crates**: `campaign-core`, `campaign-choregraph`, `campaign-conformance`, and `campaign-profile`, plus the repository tool `tools/campaign-infra-checker`
- **Required dependency**: published Choregraph history revision `b3e08e19750f53bdbcae970cdf58a47a791ed20b`
- **First consumer**: ChaosControl through pinned Campaign and Choregraph adapters
- **Configuration**: typed Nickel profiles with checked generated runtime values
- **Testing**: pure policy tests, history-adapter tests, positive and negative conformance, ChaosControl frontier parity, Octet, Clippy, Cairn, and Nix gates
- **Dependency evidence**: `provenance/CHOREGRAPH-PUBLICATION.md` records the public immutable source and consumed history contract, as fetched on 2026-09-06; the closeout gates ran from cached sources and make no fresh-acquisition claim
- **Additional consumer**: Animus candidate exploration, with Lattice retaining durable software-world acceptance. Their runtime adoption remains separate from the shared fixture.
- **Status**: The implementation is in closeout. Twelve review rounds have run, and each found a P1 or P2 finding or candidate. Every finding of rounds 1 to 11 is either repaired or recorded as a residual risk; round 11's three P2 findings were conformance coverage gaps keyed on campaign-state facts, which binding decision D97 repairs with bounded 4-way state coverage, the negative fixture `history.state_coverage`, which brings the shared conformance suite to 80 fixtures. Round 12, the final round, found no P1 and three P2 conformance coverage gaps: forks presented only one event ahead, fork heads of only six kinds, and planner-stop plan receipts missing from consumer- and policy-stopped states. Under binding decision D99 (user), which amends the closeout criterion, they are repaired by declaring the fork depth and fork head sub-kind of the presented history and by making planner-stop plan receipts compatible with every state class, each repair is verified by a regression test, and the review closes without a round 13. The round-12 P3 findings and every dimension the suite does not declare are recorded in `risk.md` as residual risks with owners. `verification.md` records the evidence. ChaosControl adoption, which supplies the second conformance pilot, legacy-policy parity, and a Campaign-backed KVM smoke, is complete: ChaosControl `b9436098` pins R5, and `ntmtnrtq` records it in `provenance/consumers.ncl`, which the boundary checks admit. R5 is a development pin, and fresh acquisition of it from the live forge endpoint was not observed; `risk.md` names the owners. The accepted-spec sync is done, and Campaign's final `nix flake check -L` passed at `fc05da20` (see `evidence/final-gate.md`). The archive remains, and stable publication follows it. This change makes no promotion or release claim.
