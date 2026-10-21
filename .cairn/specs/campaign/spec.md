# Campaign Specification

## Purpose

Defines the `campaign` capability.

## Requirements

### Requirement: Reviewed source inventory
r[campaign.source_inventory] The repository MUST record exact source revisions, files, licenses, adaptations, contributor ownership, and retained authority for every adopted mechanism.

#### Scenario: Complete inventory passes
r[campaign.source_inventory.complete]
- GIVEN the reviewed Campaign source inventory
- WHEN provenance validation runs
- THEN every adopted mechanism maps to an exact immutable source and retained authority boundary

#### Scenario: Unrecorded import fails
r[campaign.source_inventory.missing]
- GIVEN implementation copied or adapted from an unrecorded source
- WHEN provenance validation runs
- THEN stable publication fails

### Requirement: Published Choregraph history dependency
r[campaign.choregraph_dependency] The Campaign history adapter MUST consume one published immutable Choregraph history revision and MUST NOT depend on an ambient sibling worktree.

#### Scenario: Published source identities agree
r[campaign.choregraph_dependency.valid]
- GIVEN Campaign Cargo, Nix, provenance, and conformance declarations
- WHEN dependency validation runs
- THEN every declaration selects the same full Choregraph revision and compatible history schema

#### Scenario: History contract is not published
r[campaign.choregraph_dependency.blocked]
- GIVEN the required Choregraph history contract exists only in a planning branch or sibling worktree
- WHEN adapter implementation or stable publication is requested
- THEN the affected work remains blocked
- AND Campaign records the exact missing publication evidence

#### Scenario: Moving source fails
r[campaign.choregraph_dependency.moving]
- GIVEN a branch, unresolved tag, sibling path, or mismatched Choregraph revision
- WHEN dependency validation runs
- THEN stable publication fails

### Requirement: Exploration-policy boundary
r[campaign.boundary] Campaign MUST own only frontier reconstruction, candidate selection, guidance binding, checked budgets, frontier pruning, stop decisions, and structural receipts.

#### Scenario: Shared policy passes
r[campaign.boundary.shared]
- GIVEN reviewed adaptive exploration behavior
- WHEN boundary validation runs
- THEN the shared API contains only product-neutral policy values and decisions
- AND boundary validation refuses a Cargo package without its own dependency policy, including a package declared in the workspace root manifest, which Cargo admits as an implicit member outside the declared member list
- AND boundary validation refuses with this rule a dependency that the declaring member's dependency allowlist does not list and that no history, effect, or consumer family refuses first, such as a clock crate declared as a normal dependency of the core, the adapter, or `campaign-profile`, and a workspace member with no allowlist or source profile, such as a new member that depends on a random-number crate

#### Scenario: Choregraph ownership leaks into Campaign
r[campaign.boundary.history_leak]
- GIVEN source code of the Campaign core, adapter, or conformance crate implements generic event DAGs, branch references, lineage, replay cursors, merge envelopes, or Universal Weave topology, a member whose dependency policy does not admit it, or an exempt tooling member, depends on a history dependency family, any workspace member takes a dependency from a source other than a registry release, a workspace member, or the pinned Choregraph revision, a manifest overrides dependency sources, the repository holds a Cargo configuration file, or a `Cargo.lock` package comes from a source other than a registry release, a workspace member, or the pinned Choregraph revision
- WHEN architecture validation runs
- THEN stable publication fails
- AND dependency family names match with `-` and `_` treated as equal, and underscore dependency tables, including `target.*` tables, are validated like their hyphenated forms
- AND a `.cargo/config` or `.cargo/config.toml` file anywhere under the repository root, outside the infra checker's own fixture roots and without following symbolic links to directories, fails validation whatever it declares, because Cargo reads every such file between a member and the filesystem root and one can replace dependency sources, compiler flags, or the compiler itself; a `path` key in the `[toolchain]` table of a `rust-toolchain` or `rust-toolchain.toml` file outside those fixture roots fails validation the same way
- AND Cargo configuration outside the repository, in ancestor directories, `CARGO_HOME`, `--config` arguments, or the environment, is not visible to repository validation and remains a documented residual risk
- AND the identifier scans behind this rule read the sources of the core, adapter, and conformance crates only: the sources of `campaign-profile` and of the exempt infra checker are not identifier-scanned, while their manifests and dependencies are still validated under their own dependency policies; this scope is a documented residual risk owned by the infrastructure checker maintainers

#### Scenario: Consumer authority leaks into Campaign
r[campaign.boundary.consumer_leak]
- GIVEN a shared API exposes product actions, faults, prompts, coverage formats, assertions, snapshots, executor requests, persistence values, or evidence types
- WHEN dependency validation runs
- THEN stable publication fails

#### Scenario: Only test code gated by exactly `cfg(test)` is exempt from the authority scans
r[campaign.boundary.test_code]
- GIVEN a Campaign source file without an inner `#![cfg(test)]`, whose identifiers name Choregraph history, consumer, or replay authority, and whose module is declared under `#[cfg(test)]` beside another declaration of the same file without that gate, or under `#[cfg(test)]` together with another `cfg` attribute on the same declaration
- WHEN architecture validation runs
- THEN the file is not exempt as test code, and validation fails as `campaign.boundary.history_leak`, `campaign.boundary.consumer_leak`, and `campaign.replay_boundary.invalid` state for production code
- AND the history, consumer, and replay authority scans exempt a module file as test code only when every module declaration naming it carries `#[cfg(test)]` and no other `cfg` attribute, or when the file's own inner `#![cfg(test)]` removes it outside tests whatever its declarations carry; within a scanned file they also exempt an item or member gated by an outer `#[cfg(test)]`, and every token that follows an inner `#![cfg(test)]` of a module body; the effect and indirection scans of `campaign.functional_core.effect` exempt no test code

#### Scenario: Architecture validation is a lexical guard
r[campaign.boundary.lexical_guard]
- GIVEN an ordinary repository change, a dependency, a source file, or a manifest entry written the ordinary way, that introduces a forbidden dependency, ambient source, or effect
- WHEN architecture, dependency, or consumer-record validation runs
- THEN validation fails through the forms `campaign.boundary.shared`, `campaign.boundary.history_leak`, `campaign.boundary.consumer_leak`, `campaign.boundary.test_code`, `campaign.choregraph_dependency.moving`, `campaign.functional_core.effect`, `campaign.entropy.ambient`, `campaign.replay_boundary.invalid`, and `campaign.consumer_pilots.valid` enumerate
- AND validation is a lexical guard over ordinary repository changes and claims no proof against deliberate evasion: contradictory or dead `cfg` attributes, disabled test harnesses, compiler or toolchain redirection, and macro or procedural-macro constructions outside the enumerated forms are outside its guarantee and remain a documented residual risk owned by the infrastructure checker maintainers

### Requirement: Pure exploration core
r[campaign.functional_core] `campaign-core` MUST be a `no_std + alloc` functional core that uses only explicit input values.

#### Scenario: Explicit values are sufficient
r[campaign.functional_core.explicit]
- GIVEN a policy, Campaign projection, request, and bounded observations
- WHEN the core reconstructs state or computes a decision
- THEN no host service is required

#### Scenario: Core performs an external effect
r[campaign.functional_core.effect]
- GIVEN core code accesses files, databases, queues, locks, clocks, randomness, threads, processes, VMs, networks, persistence, or cleanup, or any workspace member, including an exempt tooling member, depends on an ambient randomness dependency family
- WHEN architecture validation runs
- THEN validation fails
- AND validation derives every member's compiled files from its manifest, exempt tooling members included, so a `[lib]` or `[[bin]]` target outside the member's `src/`, a build script other than the member's `build.rs` or a file under its `src/`, or an `[[example]]`, `[[test]]`, or `[[bench]]` target outside the member's `src/`, `tests/`, `examples/`, or `benches/` fails validation
- AND in the `src/` sources of the core, history adapter, and conformance members, including their `#[cfg(test)]` modules, and in their build scripts, validation reads every token tree, including attributes and their arguments such as `#[doc = ...]` and `cfg_attr`, macro arguments, and macro bodies, with raw identifiers normalised, so the identifiers `include`, `include_bytes`, `include_str`, `env`, and `option_env` anywhere, any `path` attribute, direct or nested in `cfg_attr`, a `macro_rules!` definition, and a `use` rename other than `as _` each fail validation
- AND the identifier `include` anywhere in any member's sources fails validation, and every such refusal reports this scenario's rule identifier
- AND a crate root, whether a library, a binary, or a build script, that a `#[cfg(test)]` module declaration names fails validation, because the scan never exempts a crate root as test code
- AND a core-allowlisted dependency is declared once: every manifest's dependency tables, including development and target tables, may name it only as `workspace = true` without `features` or `default-features`, no `[features]` entry may enable one of its features, and the workspace declaration disables its default features and enables no feature, so no member enables a feature on or by an allowlisted core dependency

### Requirement: Complete Campaign identity
r[campaign.identity] Every admitted campaign MUST bind versioned BLAKE3 identities for its root projection, policy, guidance profile, adapter, bounds, and control-history domain.

#### Scenario: Equal inputs give equal identity
r[campaign.identity.equal]
- GIVEN two equal canonical campaign inputs
- WHEN the core derives both campaign identities
- THEN the identities are equal

#### Scenario: Changed policy fact changes identity
r[campaign.identity.changed]
- GIVEN two campaign inputs that differ in one bound policy fact
- WHEN the core derives both campaign identities
- THEN the identities are different

#### Scenario: Missing identity fails
r[campaign.identity.missing]
- GIVEN a campaign request without one required identity
- WHEN admission runs
- THEN admission fails before a selection decision is emitted

#### Scenario: A tampered admission is not current
r[campaign.identity.admission_current]
- GIVEN a caller-built Campaign state whose admission does not re-derive to itself, such as an admission whose bound value was edited without re-deriving its bounds identity and campaign identity, with the state identity recomputed over the edited admission
- WHEN a core entry point that checks state currency runs, such as planning, receipt projection, or control-event application
- THEN the state is refused as not current with `StaleState`, because currency re-derives and validates the admission's campaign identity, bounds identity, and bound values, before any decision, receipt, or event and without state mutation
- AND states built by reconstruction or by applying control events keep a valid admission, so honest histories never meet this refusal; other inconsistencies of a caller-built state, such as its selection counters, reservations, or candidate statuses, are not re-derived and remain a documented residual risk owned by the Campaign core maintainers, because hosts obtain states only from reconstruction and control-event application

### Requirement: Exact Choregraph projection
r[campaign.history_projection] Campaign decisions MUST use an exact bounded Choregraph projection that binds graph identity, control branch, generation, selected head, projection schema, and adapter identity.

#### Scenario: Current projection passes
r[campaign.history_projection.valid]
- GIVEN a projection with exact current identities and bounds
- WHEN Campaign admission or reconstruction runs
- THEN the core can use its Campaign event facts

#### Scenario: Stale or ambiguous projection fails
r[campaign.history_projection.stale]
- GIVEN a Choregraph control history with a stale generation, wrong head, unresolved or non-fast-forward lineage, unsupported schema, mismatched graph identity, or a control branch name Choregraph refuses as empty or over its label bound
- WHEN the Campaign history adapter validates it, cold or through the host's held validated projection, and plans or publishes through that validated projection
- THEN the request is denied with a typed adapter error without a new decision
- AND Campaign core denies a selection request whose expected generation or head differs from the supplied binding with `StaleProjection`

#### Scenario: Core trusts the caller-supplied binding
r[campaign.history_projection.caller_binding]
- GIVEN a caller passes Campaign core a projection binding that no longer matches Choregraph but equals the selection request's expected generation and head
- WHEN core planning runs
- THEN the core plans against that binding because it cannot observe Choregraph
- AND only the adapter's validated-projection planning and publication path refuses the stale binding; core receipt projection refuses only a binding whose identity does not re-derive from its values (`StaleProjection`), a binding whose projection schema, adapter, control domain, or control-branch name is not the admission's, and a binding whose generation is not the admission generation plus the control-event count (`CrossedIdentity`), and projects a receipt for a binding of another head or graph at that generation as supplied, as `campaign.receipts.crossed` states
- AND hosts MUST derive the binding through the adapter's validated projection before acting on a decision

#### Scenario: A fast-forward never goes back in graph time
r[campaign.history_projection.older_graph]
- GIVEN a host holds a validated control projection and presents a current projection whose graph holds fewer events than the held projection's graph, or lacks a Campaign control draft the held projection recorded as pending below its branch head
- WHEN the Campaign history adapter validates it as a fast-forward of the held projection, including the fast-forward validation that control-event publication, `pending_control_draft`, and cache validation against a held projection perform
- THEN, once the presented lineage passes the fast-forward, payload, fence, and Campaign replay checks, it refuses with `StaleGraph`, before its shared-graph check, without a new decision and without graph or branch mutation, even though the presented control lineage is a fast-forward of the held one
- AND no plan, recovery, cache use, or publication therefore runs over a lagging graph snapshot that could admit a draft beside a pending draft the host already holds
- AND cold validation of such a graph, without a held projection, still validates it as presented, and because the held projection records only the count of the graph's other events, a graph of equal or greater length that lacks some other writer's event is not refused; such an event is not part of the reserve

### Requirement: Root-based dynamic admission
r[campaign.initial_admission] Campaign MUST admit one or more bounded root moments without requiring a complete future operation inventory.

#### Scenario: Bounded roots pass
r[campaign.initial_admission.valid]
- GIVEN unique root moments and exact policy identities within every selected bound
- WHEN initial admission runs
- THEN the core returns one admitted campaign decision

#### Scenario: Future actions are unknown
r[campaign.initial_admission.dynamic]
- GIVEN valid roots and no complete list of future operations
- WHEN initial admission runs
- THEN admission succeeds
- AND later observations can introduce new candidates

#### Scenario: Invalid roots fail
r[campaign.initial_admission.invalid]
- GIVEN no root, duplicate roots, crossed history identities, or an over-limit root set
- WHEN initial admission runs
- THEN admission fails before control-history publication

#### Scenario: Control root reserves its stop
r[campaign.initial_admission.stop_reserve]
- GIVEN a Campaign admission root whose control lineage would leave no room for its own stop because the Choregraph projection-event limit is below two, the admission generation plus two cannot be represented, or the graph length plus two exceeds the graph's event limit
- WHEN the Campaign history adapter admits the control root
- THEN admission is refused after its identity checks and its shared-graph check, as `campaign.initial_admission.one_per_graph` states, with `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted` respectively, checked in that order
- AND the graph remains unchanged

#### Scenario: Control root admission retries idempotently
r[campaign.initial_admission.root_retry]
- GIVEN a host durably admitted a Campaign control root draft and did not create the control branch, and the graph already holds exactly that root draft at its expected identity with identical content and no other Campaign control event under the admission's envelope
- WHEN the Campaign history adapter admits the identical control root again
- THEN admission returns a control root admission naming the held root event and the identical root draft, over the graph as presented, without admitting another event and counting the held root once in its stop-reserve checks
- AND the retry leaves the graph unchanged, so the returned graph equals the original admission's graph unless other writers appended to the shared graph since
- AND because a Choregraph event identity covers the draft's full content, a graph event at the root draft's identity is always that identical draft
- AND once the graph holds another Campaign control event under that envelope, including a later control event of the same campaign, the retry is refused with `SharedControlGraph`, as `campaign.initial_admission.one_per_graph` states

#### Scenario: One Campaign campaign per graph and envelope
r[campaign.initial_admission.one_per_graph]
- GIVEN a Choregraph graph that holds a Campaign control event under the projection-schema, adapter, and control-domain envelope of a Campaign admission, other than the identical root draft of that admission
- WHEN the Campaign history adapter admits that admission's control root
- THEN admission is refused after its identity checks and before its stop-reserve checks with `SharedControlGraph` naming the first such event in Choregraph event-identity order, and the graph remains unchanged
- AND cold validation and fast-forward validation of a control projection whose graph holds a Campaign control root under its envelope other than its own lineage's root refuse it, after their lineage, payload, fence, Campaign replay, and older-graph checks, with `SharedControlGraph` naming the first such root in Choregraph event-identity order, so no reserve is read and nothing is planned or published through it
- AND the host-capacity reserve accounts for one campaign per graph and envelope, and Campaign campaigns under other envelopes are other writers of the graph that the reserve does not see, so hosts MUST give each Campaign campaign its own Choregraph graph, and sharded campaigns use separate graphs

### Requirement: Bounded candidate admission
r[campaign.candidate_admission] A reusable candidate MUST bind its campaign, parent moment, operation profile, operation schema, source adapter, depth, and declared cost profile.

#### Scenario: New candidate enters the frontier
r[campaign.candidate_admission.valid]
- GIVEN an exact parent moment and a unique bounded candidate
- WHEN candidate admission runs
- THEN the core returns an admitted candidate event and updated frontier decision

#### Scenario: Candidate identity is duplicate or crossed
r[campaign.candidate_admission.invalid]
- GIVEN a duplicate identity under the declared `Reject` duplicate rule, unknown parent, crossed campaign, unsupported schema, stale adapter, depth other than the parent moment's depth plus one, depth above the admitted depth bound, or invalid cost profile
- WHEN candidate admission runs
- THEN the whole batch is denied with a typed error without frontier mutation

#### Scenario: Declared coalescing records duplicates
r[campaign.candidate_admission.coalesce]
- GIVEN the admitted duplicate rule is `Coalesce` and a batch repeats a candidate identity within the batch or already admitted
- WHEN candidate admission runs
- THEN the batch event records each repeated identity as a coalesced duplicate of the existing candidate and each new identity as admitted
- AND the existing candidate, its guidance, and its selection count remain unchanged

#### Scenario: Equal batch order normalizes
r[campaign.candidate_admission.order]
- GIVEN equal candidate batches in different input orders
- WHEN admission runs
- THEN both decisions use equal canonical order and identities

#### Scenario: Operation can discover multiple moments
r[campaign.candidate_admission.batch]
- GIVEN one candidate can produce a bounded group of consumer actions per selection
- WHEN candidate admission runs
- THEN the candidate binds the operation-generation profile
- AND it does not require a concrete selection operation or resulting child moments

### Requirement: Deterministic frontier reconstruction
r[campaign.frontier] The core MUST reconstruct retained and pruned candidates, latest guidance, selection counts, and outstanding selections from one explicit control lineage.

#### Scenario: Equal history reconstructs equally
r[campaign.frontier.equal]
- GIVEN two equal canonical Campaign projections
- WHEN frontier reconstruction runs
- THEN both reconstructed frontiers and state identities are equal

#### Scenario: Conflicting control history fails
r[campaign.frontier.conflict]
- GIVEN one candidate has crossed selections, conflicting observations, a missing admission, or an invalid control-event order
- WHEN reconstruction runs
- THEN reconstruction returns a typed conflict
- AND it does not select replacement history by sort order

#### Scenario: Selected candidate remains reusable
r[campaign.frontier.selected]
- GIVEN an accepted selection event for one retained candidate
- WHEN frontier reconstruction runs
- THEN its selection count increases
- AND it remains selectable unless retirement, pruning, or policy makes it ineligible

### Requirement: Opaque deterministic guidance
r[campaign.guidance] Campaign MUST admit bounded guidance records that bind candidate, generation, integer ranks, source facts, ordering profile, and adapter identities.

#### Scenario: Ranked candidates select deterministically
r[campaign.guidance.valid]
- GIVEN each eligible candidate has one latest valid guidance record of the campaign's one rank shape
- WHEN ranking runs
- THEN the result orders candidates by their effective integer ranks lexicographically from the first rank dimension, highest first, with canonical candidate identity ascending as its final tie-break

#### Scenario: Guidance update preserves candidate identity
r[campaign.guidance.update]
- GIVEN a retained candidate with a latest guidance record and a successor record with a higher generation and lexicographically strictly better ranks
- WHEN guidance admission runs
- THEN the core admits the successor as a strict improvement and the candidate identity remains unchanged
- AND later selection uses the successor record

#### Scenario: Equal-rank guidance coalesces
r[campaign.guidance.coalesced]
- GIVEN a retained candidate whose latest guidance record has the same identity or ranks as a successor record whose generation is not lower
- WHEN guidance admission runs
- THEN the core records the successor as a coalesced duplicate naming the existing record
- AND the latest record and the guidance-record count remain unchanged, so later selection keeps using the existing record
- AND because a coalesced duplicate adds no record, it is admitted even when the admitted guidance-record bound is used up

#### Scenario: Non-improving guidance is not a relaxation
r[campaign.guidance.non_improving]
- GIVEN a retained candidate with a latest guidance record and a successor record of another identity with different ranks, whose generation is not higher, or whose generation is higher and whose ranks are not lexicographically strictly better
- WHEN guidance admission runs
- THEN admission fails with `StaleGuidance` or `NonImprovingRelaxation` respectively, without a selection or state mutation
- AND a successor with the same identity or equal ranks is never refused as non-improving: it coalesces at a generation that is not lower, as `campaign.guidance.coalesced` states, and fails with `StaleGuidance` at a lower generation
- AND the consumer lowers ranks only through an explicit revision

#### Scenario: Guidance shape is invalid
r[campaign.guidance.invalid]
- GIVEN a record has no dimensions, more dimensions than the admitted rank-dimension bound, a stale generation, an unsupported ordering profile, a rank whose magnitude exceeds the admitted rank-magnitude bound, a retired or pruned candidate, or a mismatched identity, or the record would add a guidance record as a discovery, strict improvement, or revision while the admitted guidance-record bound is used up
- WHEN guidance admission runs
- THEN admission fails with a typed error without a selection or candidate mutation, and both a record over the rank-dimension bound and a record beyond the used-up guidance-record bound fail with `BoundExceeded`
- AND an equal-rank successor submitted through guidance admission is not refused for the guidance-record bound, because it is recorded as a coalesced duplicate that adds no record
- AND an explicit revision always adds a guidance record whatever its ranks, so an equal-rank revision fails with `BoundExceeded` while the admitted guidance-record bound is used up and is admitted as a revision otherwise, as `campaign.guidance.revision` states

#### Scenario: Declared rank orders fit the rank shape
r[campaign.guidance.declared_order]
- GIVEN a campaign whose rank shape is fixed at N dimensions and an exploratory selection profile or retention rank order naming a dimension index N or above
- WHEN selection planning, receipt re-planning, scheduling re-planning, pruning planning, prune admission under that retention rank order, or replay of a selection or prune event carrying it runs
- THEN it fails with `InvalidConfig` at the entry point's rank-order validation, which precedes every entropy draw, so the refusal never depends on the draw
- AND selection planning validates the profile right after its state-currency and request-identifier checks, before its history-binding and request-currency checks and before it consults a recorded stop or any bound, so a stopped campaign refuses such an order with `InvalidConfig` and returns its recorded stop for an order within the rank shape
- AND receipt re-planning first applies its state-currency, history-binding, and request checks, which answer with their typed errors, such as `CrossedIdentity`, then validates the profile the same way before it consults a recorded stop, so it refuses such an order with `InvalidConfig` also on a stopped campaign, and compares the supplied plan with the re-planned decision only after that validation, so a supplied plan of another decision whose request carries such an order is refused with `InvalidConfig`, not `CrossedIdentity`
- AND scheduling re-planning checks the first supplied plan and request and then the second, each by validating the profile's own content and then requiring the plan to belong to its request and to the shared state, which answer with their typed errors, such as `CrossedIdentity`; it then re-plans the first request and then the second through selection planning in the order above, so it refuses such an order with `InvalidConfig`, also on a stopped campaign and before any history-binding check, and compares each supplied plan with its re-planned decision last, refusing a mismatch with `UndeclaredSchedulingDifference`; each stage runs for both pairs before the next one starts (the profile and ownership checks of the first pair and then of the second, re-planning of the first request and then of the second, and then the comparison), so when both pairs fail at the same stage, the first pair's typed error answers and the answer depends on argument order, and when they fail at different stages, the error of the earlier stage answers in either argument order
- AND pruning planning, prune admission, and replay first apply their own earlier typed checks and return those errors instead, including `CampaignStopped` for a stopped campaign, `ControlEventLimitReached` without control-event room for new work, and `BoundExceeded` for a prune request over the admitted prune-batch bound
- AND before any guidance record fixes the rank shape, indices are checked against the admitted rank-dimension bound

#### Scenario: First guidance record fixes the rank shape
r[campaign.guidance.shape]
- GIVEN a campaign whose first admitted guidance record has a rank vector of length N
- WHEN guidance admission, guidance revision, or replay of a guidance event presents a rank vector of another length
- THEN a record of another length within the admitted rank-dimension bound fails with `InvalidGuidanceShape` without state mutation
- AND a record longer than the admitted rank-dimension bound fails with `BoundExceeded` without state mutation, because that bound is checked before the fixed shape
- AND every admitted rank vector is comparable under one total order, so no admitted input can make ranking fail or disagree with the reference planner

#### Scenario: Product meaning remains outside
r[campaign.guidance.opaque]
- GIVEN a rank dimension represents coverage, novelty, severity, probability, cost, or another domain fact
- WHEN Campaign evaluates the vector
- THEN Campaign compares only the supplied bounded integer
- AND the consumer retains its semantic meaning and conversion rule

#### Scenario: Floating-point score reaches the core
r[campaign.guidance.float]
- GIVEN a consumer uses floating-point or another domain-specific score
- WHEN its adapter prepares a guidance record
- THEN the adapter MUST convert the score to bounded integer ranks
- AND the adapter identity MUST bind the conversion profile

#### Scenario: Non-improving guidance needs explicit revision
r[campaign.guidance.revision]
- GIVEN a retained candidate and a successor guidance generation whose ranks do not strictly improve the current record, including ranks equal to the current record's
- WHEN the consumer submits it as an explicit revision naming the invalidated record and a bounded reason identity
- THEN the core admits the revision as an explicit invalidation of the prior record
- AND later selection uses the successor record
- AND an equal-rank revision is admitted as a revision that adds a guidance record, never coalesced and never refused as non-improving, while the admitted guidance-record bound still admits another record, and fails with `BoundExceeded` once that bound is used up, as `campaign.guidance.invalid` states

### Requirement: Explicit exploratory entropy
r[campaign.entropy] Exploratory selection MUST receive one bounded entropy ticket and entropy-source identity as explicit inputs under a versioned selection profile.

#### Scenario: Exploratory selection is deterministic
r[campaign.entropy.valid]
- GIVEN equal frontier, profile, entropy ticket, and entropy-source identity
- WHEN exploratory selection runs
- THEN it returns the same ranked or exploratory choice

#### Scenario: Entropy is missing or crossed
r[campaign.entropy.invalid]
- GIVEN exploratory selection lacks an entropy ticket or uses a mismatched source, campaign, state, or profile identity, or ranked selection supplies any entropy ticket
- WHEN selection runs
- THEN no selection decision is emitted

#### Scenario: Core reads ambient randomness
r[campaign.entropy.ambient]
- GIVEN Campaign core code reads a random source or hidden mutable seed
- WHEN architecture validation runs
- THEN validation fails

### Requirement: Exact candidate selection
r[campaign.selection] A selection MUST bind a unique ordinal, current state, candidate, parent, generated operation, guidance record, profile, optional entropy, costs, and expected branch generation.

#### Scenario: Best eligible candidate is selected
r[campaign.selection.valid]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, the outstanding-selection bound is not full, and a non-empty frontier has one highest-ranked eligible candidate with sufficient budgets under ranked selection or the exploit branch of exploratory selection
- WHEN selection runs
- THEN the core returns one deterministic selection decision for that candidate
- AND the explore branch instead chooses by the ticket's index draw over undecayed ranks projected onto the declared exploration order

#### Scenario: Candidate is stale or ineligible
r[campaign.selection.invalid]
- GIVEN a candidate is absent, pruned, retired, over-depth, over-budget, over the outstanding bound, or bound to stale state or guidance
- WHEN selection runs
- THEN no selection decision is emitted

#### Scenario: Campaign already stopped
r[campaign.selection.stopped]
- GIVEN the campaign control lineage contains an accepted stop
- WHEN selection runs
- THEN no selection decision is emitted

#### Scenario: Candidate is selected again
r[campaign.selection.reuse]
- GIVEN a retained non-retired candidate has one completed prior selection and current guidance
- WHEN selection runs again
- THEN it can emit a new selection with a successor ordinal and distinct operation identity

#### Scenario: Selection-count adjustment is checked
r[campaign.selection.decay]
- GIVEN a selection profile adjusts integer ranks from prior selection counts
- WHEN effective ranking runs
- THEN it uses the profile's named checked arithmetic and deterministic tie rules
- AND overflow denies selection

#### Scenario: Selection does not execute work
r[campaign.selection.non_claim]
- GIVEN a valid selection decision
- WHEN a consumer inspects it
- THEN the decision contains no claim that the operation started, completed, or produced a result

### Requirement: Durable selection publication fence
r[campaign.publication_fence] A consumer MUST execute a selected operation only after durable admission of the exact control event and generation-fenced branch update.

#### Scenario: Published selection can execute
r[campaign.publication_fence.valid]
- GIVEN the host reports durable acceptance of the exact selection event and branch-reference plan
- WHEN the shell evaluates operation eligibility
- THEN the bound operation can execute through consumer-owned authority

#### Scenario: Planned selection cannot execute
r[campaign.publication_fence.unpublished]
- GIVEN a selection decision without exact durable publication evidence
- WHEN the shell evaluates operation eligibility
- THEN the operation is not executable

#### Scenario: Stale branch update fails
r[campaign.publication_fence.stale]
- GIVEN the Choregraph control branch generation or expected head changed
- WHEN publication runs
- THEN the host rejects the plan
- AND Campaign replans from a fresh projection

#### Scenario: Publication keeps the host capacity reserve
r[campaign.publication_fence.capacity]
- GIVEN a control event whose publication would leave the control lineage, its outstanding settlements, and one stop reserve while the campaign stays open beyond the host projection bound, the control branch generation range, or the Choregraph graph's event limit
- WHEN the Campaign history adapter plans its publication
- THEN publication is refused before Choregraph admission with `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted` respectively, checked in that order, without graph or branch mutation
- AND results, closures, and the stop of an open campaign remain publishable at the bound once every Campaign control draft pending on the branch head is republished; while a draft is pending, publishing any other control event from that head is refused with `PendingControlDraft`, as `campaign.publication_fence.pending` states
- AND the reserve covers one settlement event per outstanding selection, every uncommitted Campaign control draft descending from the branch head as `campaign.publication_fence.pending_reserve` states, and one stop only; child moments and other writers sharing the graph, including Campaign campaigns under other envelopes, consume unreserved graph capacity, so hosts MUST leave the reserve the validated projection exposes free and MUST give each Campaign campaign its own graph, as `campaign.initial_admission.one_per_graph` states

#### Scenario: Identical draft republication is idempotent
r[campaign.publication_fence.republication]
- GIVEN a host durably admitted a control event's exact draft and did not complete the branch update, and the graph already holds exactly that draft at its expected identity with identical content
- WHEN the adapter publishes the same control event again from the unchanged branch
- THEN publication continues with the existing event identity and without admitting another event, and yields a publication whose payload, draft, event identity, branch plan, and control event equal the original's, over the graph as presented, with a projection binding that differs only in the graph and projection identities of the graph now holding the draft
- AND republication leaves the graph unchanged, so its graph equals the original publication's graph unless other writers appended to the shared graph since
- AND the held draft is counted once in the host-capacity checks, together with the drafts the graph holds chained below it, each counted as a selection; with no draft chained below it, as a host that publishes only on durably committed branch values leaves, a republished settlement or stop still fits at the bound, while a chain below it is outside that guarantee, as `campaign.publication_fence.durable_base` states
- AND because a Choregraph event identity covers the draft's full content, a graph event at the draft's identity is always that identical draft
- AND republishing the identical control event of a held draft whose parent is the branch head stays allowed while other control drafts are pending on the same branch head, as `campaign.publication_fence.pending_several` states, a draft chained below another pending draft is refused until the drafts above it commit, as that scenario also states, and a control event whose draft the graph does not hold is refused as `campaign.publication_fence.pending` states

#### Scenario: One pending control draft per branch head
r[campaign.publication_fence.pending]
- GIVEN the Choregraph graph holds a Campaign control draft under the validated projection's envelope, whose parent is the presented control-branch head, and does not hold the draft of the control event being published
- WHEN the Campaign history adapter publishes a control event from that branch head, after its fast-forward, fence, and Campaign checks, or plans the next decision through that projection
- THEN it refuses with `PendingControlDraft` naming the pending draft's history event identity, the child of the branch head first in Choregraph event-identity order, before its host-capacity checks, before admitting any draft, and before any core planning, without graph or branch mutation
- AND a stale publisher, or a publisher that lost the branch compare-and-set, therefore cannot consume the reserve of the lineage the branch adopted, and the planner never plans past a pending draft

#### Scenario: Pending control draft is recovered first
r[campaign.publication_fence.pending_recovery]
- GIVEN a host holds a validated control projection and presents a current projection whose graph holds one or more uncommitted Campaign control drafts descending from its branch head
- WHEN the host asks the Campaign history adapter for the pending control draft
- THEN the adapter validates the current projection as a fast-forward of the held one, with that validation's typed errors, and returns the history event, Campaign control event, and payload identities of the draft to republish first, the child of the branch head first in Choregraph event-identity order, or none when no draft is pending
- AND it refuses a pending draft naming a control event the lineage already accepted with `DuplicateControlEvent`, because that draft can never complete
- AND the host MUST republish that exact control event as an idempotent republication, through the draft the adapter names, or, when that draft is one it cannot complete as `campaign.publication_fence.pending_blocked` states, the identical control event of another held child of the branch head as `campaign.publication_fence.pending_several` allows, before planning or publishing anything else on the branch and before appending any other graph event, and hosts MUST serialize graph appends per control branch
- AND recovery republishes drafts in chain order: once the named draft's branch update commits, a draft chained below it is the child of the new branch head, and the adapter names it next; recovery of such a chain, which only a host that breaks `campaign.publication_fence.durable_base` leaves, is outside the recovery guarantee

#### Scenario: A pending control draft is part of the reserve
r[campaign.publication_fence.pending_reserve]
- GIVEN a control projection whose graph holds uncommitted Campaign control drafts under its envelope that descend from its branch head, whether one draft pending on the head, drafts a host chained below it before its branch update committed, or sibling drafts racing publishers left on the head
- WHEN the Campaign history adapter validates the projection, cold or as a fast-forward of a held one, and a host reads its reserve or plans or publishes through it
- THEN the validated projection counts every such draft as a selection, the largest demand one control event makes, namely its own control event plus one settlement event, in addition to one settlement per outstanding selection and the stop of an open campaign, and exposes the control events and graph events that reserve still owes as `reserved_control_events` and `reserved_graph_events`, and the graph events left outside it, the graph's event limit less its events and `reserved_graph_events` and zero once the reserve is consumed, as `graph_room`
- AND planning through the projection starts its host-capacity checks from that same demand, publication of a new draft checks the actual demand after that draft, so results, closures, and the stop remain publishable at the bound, and republication checks the demand of the republished draft together with the drafts the graph holds chained below it, each counted as a selection, which never exceeds the counted reserve
- AND `graph_room` bounds only the appends of other writers: a published settlement or stop of the campaign draws on the reserve and leaves `graph_room` unchanged, while any other new control event takes one event of it and a new selection two, its own event and its settlement; `reserved_control_events` counts what the control history still owes and is not kept within the host projection bound or generation range, because a pending draft counted as a selection may owe less than it counts
- AND a host that publishes only on durably committed branch values, as `campaign.publication_fence.durable_base` states, and whose other graph appends stay within the exposed `graph_room` can therefore always republish the draft a partial commit left pending on the head, settle every outstanding selection including one the republished draft publishes, and publish the stop; of racing sibling drafts whose joint demand exceeds the host capacity, only a draft whose actual demand still fits republishes, as `campaign.publication_fence.pending_several` states; a draft chained below another is still counted, but its recovery is outside this guarantee
- AND hosts MUST leave the exposed reserve free; after an append beyond `graph_room` the adapter refuses whatever publication no longer fits with `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted`, checked in that order, and, once no control draft is pending, every plan through the projection that core planning accepts with the same errors in the same order; while a draft is still pending, planning answers `PendingControlDraft` first, as `campaign.publication_fence.pending` states, and a request that core planning refuses answers with core's typed error, in the order `campaign.stop.host_capacity` states

#### Scenario: Several control drafts pending on one branch head
r[campaign.publication_fence.pending_several]
- GIVEN the Choregraph graph holds more than one uncommitted Campaign control draft under the validated projection's envelope descending from the presented control-branch head, as racing publishers from one held projection leave siblings on the head or a host that breaks `campaign.publication_fence.durable_base` leaves a draft chained below another
- WHEN the host plans through that projection, asks for the pending control draft, or publishes a control event
- THEN planning, every `PendingControlDraft` refusal of a publication, and `pending_control_draft` name the same draft, the child of the branch head first in Choregraph event-identity order, and the reserve counts every descending draft, as `campaign.publication_fence.pending_reserve` states
- AND republishing the identical control event of any held draft that is a child of the branch head stays allowed; once its branch update commits, its sibling drafts are no longer descendants of the branch head, and a draft chained below it becomes the child of the new branch head, pending there and named next
- AND a draft chained below another pending draft is not a republication until the drafts above it commit, because its publication names a branch value the branch has not reached: publishing its control event from the presented head is refused, without graph or branch mutation, with `PublicationFenceMismatch` for a selection and with the Campaign replay error `StaleState` for every other control event, because every control event after the admission names the state it was computed from, which is the state after the uncommitted draft above it, whether or not the event depends on that draft, such as a result for the selection that draft publishes or a stop
- AND a control event computed from the presented head's own state, which a draft chained below another carries only when built outside the adapter's publication, is not that draft's republication and is refused with `PendingControlDraft` naming the same draft `pending_control_draft` names, as `campaign.publication_fence.pending` states
- AND when racing drafts leave the counted reserve beyond the host capacity, republication of a draft whose actual demand no longer fits is refused with `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted`, checked in that order, while a draft whose demand fits, such as a stop, still republishes, so the adapter grants no selection that cannot settle; a branch compare-and-set a host commits outside the adapter can still strand such a selection, which is the residual the serialization MUST of `campaign.publication_fence.pending_recovery` covers

#### Scenario: A pending control draft the host cannot complete blocks its head
r[campaign.publication_fence.pending_blocked]
- GIVEN a Campaign control draft pending on the branch head that the host cannot complete, because it is a forged or foreign child under the Campaign envelope whose payload is not the draft of the control event it names, because the host lost the journal of its control event, or because a lowered host bound no longer admits its republication
- WHEN the host plans through that projection, publishes a result, closure, stop, or other control event from that head, or republishes the control event the draft names
- THEN the adapter keeps refusing, with `PendingControlDraft` naming the draft `campaign.publication_fence.pending_several` names or the capacity error of the lowered bound, without graph or branch mutation, so the branch head stays blocked unless the host completes another held child of the head by republishing its identical control event, as `campaign.publication_fence.pending_several` allows
- AND Campaign offers no operation that abandons a pending draft; this residual relies on hosts serializing graph appends per control branch and is recorded as a residual risk owned by the Campaign history adapter maintainers and the consumer maintainers who write the shared graph

#### Scenario: Hosts publish only on a durably committed branch value
r[campaign.publication_fence.durable_base]
- GIVEN a host that published a Campaign control event whose branch compare-and-set is not yet durable
- WHEN the host would plan, publish, or append another control event of the campaign
- THEN the host MUST first make that branch update durable, or recover its draft as `campaign.publication_fence.pending_recovery` states, and MUST plan and publish only through a projection validated at a durably committed branch value, never through the branch value a publication plans before its compare-and-set is durable
- AND the recovery guarantees of `campaign.publication_fence.republication`, `campaign.publication_fence.pending_recovery`, `campaign.publication_fence.pending_reserve`, and `campaign.publication_fence.pending_several` cover the drafts such hosts leave, a draft whose branch update a crash left uncommitted and sibling drafts racing publishers leave on one head, and not a pipelined chain of drafts published on uncommitted branch values
- AND from the durable head, a chained selection draft is refused with `PublicationFenceMismatch` and every other chained control event with the Campaign replay error `StaleState`, as `campaign.publication_fence.pending_several` states
- AND the adapter still counts every uncommitted draft descending from the head in the reserve, each as a selection, as a conservative defence for a host that breaks this rule; such a host may be refused at the bound with `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted` when it republishes a pipelined chain whose lower drafts settle or stop, which blocks the head and can strand an executed selection; this pipelined-chain case is a residual risk owned by the Campaign history adapter maintainers and the consumer hosts

### Requirement: Checked exploration budgets
r[campaign.budgets] Campaign MUST use checked arithmetic for candidate, selection, outstanding-selection, depth, control-event, guidance, finding, and consumer resource budgets.

#### Scenario: Costs fit all budgets
r[campaign.budgets.valid]
- GIVEN an eligible candidate whose declared costs fit every remaining budget
- WHEN selection runs
- THEN the costs are reserved in the selection decision

#### Scenario: One budget is exhausted
r[campaign.budgets.exhausted]
- GIVEN a retained candidate whose declared cost in one dimension exceeds that dimension's budget minus its reserved amount
- WHEN eligibility evaluation runs
- THEN the candidate is not selected and does not await guidance
- AND selection and wait decisions name no dimension; only a `BudgetExhausted` stop identifies the exhausted dimension

#### Scenario: Arithmetic overflow fails
r[campaign.budgets.overflow]
- GIVEN budget, depth, or control-event values whose sum or difference cannot be represented
- WHEN admission, replay, or reconstruction runs
- THEN the operation fails with a typed error without wrapped arithmetic: `ArithmeticOverflow` for depth and control-event arithmetic, and `BudgetExhausted` for a replayed reservation that exceeds the remaining budget of its dimension

#### Scenario: Unrepresentable sum does not fit
r[campaign.budgets.fit]
- GIVEN a candidate cost whose sum with the reserved amount of its dimension cannot be represented
- WHEN selection eligibility or a bound stop evaluates whether the cost fits
- THEN the core compares the cost with the budget minus the reserved amount, computed by checked subtraction
- AND the candidate does not fit, without an overflow error
- AND a caller-built state whose candidate declares a cost dimension twice or out of canonical order, whose outstanding selection reserves one twice or out of order, or whose recorded `BudgetExhausted` stop names a dimension twice or out of order is refused as not current with `StaleState` before any fit is evaluated, by every entry point that checks state currency

#### Scenario: Control-event budget keeps a settlement and stop reserve
r[campaign.budgets.control_event_reserve]
- GIVEN an admitted campaign whose control-event count plus its outstanding selections plus one stop reserve, while no stop is recorded, fits the admitted control-event bound
- WHEN a candidate batch, guidance, revision, retirement, or prune event would leave that sum above the bound
- THEN the entry point fails with `ControlEventLimitReached` before emitting an event, and replay rejects such an event
- AND results and closures of outstanding selections always fit because each outstanding selection reserved its settlement event
- AND a stop always fits because it consumes the stop reserve it replaces and leaves the sum unchanged, so stop admission and replay never refuse the stop of such a campaign with `ControlEventLimitReached`

#### Scenario: External outcome does not refund selection
r[campaign.budgets.no_refund]
- GIVEN a published selection whose external operation fails or remains unobserved
- WHEN budget reconstruction runs
- THEN its reserved selection costs remain consumed

### Requirement: Explicit candidate retirement
r[campaign.retirement] Campaign MUST retire a reusable candidate only through an exact current-state control event and bounded reason identity.

#### Scenario: Candidate retires
r[campaign.retirement.valid]
- GIVEN a retained candidate and an exact consumer or policy retirement request
- WHEN retirement admission runs
- THEN the candidate becomes unavailable for new selections
- AND its identity and prior history remain unchanged

#### Scenario: Retired candidate has outstanding work
r[campaign.retirement.outstanding]
- GIVEN a retired candidate has one outstanding selection
- WHEN the exact result or unobserved closure enters control history
- THEN that selection can close
- AND the candidate remains retired

#### Scenario: Retirement is stale or crossed
r[campaign.retirement.invalid]
- GIVEN a retirement request uses stale state, an unknown candidate, or another campaign, policy, or adapter
- WHEN retirement admission runs
- THEN the request is denied without state mutation

### Requirement: Exact exploration results
r[campaign.observations] One accepted result MUST bind a published outstanding selection, its parent and operation, bounded child moments, exploration events, adapter, and optional derived records.

#### Scenario: Exact result extends exploration
r[campaign.observations.valid]
- GIVEN a published outstanding selection and one exact bounded result with zero or more child moments
- WHEN result admission runs
- THEN the core binds every child moment and supplied derived record
- AND the selection is no longer outstanding

#### Scenario: Crossed result fails
r[campaign.observations.crossed]
- GIVEN a result for another selection, parent, operation, campaign, adapter, or exploration event
- WHEN result admission runs
- THEN it is denied without control-state mutation

#### Scenario: Conflicting second result fails
r[campaign.observations.duplicate]
- GIVEN one result or closure is already accepted for a selection
- WHEN another settlement naming that selection passes the checks that precede the selection lookup
- THEN an identical resubmission is refused with `DuplicateResult` and any unequal result or closure with `InvalidRetryBoundary`, without state mutation
- AND a settlement that fails an earlier check is refused with that check's typed error instead, without state mutation, such as `CrossedIdentity` for a result of another campaign or adapter or a closure of another campaign, `MissingUncertaintyAcknowledgement` for an unacknowledged closure, and `MalformedIdentity` for a forged closure identity

#### Scenario: Result exceeds its child bound
r[campaign.observations.child_bound]
- GIVEN one result contains too many child moments or duplicate child identities
- WHEN result admission runs
- THEN the result is denied without control-state mutation

### Requirement: Explicit unobserved closure
r[campaign.unobserved] A published outstanding selection MAY close without a result only through an exact consumer reason and uncertainty acknowledgement.

#### Scenario: Unobserved selection closes explicitly
r[campaign.unobserved.valid]
- GIVEN an outstanding selection and an exact bounded consumer reason with uncertainty acknowledgement
- WHEN closure admission runs
- THEN the selection leaves the outstanding set
- AND no child moment or execution outcome is inferred

#### Scenario: Missing acknowledgement fails
r[campaign.unobserved.invalid]
- GIVEN an outstanding selection without a result or uncertainty acknowledgement
- WHEN closure admission runs
- THEN closure is denied

### Requirement: Bounded finding references
r[campaign.findings] Campaign MUST retain only bounded opaque finding references and supplied priority facts.

#### Scenario: Finding reference is retained
r[campaign.findings.valid]
- GIVEN an exact result with a unique bounded finding reference
- WHEN result admission runs
- THEN the reference enters Campaign state and counts toward the campaign finding bound
- AND it affects planning only through an explicit stopping rule's finding limit; Campaign ranking never reads findings, so a consumer that wants findings to change priority encodes them in its guidance ranks

#### Scenario: Finding does not stop by default
r[campaign.findings.non_terminal]
- GIVEN an accepted finding and no explicit stop request or stop profile
- WHEN the next decision runs
- THEN exploration can continue

#### Scenario: Campaign interprets finding meaning
r[campaign.findings.semantic_leak]
- GIVEN a rule requires defect, assertion, proof, severity, or release meaning
- WHEN shared-policy validation runs
- THEN the rule remains in a consumer adapter

### Requirement: Safe frontier pruning
r[campaign.pruning] Campaign MUST prune or deprioritize only retained candidates without outstanding selections through explicit deterministic plans.

#### Scenario: Over-limit frontier is pruned
r[campaign.pruning.valid]
- GIVEN an admitted frontier whose retained candidates exceed the selected retained-candidate bound
- WHEN pruning is planned under a declared retention profile and rank order
- THEN every retained candidate with an outstanding selection is kept and occupies one retained slot whatever its rank
- AND the remaining slots keep the highest-ordered idle retained candidates by latest guidance ranks projected onto the rank order, descending, with unguided candidates after every guided one and candidate identity ascending as the final tie-break
- AND the plan prunes up to the admitted prune-batch bound of the lowest-ordered excess candidates and identifies every pruned candidate

#### Scenario: Pruning targets authoritative data
r[campaign.pruning.invalid]
- GIVEN a pruning request would erase Choregraph events, rewrite ancestry, remove a candidate with outstanding work, delete artifacts, or claim safe deletion
- WHEN pruning runs
- THEN the request is denied

#### Scenario: Rejected pruning leaves state unchanged
r[campaign.pruning.unchanged]
- GIVEN any invalid pruning request
- WHEN the core rejects it
- THEN the prior frontier identity remains unchanged

### Requirement: Explicit pending-input wait
r[campaign.wait] Campaign MUST return an explicit wait decision when no selection can proceed while required guidance or published results remain pending.

#### Scenario: Candidate awaits guidance
r[campaign.wait.guidance]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, no candidate is selectable, and a retained candidate has no guidance record, its declared costs fit every remaining budget, the admitted guidance-record bound still admits another record, and the control-event count plus the outstanding selections plus one guidance event, one selection, its settlement, and one stop fits the admitted control-event bound
- WHEN the next decision runs
- THEN the core returns `AwaitingGuidance`, also while selections remain outstanding
- AND it does not report frontier exhaustion

#### Scenario: Guidance that can no longer arrive is not awaited
r[campaign.wait.guidance_exhausted]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, no candidate is selectable, and every retained candidate without guidance either exceeds a remaining budget or the admitted guidance-record bound is used up
- WHEN the next decision runs
- THEN the core does not return `AwaitingGuidance`
- AND it returns `AwaitingResults` while selections remain outstanding and otherwise the `BudgetExhausted` or `FrontierExhausted` stop, so the campaign never waits for guidance that cannot be admitted or selected

#### Scenario: Guidance is awaited only with room to select
r[campaign.wait.guidance_room]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, no candidate is selectable, and a retained candidate without guidance fits every remaining budget while the admitted guidance-record bound still admits another record, but the control-event count plus the outstanding selections plus one guidance event, one selection, its settlement, and one stop exceeds the admitted control-event bound
- WHEN the next decision runs
- THEN the core does not return `AwaitingGuidance`, because the guidance event would consume the last selection slot
- AND it returns `AwaitingResults` while selections remain outstanding and otherwise the `ControlEventLimitReached` stop, which stop admission then accepts

#### Scenario: Outstanding work can change the frontier
r[campaign.wait.results]
- GIVEN no stop is recorded, no candidate is currently selectable, no candidate awaits guidance, and at least one published selection remains outstanding
- WHEN the next decision runs
- THEN the core returns `AwaitingResults`
- AND it does not emit a stop or effect plan

#### Scenario: Outstanding capacity is full
r[campaign.wait.capacity]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, and an eligible candidate exists but the outstanding-selection bound is full
- WHEN the next decision runs
- THEN the core returns `AwaitingCapacity`
- AND it does not consume another selection budget

### Requirement: Explicit bounded stop decisions
r[campaign.stop] Campaign MUST emit one explicit stop reason when a bound, frontier condition, or exact consumer stop request ends policy selection.

#### Scenario: Frontier is exhausted
r[campaign.stop.frontier]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, no candidate is selectable or awaits guidance, no selection is outstanding, no retained candidate exceeds a remaining budget, and every retained candidate has guidance or the admitted guidance-record bound is used up
- WHEN the next decision runs
- THEN the core returns `FrontierExhausted`

#### Scenario: Budget is exhausted
r[campaign.stop.budget]
- GIVEN no stop is recorded, the selection and control-event bounds still admit a selection, no stopping-rule finding limit is reached, no candidate is selectable or awaits guidance, no selection is outstanding, at least one retained candidate, guided or not, exceeds a remaining selected budget, and no retained candidate without guidance fits every remaining budget while the admitted guidance-record bound still admits another record
- WHEN the next decision runs
- THEN the core returns `BudgetExhausted` listing, in canonical order, every dimension in which some retained candidate's declared cost exceeds the remaining budget

#### Scenario: Selection bound is reached
r[campaign.stop.selection_limit]
- GIVEN the admitted selection bound is reached and no selection remains outstanding
- WHEN the next decision runs
- THEN the core returns `SelectionLimitReached`
- AND no further selection is emitted

#### Scenario: Control-event bound leaves no room for a selection
r[campaign.stop.control_event_limit]
- GIVEN no stop is recorded, the selection bound is not reached, and the control-event count plus the outstanding selections plus one selection, its settlement, and one stop exceeds the admitted control-event bound
- WHEN the next decision runs
- THEN the core returns `AwaitingResults` while selections remain outstanding and otherwise `ControlEventLimitReached`
- AND the core also returns that wait or stop when the bound still admits a selection but no candidate is selectable and only the control-event bound keeps a retained candidate without guidance from guidance and selection, as `campaign.wait.guidance_room` states
- AND on an open campaign, stop admission validates only the current state's bounds and frontier, never a profile's stopping rule: it accepts `ControlEventLimitReached` exactly when the planner under a profile without a stopping-rule finding limit would return that stop for the current state, and refuses it otherwise with `InvalidStop`
- AND once a stop is recorded, including a recorded `ControlEventLimitReached` stop, the planner returns that recorded stop, while stop admission and replay refuse every further stop, `ControlEventLimitReached` included, with `CampaignStopped`, as they refuse the events `campaign.stop.terminal` lists
- AND stop admission accepts a `PolicyRequested` stop with a bounded reason identity whenever no stop is recorded, like a `ConsumerRequested` stop, whether or not a stopping-rule finding limit is reached; because it never evaluates a finding limit, it also accepts `ControlEventLimitReached` for a state in which the planner under a profile with a reached finding limit returns the `PolicyRequested` stop first, as `campaign.stop.precedence` orders them

#### Scenario: Planner decisions have one precedence
r[campaign.stop.precedence]
- GIVEN a valid selection request whose state meets the conditions of more than one decision
- WHEN the next decision runs
- THEN the core returns the first applicable decision in this order: the recorded stop; the selection bound; the control-event bound; the stopping rule's finding limit; a selection or `AwaitingCapacity` when a candidate is selectable; `AwaitingGuidance`; `AwaitingResults`; `ControlEventLimitReached` when only the control-event bound keeps a retained candidate without guidance from guidance and selection; `BudgetExhausted`; `FrontierExhausted`
- AND each bound decision waits with `AwaitingResults` instead of stopping while selections remain outstanding

#### Scenario: Consumer requests stop
r[campaign.stop.consumer]
- GIVEN an exact current-state consumer stop request with a bounded reason identity, including the Campaign history adapter's host-capacity stop under its adapter-owned capacity reason identity
- WHEN stop admission runs
- THEN the core returns `ConsumerRequested`
- AND any outstanding selection remains explicit until result or closure

#### Scenario: Host control capacity ends selection
r[campaign.stop.host_capacity]
- GIVEN a validated control projection without a pending control draft and a core decision whose next control events the host projection bound, control branch generation range, or Choregraph graph event limit cannot hold together with the outstanding settlements and one stop: one selection and its settlement for a core selection or `AwaitingCapacity` decision, or one guidance event, one selection, and its settlement for a core `AwaitingGuidance` decision
- WHEN the adapter plans the next decision through that projection
- THEN that decision becomes `AwaitingResults` while selections remain outstanding and otherwise a `ConsumerRequested` stop under the adapter-owned capacity reason identity, not `ControlEventLimitReached`
- AND core stops and result waits are kept, and when the projection's recorded reserve, its lineage with one settlement per outstanding selection and the stop of an open campaign, no longer fits the host capacity because an append went beyond the exposed `graph_room`, every plan through the projection that core planning accepts is refused with `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted`, checked in that order, because no settlement, stop, or other control event could be published there
- AND planning through the projection checks in this order: the request's binding (`ProjectionBindingMismatch`), a pending control draft (`PendingControlDraft`), core planning, whose typed errors, such as `StaleState` or `InvalidEntropy`, answer before any host-capacity check, then the recorded reserve with the capacity errors above, and last the host-capacity transformation of the core decision into the wait or capacity stop; no decision is returned while the recorded reserve does not fit, whichever error answers

#### Scenario: Accepted stop blocks later selection
r[campaign.stop.terminal]
- GIVEN an accepted stop decision
- WHEN another selection request arrives for the same campaign
- THEN no selection is emitted, and candidate, guidance, revision, retirement, prune, and selection events are refused with `CampaignStopped` at their entry points and in replay
- AND exact results or closures for prior outstanding selections remain admissible

#### Scenario: Accepted stop repeats while outstanding work settles
r[campaign.stop.recorded]
- GIVEN an accepted stop and at least one outstanding selection
- WHEN the next decision runs
- THEN the core returns the recorded stop unchanged, not `AwaitingResults`
- AND the outstanding selections still settle through exact results or closures

#### Scenario: Stop does not claim exhaustive search
r[campaign.stop.non_claim]
- GIVEN any valid stop decision
- WHEN its receipt is projected
- THEN the receipt states that unexplored operations or moments can remain

### Requirement: Fork semantics remain distinct
r[campaign.fork] An exploration fork MUST be a distinct child moment produced by one published selection operation.

#### Scenario: One operation creates sibling moments
r[campaign.fork.valid]
- GIVEN one published selection produces two exact distinct child moments from its parent
- WHEN result admission runs
- THEN both children retain the same parent and selection identity with distinct event identities

#### Scenario: Duplicate child is mislabeled as a fork
r[campaign.fork.duplicate]
- GIVEN one result repeats the same child moment identity or conflicts with an existing event
- WHEN result admission runs
- THEN the duplicate or conflict is denied

### Requirement: Infrastructure retry remains consumer-owned
r[campaign.retry_boundary] Campaign MUST NOT create a new candidate or selection for an infrastructure retry of the same published selection.

#### Scenario: Consumer retries the same selection
r[campaign.retry_boundary.valid]
- GIVEN a published selection with an uncertain or failed infrastructure execution
- WHEN the consumer retries execution
- THEN it uses the same selection and operation identities under consumer-owned duplicate-risk policy

#### Scenario: Retry is submitted as new exploration
r[campaign.retry_boundary.invalid]
- GIVEN a request labels repeated execution of the same selection as a new fork
- WHEN Campaign admission runs
- THEN the request is denied or requires a distinct consumer operation identity with distinct domain meaning

### Requirement: Replay and minimization remain outside Campaign
r[campaign.replay_boundary] Campaign MUST NOT select replay lineages, execute replay, compare replay results, or perform minimization.

#### Scenario: Consumer requests replay
r[campaign.replay_boundary.valid]
- GIVEN a consumer needs one prior exploration path
- WHEN replay preparation runs
- THEN the consumer uses Choregraph lineage and product-owned replay policy outside Campaign

#### Scenario: Campaign gains replay authority
r[campaign.replay_boundary.invalid]
- GIVEN source code of the Campaign core, adapter, or conformance crate selects semantic parents, executes a replay, evaluates replay truth, or minimizes a finding
- WHEN architecture validation runs
- THEN validation fails

### Requirement: Derived cache validation
r[campaign.cache] A cached frontier or policy state MUST remain derived and MUST bind the exact Choregraph projection, Campaign policy, adapter, and state identities.

#### Scenario: Fresh cache is accepted
r[campaign.cache.valid]
- GIVEN a cache whose complete identity matches a fresh authoritative projection
- WHEN cache validation runs
- THEN the core can use the cached derived state

#### Scenario: Stale or incomplete cache fails
r[campaign.cache.invalid]
- GIVEN a cache with a stale head, generation, graph, policy, adapter, or missing control event
- WHEN cache validation runs
- THEN the cache is rejected
- AND the host must reconstruct from authoritative history

### Requirement: Bounded Campaign receipts
r[campaign.receipts] Campaign core decisions and published control events MUST have bounded BLAKE3 receipts that bind exact history, state, request, policy, guidance, entropy, budget, adapter, and decision identities.

#### Scenario: Equal receipt inputs are stable
r[campaign.receipts.deterministic]
- GIVEN two equal canonical decision inputs
- WHEN receipt projection runs
- THEN both receipts and identities are equal

#### Scenario: Crossed receipt input fails
r[campaign.receipts.crossed]
- GIVEN receipt inputs that combine different campaigns, states, or decisions, a history binding of another projection schema, adapter, control domain, or control-branch name or at a generation other than the admission generation plus the state's control-event count, a request whose expected generation or head differs from that binding, a plan receipt whose request profile does not re-plan the supplied decision, or a selection event receipt requested with a selection profile other than the one the event carries
- WHEN receipt projection runs
- THEN projection fails with a typed error
- AND plan, event, and refusal receipt projection each check that the state is current before checking its history binding, and refuse a caller-built state without its admission event with `MissingAdmission` instead of failing an assertion
- AND Campaign core cannot observe Choregraph, so it projects a receipt for a binding of another head or graph at that exact generation as supplied; only the Campaign history adapter's validated projection refuses such a binding, and hosts MUST derive the binding through it
- AND stop events and a recorded stop carry no selection profile, so their receipts, including those of a `PolicyRequested` stop, bind whichever valid production profile the request names and do not attest that this profile's stopping rule produced the stop
- AND a receipt binds the admitted campaign policy identity from state and the exact selection profile, whose content-derived profile identity frames the profile's compiled implementation policy identity; that implementation policy identity is a distinct identity and is not compared with the admitted campaign policy identity

#### Scenario: Receipt keeps non-claims
r[campaign.receipts.non_claims]
- GIVEN a valid Campaign receipt
- WHEN a consumer inspects it
- THEN it claims no exhaustive search, event truth, operation success, replay success, finding correctness, authorization, release readiness, or safe deletion

#### Scenario: Host-capacity decisions have no plan receipt
r[campaign.receipts.host_capacity]
- GIVEN the Campaign history adapter replaced a core decision with a host-capacity `AwaitingResults` wait or host-capacity stop
- WHEN a consumer requests a pre-publication plan receipt for that decision
- THEN Campaign projects none, because core re-planning without the host bound yields a different decision and plan receipt projection refuses the mismatch
- AND the published host-capacity stop event is receipt-projectable as an event receipt, while a host-capacity wait publishes nothing and has no receipt

### Requirement: Typed Campaign configuration
r[campaign.configuration] Human-authored exploration, guidance, entropy, budget, pruning, adapter, and receipt profiles MUST use typed Nickel contracts and checked generated runtime values.

#### Scenario: Valid profile exports
r[campaign.configuration.valid]
- GIVEN a complete profile that satisfies its Nickel contracts
- WHEN export runs
- THEN one deterministic runtime value is generated

#### Scenario: Invalid profile fails
r[campaign.configuration.invalid]
- GIVEN a zero, unsupported, contradictory, overflowing, or over-limit profile value
- WHEN contract validation runs
- THEN export fails before runtime use

#### Scenario: Stale projection fails
r[campaign.configuration.stale]
- GIVEN generated runtime data that differs from current Nickel source
- WHEN freshness validation runs
- THEN repository checks fail

### Requirement: Reusable conformance suite
r[campaign.conformance] The repository MUST publish positive and negative suites for admission, reconstruction, guidance, entropy, selection, retirement, fencing, budgets, results, closures, waits, stops, pruning, caches, and receipts.

#### Scenario: Valid adapter passes
r[campaign.conformance.positive]
- GIVEN an adapter that preserves all selected contracts
- WHEN the conformance suite runs
- THEN every positive fixture passes

#### Scenario: Invalid adapter fails
r[campaign.conformance.negative]
- GIVEN an adapter that uses stale history, executes before publication, crosses identities, miscounts budgets, rewrites history, or overclaims receipts
- WHEN the conformance suite runs
- THEN at least one named negative fixture fails

#### Scenario: Conformance samples list lengths and forks at their bounds
r[campaign.conformance.bounds]
- GIVEN an adapter mapping that is correct only for short lists, shallow forks, forks of the latest history, oldest-first settlement, ranks decided by their first or last level or ordered largest level first, or narrowed entropy draws
- WHEN the conformance suite runs
- THEN it presents guidance rank levels, result children, and findings per result at length one, an intermediate length, and the active profile's maximum the harness can express, including rank families where a middle level decides and where a later level exceeds an earlier one
- AND it presents forks at depths zero to three and at the deepest depth that the projection bound, the control-event room, the graph room, and the fork fill's room admit, built over the latest held head and over the history before the earlier held head, cold and through the two catalog held projections, and presents the honest fast-forward, every rewind, the forks at every depth and head kind the held base admits, and every class of its history catalog, including forged, suffix, projection-limit, and shared-graph histories that `campaign.initial_admission.one_per_graph` refuses, through a held projection of every held length from two events, of every holder kind: idle, with a selection outstanding, settled by result, settled by closure, and stopped; the root holder, which has no shorter held prefix to rewind to and no base to fork or build catalog classes over, meets the honest fast-forward, its lineage on the older graph, and another campaign's root, and a forged publication fence, which needs a selection, is presented through every holder whose planner selects
- AND through every held projection but the longest it presents that projection's lineage on the older graph captured before its later drafts, which `campaign.history_projection.older_graph` refuses; at the deepest fork depth it presents every head kind the base admits until a holder of that kind has met them all, and then the stop head, the one head that reaches that depth
- AND over a stopped base the deepest admitted fork depth is bounded by the settlements of the selections still outstanding at the stop, and the held lineage records its stop with two selections outstanding and then a post-stop result, so the stopped holder is advanced over every post-stop settlement history as a fast-forward that loads
- AND the held matrix fails unless every holder kind meets a refused rewind, a fork of every head kind a base of that kind admits, fork depths zero to three and the deepest depth, every catalog class, the forged publication fence excepted for the stopped kind, and an older graph; every held length meets a refusal and, but the longest, an older graph; the root holder meets another campaign's root; every held length after the root meets the shallowest and deepest fork and every class its holder admits; and every stopped holder meets an advance that loads
- AND it settles two in-flight selections of one candidate by result and by closure in both orders, and decodes every entropy ticket against the scenario's explore and index draws over an eligible count that distinguishes narrowed draws
- AND it requires exact counts, exact selection attribution, and the exact load or non-fast-forward answer, so such a mapping fails a named fixture

#### Scenario: Conformance presents every stop to the claims oracle
r[campaign.conformance.claims]
- GIVEN an adapter mapping that attaches an exhaustive-search or other forbidden claim to a receipt only once Campaign stops, only for some stop reasons, or only for some control-event kinds, states, or selection modes
- WHEN the conformance suite runs
- THEN it presents to the claims oracle the event receipt of every accepted control event of each claims session, the admission, candidate batches, guidance, selections, results with findings and child moments, closures, retirements, prunes, and stops, projected against every later state of that session, running and stopped, from ranked and exploratory requests, where the stopped states cover every planner stop reason, the recorded consumer stop, and the Campaign history adapter's host-capacity stop
- AND it presents the ranked and exploratory plan receipts of every planner stop before and after it is recorded, the frontier stop of an empty frontier and of a frontier whose every candidate was selected, settled, and retired, the budget stop, the selection-limit and control-event-limit stops, and the policy stop of a reached finding limit, together with the recorded stop's `StopAccepted` event receipt and the refusal receipts the sessions produce
- AND the suite requires the consumer's receipt mapping to attach no forbidden claim to any presented receipt
- AND such a mapping fails a named fixture; a mapping keyed on the campaign state fails one within the declared state features and bounds of `campaign.conformance.state_coverage`

#### Scenario: Conformance covers every four-way combination of reachable state features
r[campaign.conformance.state_coverage]
- GIVEN an adapter mapping whose load, cache, or receipt-claim answer depends on declared features of the held or current campaign state: its status (running, or stopped by each stop reason, with the Campaign history adapter's host-capacity stop and any other consumer stop distinguished) and whether it holds guidance, has selected, has a selection outstanding, has accepted results, findings, or discoveries, has closed a selection unobserved, and has retired, pruned, or retained candidates; or on the declared dimensions of the presented history: how many events a fork adds beyond the dropped head (its fork depth) and the event sub-kind of its head
- WHEN the conformance suite runs
- THEN it builds through the adapter one campaign state for each class of a pinned covering table in which every combination of four feature values that occurs in a class reachable within eight control events after the admission is carried by some table class, where a bounded search over `campaign-core`, from each of the suite's seven admission configurations over at most three candidates, shows every class reachable within that bound belongs to the suite's declared class set, and replay shows every declared class is reached
- AND through every table state it presents to the load path and to cache validation the fixed shapes: the state's lineage on the graph before a later exploration moment (the stale graph), the honest fast-forward when one exists or the post-stop advance while a selection is outstanding, every rewind, forks of the history before the head at fork depths zero and one, and every catalog class the head admits; a bare admission, which has no earlier history, meets another campaign's root instead of forks and catalog classes
- AND it presents to the load path and to cache validation, through table states, forks of the history before the head at fork depths zero, one, two, three, and the deepest depth that the projection bound, the control-event room, the graph room, and the fork fill's room admit, and forks whose head is each event sub-kind the fork state admits (candidate batches with and without a coalesced duplicate; discovery, strictly improving, coalesced, and revising guidance; ranked and exploratory selections; results with and without child moments and findings; closures; consumer- and policy-origin retirements; prunes; consumer, host-capacity, policy, and planner stops), so that every pair of a fork depth or head sub-kind with a combination of three held feature values that some table state admits is presented
- AND at every table state it presents to the claims oracle the event receipt of every control event of the state's lineage, which contains every event sub-kind compatible with its class (candidate batches with and without a coalesced duplicate; discovery, strictly improving, coalesced, and revising guidance; ranked and exploratory selections; results with and without child moments and findings; closures; consumer- and policy-origin retirements; prunes; the class's stop), the plan receipts of every plan made along the lineage and of the state itself, including a planner stop's plan receipt at states of every class, consumer- and policy-stopped ones included, and, while the campaign runs, refusal receipts, under every combination of consumer signals
- AND the fixture fails unless every carried combination, every admitted pair of a fork depth or head sub-kind with three held feature values, and every compatible pair of receipt kind and three feature values was presented, and it fails a named fixture when the adapter's answer differs from Campaign's or a presented receipt carries a forbidden claim
- AND a mapping that misbehaves only for a combination of five or more feature values, for facts the declared features, fork depths, and head sub-kinds do not distinguish, or for states reachable only beyond eight control events after the admission, beyond three candidates, or outside the suite's seven admission configurations is outside this scenario, and so is a mapping keyed on a dimension the suite does not declare (the change's `risk.md` lists them)

### Requirement: ChaosControl exploration-policy migration
r[campaign.chaoscontrol_migration] The supported ChaosControl path MUST consume an immutable Campaign revision while ChaosControl retains execution, snapshot, coverage, fault, assertion, finding, replay, minimization, and evidence authority.

#### Scenario: Shared policy preserves selected behavior
r[campaign.chaoscontrol_migration.parity]
- GIVEN the reviewed ChaosControl frontier and the Campaign-backed adapter
- WHEN both evaluate the bounded migration corpus
- THEN candidate order, selected parents, priority results, pruning, budget use, and stop classes agree

#### Scenario: Product authority remains outside
r[campaign.chaoscontrol_migration.authority]
- GIVEN a ChaosControl candidate or observation over shared policy
- WHEN it is ranked or admitted
- THEN ChaosControl owns its domain meaning and evidence
- AND Campaign reports only structural exploration decisions

#### Scenario: Seed runner remains outside migration
r[campaign.chaoscontrol_migration.seed_runner]
- GIVEN the independent multi-seed runner needs dispatch, progress, resume, aggregation, or KVM parity
- WHEN migration scope is evaluated
- THEN those mechanics remain in ChaosControl or another execution owner
- AND Campaign does not absorb them

### Requirement: Independent consumer pilots
r[campaign.consumer_pilots] ChaosControl and one independent non-VM fixture adapter MUST pass the same dynamic-frontier conformance suite before stable publication.

#### Scenario: Two adapters pass
r[campaign.consumer_pilots.valid]
- GIVEN the ChaosControl adapter and an independent fixture adapter
- WHEN both run the shared conformance suite
- THEN both preserve consumer authority and agree on shared policy decisions
- AND boundary validation refuses a consumer-pilot record whose recorded Campaign conformance fixture count or transcript digests differ from the values the conformance suite's consumer test pins, and it counts those pins only when the conformance library keeps its test harness and declares its test module, that module registers the consumer test module without a condition or path, none of the library root, the test module, and the consumer module carries an inner `cfg` attribute, and the consumer module holds a top-level test, neither ignored, expected to panic, nor conditionally compiled, whose body names every pinned value and runs the suite by calling `run_suite`, directly or through a function of the same module that the body calls
- AND a pinned value that is not a plain decimal literal, such as a zero-prefixed count, is refused with this scenario's typed rule identifier rather than aborting validation

#### Scenario: Consumer-specific API blocks stability
r[campaign.consumer_pilots.leak]
- GIVEN a public Campaign API requires one consumer's operation, result, storage, executor, or evidence type
- WHEN publication readiness is evaluated
- THEN stable publication is denied

### Requirement: Non-VM candidate exploration fixture
r[campaign.non_vm_candidate_fixture] Shared conformance MUST include an Animus-shaped candidate fixture that preserves opaque domain facts and consumer-owned execution and acceptance.

#### Scenario: Bounded candidate policy reproduces
r[campaign.non_vm_candidate_fixture.reproducible]
- GIVEN equal candidate, rank, entropy, budget, and history projections from the non-VM fixture
- WHEN Campaign selection runs
- THEN equal inputs produce equal bounded decisions without workspace or agent effects

#### Scenario: Selection is presented as acceptance authority
r[campaign.non_vm_candidate_fixture.authority]
- GIVEN a high rank, selected candidate, zero exit status, or validator-reference identity without consumer acceptance authority
- WHEN the conformance fixture evaluates the boundary
- THEN it grants no execution, promotion, deletion, or software-world acceptance authority

#### Scenario: Foreign runtime or consumer types enter the core
r[campaign.non_vm_candidate_fixture.foreign_types]
- GIVEN BranchContext Python, FUSE operations, model prompts, validator semantics, or software-world types inside `campaign-core`
- WHEN dependency and architecture checks run
- THEN conformance fails
