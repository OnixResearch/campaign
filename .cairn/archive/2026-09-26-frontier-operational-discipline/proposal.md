## Why

Campaign owns product-neutral adaptive exploration policy. Its frontier decides which candidate can execute next.

The `mbrock/luv` workshop keeps a source study of frontier-shaped computation at `wiki/frontier-traversals.org`. That page states operational roles (unfold, reserve, plan, admit, schedule, commit), ordering laws (reservation precedes adjacency, commitment follows naming), and evidence rules (reference implementations stay outside production dispatch; differential evidence separates identity, work, time, and memory). These disciplines transfer directly to Campaign frontier policy.

The active `build-campaign-core` change describes the frontier policy but does not bind the operational shape of the frontier machinery. Without these laws, an implementation can collapse candidate identity, selection, and result into one record, or keep a reference solver selectable in production dispatch. This change adds binding discipline without duplicating the scope of `build-campaign-core`.

## What Changes

- Require distinct frontier roles and forbid role collapse.
- Require reservation before adjacency and commit only after child names are stable.
- Require identity preservation instead of visited-set booleans.
- Separate discovery, monotone relaxation, and invalidation dynamics.
- Make admission, coalescing, scheduling, and stopping explicit policy decisions.
- Keep reference policy implementations as differential evidence only, never production dispatch candidates.
- Separate identity, work, time, and memory in differential evidence.
- Add the matching positive, negative, and boundary fixture coverage.

## Impact

- **Files**: this lifecycle package and the `campaign-core` frontier machinery that `build-campaign-core` implements: `discipline/` (roles, dynamics, declared scheduling, reference isolation, differential evidence), the reservation and stable-discovery path in `frontier/settlement`, and admission and coalescing in `policy/intake`. The readable reference planner lives only in `campaign-conformance::reference`.
- **Testing**: pure policy tests, positive and negative fixtures, receipt checks, Octet, Clippy, Cairn validation and gates, and `nix flake check -L`.
- **Dependencies**: none new. Source evidence: `mbrock/luv` `wiki/frontier-traversals.org` at `638390086d236a383acd8f9eb98dea452d91ea2c`, recorded in `provenance/sources.ncl`.
- **Status**: The discipline is implemented in `campaign-core` and checked by core tests and the conformance suite. It shares the `build-campaign-core` closeout review rounds, whose rubric and framing that change's design records. Accepted-spec sync and archive are pending and follow `build-campaign-core`. This change makes no promotion or release claim.
