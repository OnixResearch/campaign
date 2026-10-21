## Phase 1: Role discipline in the core

- [x] [serial] r[campaign.fd.roles] Encode distinct frontier roles (unfold, reserve, plan, admit, schedule, commit) in `campaign-core` types.
- [x] [parallel] r[campaign.fd.roles.collapsed] Add a fixture that attempts role collapse and must fail.

## Phase 2: Ordering and identity laws

- [x] [serial] r[campaign.fd.reservation] Enforce reservation before adjacency and commit-after-naming in the selection and observation path.
- [x] [parallel] r[campaign.fd.reservation.unreserved] Add a fixture that commits a parent before child reservation and must fail.
- [x] [serial] r[campaign.fd.identity] Map every discovered identity to one stable result.
- [x] [parallel] r[campaign.fd.identity.shared] Add a fixture for shared identity that must observe one result.

## Phase 3: Dynamics separation

- [x] [serial] r[campaign.fd.dynamics] Separate discovery, monotone relaxation, and invalidation in the policy core.
- [x] [parallel] r[campaign.fd.dynamics.relax] Add a fixture for a non-improving update that must not schedule work.
- [x] [parallel] r[campaign.fd.dynamics.invalidate] Add a fixture for explicit invalidation that must not read as ordinary addition.

## Phase 4: Admission, scheduling, and receipts

- [x] [serial] r[campaign.fd.admission] Make admission and coalescing explicit policy decisions.
- [x] [parallel] r[campaign.fd.admission.policy] Record admission and coalescing reasons in receipts.
- [x] [serial] r[campaign.fd.scheduling] Make priority and stopping declared policy.
- [x] [parallel] r[campaign.fd.scheduling.declared] Add a fixture that requires declared scheduling differences.

## Phase 5: Reference isolation and evidence

- [x] [serial] r[campaign.fd.oracle] Keep reference policy implementations out of production dispatch.
- [x] [parallel] r[campaign.fd.oracle.isolated] Add a fixture that attempts runtime selection of a reference solver and must fail.
- [x] [serial] r[campaign.fd.evidence] Separate identity, work, time, and memory in differential evidence.
- [x] [parallel] r[campaign.fd.evidence.questions] Add a fixture that conflates equal fields with equal work and must fail.
- [x] [serial] r[campaign.fd.verification] Add the focused positive, negative, and boundary matrix and run the discipline gates.
