# Frontier Operational Discipline

## ADDED Requirements

### Requirement: Distinct frontier roles

r[campaign.fd.roles] The core MUST model the frontier with distinct roles: unfolding, reservation, planning, admission, scheduling, and commitment. The core MUST NOT collapse the identities of an unvisited candidate, a queued work item, and a computed result into one record type.

#### Scenario: Roles remain distinct

r[campaign.fd.roles.distinct]
- GIVEN a candidate, a queued work item, and a result refer to the same moment
- WHEN the core projects the frontier
- THEN each role MUST remain separately identifiable

#### Scenario: Collapsed identity fails

r[campaign.fd.roles.collapsed]
- GIVEN an implementation stores the unvisited candidate, the queued item, and the result as one record
- WHEN the role check runs
- THEN the check MUST report the collapse as a defect

### Requirement: Reservation precedes adjacency

r[campaign.fd.reservation] When an observation binds child moments, the core MUST reserve a stable identity for every bound child before the parent moment commits. The core MUST NOT require a child to leave the frontier before the parent commits.

#### Scenario: Parent commits before children leave the frontier

r[campaign.fd.reservation.ordering]
- GIVEN a selected operation produces child moments
- WHEN the parent observation commits
- THEN every child MUST already hold a reserved identity
- AND no child MUST be required to have left the frontier

#### Scenario: Unreserved child fails

r[campaign.fd.reservation.unreserved]
- GIVEN an observation commits a parent before all child identities are reserved
- WHEN the ordering check runs
- THEN the check MUST reject the commit

### Requirement: Identity preservation is not a visited set

r[campaign.fd.identity] The core MUST map every discovered identity to one stable result. Re-discovery of the same identity MUST NOT create a new result or a new boolean historical fact.

#### Scenario: Shared identity observes one result

r[campaign.fd.identity.shared]
- GIVEN two paths discover the same identity
- WHEN the core resolves the identity
- THEN both paths MUST observe the same stable result

### Requirement: Separate discovery dynamics

r[campaign.fd.dynamics] The core MUST treat discovery, monotone relaxation, and invalidation as distinct dynamics. The core MUST NOT force removal or revision through an admission protocol that pretends deletion is an ordinary join.

#### Scenario: Relaxation admits only strict improvement

r[campaign.fd.dynamics.relax]
- GIVEN a guidance update does not strictly improve the current value
- WHEN admission validation runs
- THEN the core MUST NOT schedule the work

#### Scenario: Invalidation is explicit

r[campaign.fd.dynamics.invalidate]
- GIVEN a prune or retirement decision revises a previously justified value
- WHEN the core processes the decision
- THEN the core MUST trace or clear the invalidated dependency explicitly
- AND the core MUST NOT present the revision as an ordinary addition

### Requirement: Admission and coalescing are explicit policy

r[campaign.fd.admission] Admission and coalescing decisions MUST come from an explicit policy. Push and pop operations MUST NOT silently imply the admission rule. The chosen admission decision MUST appear in the receipt.

#### Scenario: Duplicate admission is policy-visible

r[campaign.fd.admission.policy]
- GIVEN an implementation enqueues a duplicate work item
- WHEN the admission policy rejects or coalesces it
- THEN the receipt MUST record the rejection or coalescing reason

### Requirement: Scheduling order is policy, not accident

r[campaign.fd.scheduling] Frontier priority and stopping MUST be explicit scheduling decisions. One policy MUST differ from another only by declared admission, priority, and stopping rules.

#### Scenario: Scheduling comparison is declarable

r[campaign.fd.scheduling.declared]
- GIVEN two policies select work in different orders
- WHEN the core compares them
- THEN the declared admission, priority, and stopping rules MUST explain the difference

### Requirement: Reference implementation is evidence only

r[campaign.fd.oracle] A reference policy implementation used for differential evidence MUST remain selectable only as test or reference evidence. The reference MUST NOT remain a candidate in production runtime dispatch.

#### Scenario: Reference is not runtime-selectable

r[campaign.fd.oracle.isolated]
- GIVEN a compiled production policy and a reference policy exist
- WHEN production dispatch selects a solver
- THEN dispatch MUST select the compiled production policy
- AND the reference MUST be reachable only through an explicit reference path

### Requirement: Differential evidence separates questions

r[campaign.fd.evidence] Differential comparison MUST separate field identity, semantic work, time, and memory into distinct questions. The comparison MUST NOT conflate identical fields with identical work.

#### Scenario: Identical fields do not imply identical work

r[campaign.fd.evidence.questions]
- GIVEN two policies produce equal fields
- WHEN the comparison reports
- THEN the report MUST state field equality and work equality as separate facts

### Requirement: Fixture coverage

r[campaign.fd.verification] Positive, negative, and boundary fixtures MUST cover distinct roles, reservation order, identity preservation, dynamics separation, admission policy, scheduling declaration, reference isolation, and separated evidence questions.

#### Scenario: Focused discipline matrix passes

r[campaign.fd.verification.matrix]
- GIVEN the discipline laws, fixtures, and documentation are complete
- WHEN the focused validation runs
- THEN conforming inputs MUST behave as declared
- AND every collapsed-role, unreserved-child, revisit, hidden-admission, ambiguous-schedule, or oracle-override input MUST fail as declared
