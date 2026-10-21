## Context

The active `build-campaign-core` change defines Campaign's pure frontier policy: dynamic frontier reconstruction, candidate selection, guidance binding, budgets, pruning, and stop decisions. On its own, that change does not bind the operational shape of the frontier machinery. This change binds it. The implementation of both changes has landed in `campaign-core` and is under the shared closeout audit.

`mbrock/luv` is a Common Lisp GPU workshop. Its wiki page `frontier-traversals.org` is a source study of frontier-shaped computation. It studies a small Haskell repository as source evidence, not as a library to port. The study names five operational roles, states the reservation-before-adjacency ordering, separates discovery dynamics, and isolates reference implementations to differential evidence. This change adapts those disciplines to Campaign's policy core as normative requirements.

## Goals

- Keep frontier roles distinct and inspectable.
- Reserve child identity before parent commitment.
- Preserve identity instead of recording visit history.
- Separate discovery, relaxation, and invalidation dynamics.
- Make admission, scheduling, and stopping explicit policy.
- Keep reference implementations as evidence only.
- Separate identity, work, time, and memory in differential evidence.

## Non-goals

- A generic worklist framework or queue utility in Campaign.
- Any change to Choregraph history, branch, replay, or weave behavior.
- Any change to consumer execution, retry, snapshot, or evidence authority.
- Execution planning or lowering to SIMD or GPU kernels inside Campaign.
- Replacing `build-campaign-core`. This change binds its implementation.

## Decisions

### Decision: Roles are distinct types

**Choice:** The core models unfolding, reservation, planning, admission, scheduling, and commitment as distinct roles. The core does not share one record identity between an unvisited candidate, a queued work item, and a computed result.

**Rationale:** Role collapse is the recurring frontier defect. The luv study warns that "neighbor," "unvisited node," "queued item," and "computed result" all too easily become one record. Separation keeps profiler output and debugger state legible.

### Decision: Reservation precedes adjacency

**Choice:** An observation that binds child moments reserves a stable identity for every child before the parent commits. A child does not need to leave the frontier for the parent to commit.

**Rationale:** The luv study calls this ordering the small invariant that makes shared and cyclic structure ordinary rather than special. It prevents waiting on a recursive post-order fold where no waiting belongs.

### Decision: Identity is durable, not a visited set

**Choice:** Discovery creates stable result identity. Re-discovering the same identity observes the same result.

**Rationale:** A visited bit answers whether an identity was seen. It does not answer which stable result the identity owns. The luv study states this distinction directly.

### Decision: Dynamics are separate

**Choice:** Discovery, monotone relaxation, and invalidation are distinct dynamics. Revision and removal are not forced through an admission protocol that pretends deletion is an ordinary join.

**Rationale:** "Has an identity" and "has reached its final value" are different predicates. Forcing them together hides delete semantics and makes removal look like an ordinary addition.

### Decision: Admission and coalescing are policy

**Choice:** Admission and coalescing decisions come from explicit policy and appear in receipts. Push and pop operations do not imply the admission rule.

**Rationale:** A generic queue cannot infer when a worse candidate is obsolete or when two updates should merge. The admission and coalescing seam is policy, not queue convenience.

### Decision: Scheduling is declared policy

**Choice:** Priority and stopping are explicit decisions. Policy differences are explained by declared admission, priority, and stopping rules.

**Rationale:** The luv study shows that different algorithms differ mainly at admission and order. Declared scheduling keeps that difference inspectable.

### Decision: Reference implementations are evidence only

**Choice:** A reference policy implementation stays reachable only through an explicit reference path. Production dispatch selects the compiled or selected policy.

**Rationale:** A retired solver remains only as test or reference evidence. Keeping it runtime-selectable risks silent algorithm drift.

### Decision: Evidence separates questions

**Choice:** Differential evidence separates identical fields, equal semantic work, time, and memory into distinct facts.

**Rationale:** Identical fields do not prove identical work. The luv study's A/B comparison asks four different questions without confusing them.

## Implementation

Each decision maps to `campaign-core` code as follows:

- **Roles:** `discipline/classification` (`project_frontier_roles`, `check_no_role_collapse`). Unfold maps to stable discoveries, reserve to child reservations, plan to retained guided candidates, admit to admitted candidates, schedule to outstanding selections, and commit to accepted results. Receipts bind the role projection identity. Conformance requires an adapter's role identities to equal the candidate, selection, and result identities.
- **Reservation:** result settlement in `frontier/settlement` reserves an identity for every child before the parent's result commits.
- **Identity:** stable discoveries are keyed by child moment (`FrontierState::discovery`). A rediscovery observes the existing discovery. A payload already bound to another child or result is `ConflictingResult`.
- **Dynamics:** `discipline/claims` (`FrontierDynamics`, `validate_dynamics`, called on the admission and retention replay paths). `admit_guidance` accepts only discovery, strict improvement, or a coalesced duplicate. `revise_guidance`, retirement, and pruning carry explicit invalidation identities.
- **Admission and coalescing:** `DuplicateCandidateRule::{Reject, Coalesce}` is bound into the admission policy identity. Batch events record per-candidate outcomes. A rejected duplicate yields `explain_admission_refusal` and a refusal receipt.
- **Scheduling:** `discipline/declared` (`SchedulingRules`, `compare_scheduling`, `explain_selection_difference`). An explanation re-plans both requests on the shared state and accepts only declared priority or entropy differences for a changed candidate.
- **Reference isolation:** `PolicyImplementation` is bound into the selection profile. Planning, replay, publication, and receipts refuse `ReferenceEvidence` with `ReferencePolicySelected`. The readable reference planner is reachable only through `campaign_conformance::reference`. It computes budget fit, guidance waits, and rank-order refusals as production does, and the differential corpus includes those boundary states.
- **Evidence:** `discipline/comparison` (`compare_decisions`, `validate_differential_evidence`). Field identity and planning work are separate answers. Time and memory are reported as not measured, and any other answer is refused.

Test evidence for these laws is listed in `build-campaign-core/verification.md`.

## Dependency Direction

```text
campaign-core
      ^
      |
frontier-operational-discipline (this change binds its frontier types)
```

No new dependencies. This change is a binding amendment to `build-campaign-core`.

## Decision Flow

```text
consumer candidates, guidance, entropy, and results
  -> core unfolds, reserves, plans, admits, schedules, and commits
  -> core records admission, scheduling, and stop decisions in receipts
  -> reference policy runs only on explicit reference paths
```

## Adversarial Audit

The audit must try to show that Campaign:

- collapses role identities into one record;
- commits a parent before every child holds a reserved identity;
- treats re-discovery as a new result;
- forces removal through an addition-shaped admission protocol;
- hides admission or coalescing from receipts;
- explains a scheduling difference with no declared policy;
- keeps a reference solver in production dispatch;
- reports identical fields as identical work.

The `build-campaign-core` closeout review rounds include these items, under the same closeout criterion (binding decisions D73, D93, and D96, amended for the final round by D99):

> Closeout requires a review round with no P1 and no P2 finding. P1/P2 findings are repaired and reviewed again. The findings of the final clean round are recorded in risk.md as residual risks with owners (and repaired when cheap).

Binding decision D99 (user, 2026-09-26) amends this criterion for round 12, the final round: its three P2 findings, conformance coverage gaps in the history presented to loads and caches and in the planner-stop plan receipts of consumer- and policy-stopped states, are repaired and verified by regression tests without a further review round, and its P3 findings, together with every dimension the conformance suite does not declare, are recorded in the `build-campaign-core` `risk.md` as residual risks with owners.

Rounds classify findings with the severity rubric recorded in the `build-campaign-core` design (**Adversarial Audit**). Under that rubric, a conformance shell counts as P2 only when it models a plausible consumer implementation strategy; a shell keyed to suite internals, such as fixture names, suite-chosen identities, or thresholds beyond the maxima the suite presents, is a P3 coverage gap. From round 11 on, each round is a specification-compliance and coverage review (D96): P1 and P2 findings need an executed test, and the infra checker is reviewed only for ordinary repository changes, so deliberately evasive constructions are P3 (D93). From round 12 on, a shell whose misbehaviour needs a conjunction of five or more declared state feature values, facts the declared features merge, or states beyond the declared bounds of `campaign.conformance.state_coverage` (eight control events after the admission, three candidates, the suite's seven admission configurations) is also P3 (D97).

Each repaired finding is kept as a named regression test.

## Risks and Trade-offs

- Distinct role types add surface area in the core. The audit demands explicit role separation regardless.
- Identity preservation requires more than a visited bit. The receipts must record the stable correspondence.
- Strict dynamics separation adds policy surface. It prevents silent delete semantics.
- The luv laws are source evidence, not proof of fitness for Campaign's exact contract. Fixtures must demonstrate each law on Campaign's own policy surface.

## Non-Claims

This change does not prove exhaustive exploration, candidate quality, search usefulness, event truth, payload authenticity, action success, or release readiness.
