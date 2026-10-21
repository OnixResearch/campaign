## Context

Campaign coordinates bounded adaptive exploration. Future actions become available only after prior actions produce new moments and observations.

The old design admitted a complete finite attempt inventory and owned dispatch, retry, cancellation, checkpoints, and reduction. That model fits batch work, not Antithesis-style exploration.

Choregraph already owns deterministic machine transitions. Its branchable-history design assigns immutable event DAGs, branch references, lineage projections, replay delegation, merge envelopes, and optional Universal Weave topology to separate pure crates.

Campaign must compose that contract. It must not duplicate it.

## Goals

- Select unexplored work from a dynamic bounded frontier.
- Reconstruct policy state from Campaign events in authoritative Choregraph history.
- Bind consumer-supplied guidance without importing domain semantics.
- Enforce checked exploration and resource budgets.
- Produce deterministic selection, pruning, retention, and stop decisions.
- Keep every external effect and domain claim outside the core.

## Non-goals

- Generic event history, branch references, cursors, lineage, merges, or replay.
- Universal Weave integration or mutable navigation topology.
- Action generation, execution timing, retry, cancellation, or placement.
- Filesystem, database, queue, lock, clock, process, VM, network, or artifact operations.
- Coverage meaning, assertion meaning, finding truth, replay policy, or minimization.
- Authorization, artifact or evidence retention, publication, deployment, or release decisions.

## Decisions

### Decision: Own exploration policy, not execution lifecycle

**Choice:** Campaign owns dynamic frontier state, candidate selection, guidance binding, budgets, pruning plans, retention decisions, stop reasons, and structural receipts.

**Rationale:** Choregraph owns execution-machine semantics. Consumers own effect execution and operational recovery.

### Decision: Use Choregraph as authoritative history

**Choice:** `campaign-choregraph` will depend on a published Choregraph history contract. It will map Campaign event payloads to immutable history envelopes and fenced branch-reference plans.

`campaign-core` will not depend on Choregraph types. It will consume a bounded Campaign projection that the adapter derives from Choregraph.

**Rationale:** This direction keeps the policy core reusable and prevents a second generic history implementation.

### Decision: Pin the published history contract

**Choice:** The Choregraph worktree revision `dc986f60ed99866df5509aff2d1ac0bec12fa940` remains design provenance only.

The adapter consumes published revision `b3e08e19750f53bdbcae970cdf58a47a791ed20b` through equal Cargo and Nix pins. `provenance/CHOREGRAPH-PUBLICATION.md` records the 2026-09-06 public source fetch, exact contract, and owner-grant basis. The closeout gates ran from cached sources and make no fresh-acquisition claim (see `verification.md`).

**Rationale:** A full published Git revision supplies a durable dependency. An ambient sibling worktree does not. Conformance remains a separate acceptance requirement.

### Decision: Separate exploration history from control history

**Choice:** Choregraph history can contain two explicit Campaign domains.

- Exploration events bind moments, consumer action transcripts, observations, findings, and parent relationships.
- Control events form a linear policy lineage for candidate admission, guidance updates, selections, results, closures, pruning, budgets, and stop decisions.

The current control branch is generation-fenced. The policy core reconstructs state only from its selected control lineage and bound exploration projection.

**Rationale:** Exploration can branch while policy decisions still need one authoritative order.

### Decision: Admit roots, not a complete future inventory

**Choice:** Initial admission binds one or more root moments, an exact Choregraph projection, policy identity, guidance identity, adapter identity, and named bounds.

New candidates can enter after later observations. No admission rule requires the future search space to be known.

**Rationale:** Adaptive search discovers work during execution.

### Decision: Keep reusable exploration candidates opaque

**Choice:** A candidate binds a parent moment, operation profile, operation schema, source adapter, depth, and cost profile.

A candidate remains selectable until an explicit retirement or pruning decision removes it. Each selection creates a unique selection instance and operation identity.

One selected operation can discover zero or more child moments. The consumer records each action and observation in Choregraph history.

Campaign validates identity, bounds, freshness, and duplicate rules. The consumer validates operation meaning, action composition, and feasibility.

**Rationale:** ChaosControl can expand one frontier moment many times. A one-use candidate would not preserve that behavior.

### Decision: Keep guidance separate from candidate identity

**Choice:** Consumers submit bounded guidance records that bind a candidate, rank vector, guidance generation, source facts, and adapter identity.

Campaign selects the latest accepted guidance record on the control lineage. A guidance update does not change the candidate identity.

Campaign applies the selected deterministic ordering and canonical tie-break policy. It does not interpret coverage, novelty, severity, or probability.

Consumers must convert floating-point or domain-specific scores before the boundary. The adapter identity binds the conversion rule.

**Rationale:** Candidate identity stays stable while selection count and product guidance change.

### Decision: Make exploratory entropy explicit

**Choice:** A selection profile can choose ranked or exploratory selection. Exploratory selection receives an explicit bounded entropy ticket and entropy-source identity.

The core deterministically applies the profile threshold and candidate-index rule. It never reads ambient randomness.

**Rationale:** ChaosControl uses epsilon-greedy selection. Explicit entropy preserves replayable policy without adding a hidden random source.

### Decision: Publish selection before execution

**Choice:** A selection becomes executable only after the host reports durable admission of the exact selection event and the fenced control-branch update.

Each selection binds a unique ordinal, candidate, operation, guidance record, entropy, costs, and prior Campaign state. Selection increments the candidate count.

Selection does not consume the candidate. A stale branch generation or head mismatch causes replanning from a freshly validated projection. While another publisher's control draft is pending on the branch head, planning refuses until that draft is completed (see the pending-draft decision below). Campaign does not execute the operation.

**Rationale:** Durable selection prevents concurrent planners from executing the same selection instance from stale state.

### Decision: Track only structural outstanding work

**Choice:** Campaign tracks each published selection as outstanding until an exact result or explicit unobserved closure enters control history.

A result can bind zero or more child moments. An unobserved closure requires a consumer reason and uncertainty acknowledgement.

Campaign enforces a maximum outstanding-selection bound. It does not model reserve, start, retry, cancellation, timeout, or executor state.

**Rationale:** Bounded parallel exploration needs structural bookkeeping without absorbing execution lifecycle authority.

### Decision: Treat frontier state as derived

**Choice:** Frontier state contains retained candidates, current guidance, selection counts, and outstanding selections derived from Campaign control events. It is not an independent authoritative database.

A host can cache the frontier. On restart, it must reconstruct or validate the cache against exact history and policy identities.

**Rationale:** This rule avoids another checkpoint or persistence format with separate truth.

### Decision: Wait for outstanding results before stopping

**Choice:** If no candidate can be selected while guidance or published results remain pending, Campaign returns an explicit wait decision.

A wait decision requests no effect and makes no outcome claim. Frontier or budget stop requires no pending guidance or outstanding result.

**Rationale:** Parallel exploration can temporarily use every slot or await results that introduce new candidates.

### Decision: Make accepted stops monotonic

**Choice:** An accepted stop blocks all later selections. Exact results and unobserved closures can still settle selections that were already outstanding.

Further exploration requires a new campaign identity or a later versioned resume contract. The first version has no resume transition.

**Rationale:** A stop receipt must not become temporary because unrelated input arrives later.

### Decision: Make budgets resource-based

**Choice:** Policy defines named limits for candidates, selections, outstanding selections, depth, control events, guidance records, findings, and consumer resource dimensions.

All counters use checked arithmetic. A selection reserves declared costs before action execution.

**Rationale:** Exploration is bounded by resources, not by a complete inventory.

### Decision: Keep findings non-terminal

**Choice:** Campaign records bounded opaque finding references with their supplied priority-facts identity and never reads them in ranking. Findings affect planning only through an explicit stopping rule's finding limit; a consumer that wants findings to change priority encodes them in its guidance ranks. A finding does not terminate exploration unless an explicit stop policy selects that outcome.

**Rationale:** Adaptive systems often continue after they find a defect.

### Decision: Retire candidates without deleting history

**Choice:** A consumer or policy can retire a retained candidate through an exact control event and bounded reason identity.

Retirement blocks new selections. Existing outstanding selections can still close. Retirement does not erase the candidate or its Choregraph history.

**Rationale:** A reusable candidate needs an explicit end state before the frontier can become exhausted.

### Decision: Prune only derived exploration work

**Choice:** A pruning plan can remove or deprioritize retained candidates that have no outstanding selections.

It cannot erase Choregraph events, rewrite branch ancestry, remove consumer artifacts, or claim safe deletion.

**Rationale:** Search policy and immutable history have different authority.

### Decision: Separate fork, retry, replay, and minimization

**Choice:** A fork creates a distinct child moment from one selected operation. Reselecting a candidate creates a new selection and operation identity.

An infrastructure retry repeats execution for the same published selection outside Campaign policy. Replay uses an explicit Choregraph lineage.

Minimization remains a consumer-owned search with separate identity and evidence.

**Rationale:** Combining these operations obscures duplicate risk and evidence meaning.

### Decision: Keep Universal Weave in Choregraph

**Choice:** Campaign will not add a Universal Weave dependency or adapter.

Consumers can use Choregraph's optional derived topology for navigation and visualization. That topology cannot affect authoritative Campaign decisions without an exact Choregraph projection.

**Rationale:** Choregraph already owns the correct immutable-history and mutable-topology boundary.

### Decision: Require two consumers

**Choice:** Stable publication requires ChaosControl and an Animus-shaped non-VM fixture adapter to pass the same dynamic-frontier conformance suite.

The fixture uses opaque candidate, validator-reference, and workspace-reference identities. BranchContext `b0778564fbc75eb82a61687d211ee6050f9750bf` supplies bounded exploration patterns only.

Both pilots pass: the in-repo Animus-shaped fixture, and ChaosControl at adoption commit `b9436098`, which pins Campaign R5 and is recorded in `provenance/consumers.ncl` since `ntmtnrtq` (see `verification.md`). R5 is a development pin (see `risk.md`).

Campaign does not import its Python runtime or interpret validator outcomes. A high rank, zero exit status, or selected candidate cannot grant execution, workspace promotion, or software-world acceptance authority.

**Rationale:** One consumer cannot show that the policy contract is product-neutral.

### Decision: Treat publication as dependency selection only

**Choice:** The published Choregraph revision clears dependency selection only. Local or published source availability does not prove adapter behavior. Adapter, publication-fence, and consumer conformance checks keep their own evidence obligations.

**Rationale:** Neither a design reference nor source availability can substitute for executed conformance against the pinned contract.

### Decision: Separate relaxation from explicit guidance revision

**Choice:** `admit_guidance` accepts only discovery, strict improvement, or an equal-rank coalesced duplicate. A successor that does not strictly improve the current record must arrive through `revise_guidance`, which names the invalidated record and a bounded reason and derives an explicit invalidation identity.

**Rationale:** Consumers with domain decay, such as ChaosControl's division by selection count, need lower successor ranks. Treating them as ordinary relaxation would hide a revision as an addition.

### Decision: Order exploratory choices by a declared rank projection

**Choice:** An exploratory profile declares `exploration_order`, a list of rank dimensions. Tickets carry two draws: `explore_draw` decides the branch against the threshold, and `index_draw` indexes the eligible candidates ordered by their undecayed ranks projected onto that order.

**Rationale:** A consumer can then reproduce its legacy epsilon choice exactly from its own seeded draws, and Campaign still reads no ambient randomness.

### Decision: Plan retention against a declared rank order

**Choice:** `plan_pruning` prunes the lowest-ordered candidates beyond `max_retained_candidates` by a declared `rank_order`, at most `max_prune_batch` of them per plan, so one plan can leave more than `max_retained_candidates` retained; it never prunes a candidate with outstanding work, and binds the retention profile and order into the prune plan. The retained bound does not limit admission; `max_candidates` bounds total admissions.

**Rationale:** Retention needs a deterministic, declared ranking that can differ from the selection order, as ChaosControl's truncation does.

### Decision: Reserve control-event headroom for settlement and stop

**Choice:** Every admitted state keeps `control_event_count + outstanding + 1 <= max_control_events` until a stop is recorded. The planner stops with `ControlEventLimitReached` or waits for results instead of spending the reserve.

**Rationale:** A control-event bound must never strand outstanding work or make a required stop unrecordable.

### Decision: Record selection policy inputs in history

**Choice:** A published selection event carries its full selection profile and optional entropy ticket. Replay re-validates both and recomputes the policy's choice on the prior state; an off-policy, reference-planned, or unbound selection does not replay.

**Rationale:** Authoritative history must show that each selection is the declared policy's decision, not merely a well-formed record.

### Decision: Bind publication to validated state and fast-forward lineage

**Choice:** The adapter applies each control event to the Campaign state at the fenced head before admitting it, keeps validated projections and publications opaque, grants execution only to the exact published decision, and binds a stable branch-name identity. Hosts that hold a validated projection advance it with `validate_control_advance`, which rejects rewinds, renames, and fork jumps. Every recorded selection fence must equal the branch value at its lineage position.

**Rationale:** A branch fence alone cannot prove that the published decision was computed from current state or that the branch only moved forward.

### Decision: Project receipts only for genuine decisions

**Choice:** `project_receipt` re-plans the request and requires equality with the supplied plan. Event receipts authenticate the event's content-derived identity and its membership in history. A rejected duplicate yields an explicit refusal and a refusal receipt that records the reason.

**Rationale:** A receipt must bind facts Campaign actually decided, including admission and coalescing outcomes.

### Decision: Load profiles through a strict typed loader

**Choice:** `campaign-profile` decodes the exported `campaign.profile.v2` JSON with closed records and string tags and builds core values through their constructors. Nickel relations and the loader reject contradictory profiles, including unreachable finding limits, decay outside the `i64` rank domain, and a control-event bound without a stop reserve.

**Rationale:** Runtime values must come from the checked human-authored profile, and no contract-valid profile may wedge planning.

### Decision: Make conformance an oracle-equality port

**Choice:** `campaign-conformance` defines a consumer-mapping port. Where the consumer shell decides something Campaign also decides (execution gate, history load, cache validation), the suite requires the consumer's answer to equal Campaign's answer over enumerated honest and adversarial inputs, and it samples claims, mappings, and draws across their boundary classes.

**Rationale:** Fixed happy-path fixtures let a consumer conform while bypassing the fence or authority boundary.

### Decision: Validate through the held projection once one exists

**Choice:** `ControlPublicationRequest::previous` is a required `&ValidatedControlProjection`: the projection the event was computed from. `plan_control_event_publication` always validates the current branch through `validate_control_advance`. `validate_control_projection` stays the only cold-start entry. `CacheValidationRequest::previous` takes an optional held projection; when one is present, cache validation goes through the same advance check. A `compile_fail` doctest on `ControlPublicationRequest` shows that a publication without a held projection does not compile.

**Rationale:** An optional held projection let a host skip the fast-forward check. That host could then publish post-stop work on a fork jump, or use a fork's cache as current state.

**Code:** `crates/campaign-choregraph/src/contracts.rs` (`ControlPublicationRequest`), `admission/mod.rs` (`plan_control_event_publication`), `fast_forward/mod.rs` (`validate_control_advance`), `freshness/mod.rs` (`CacheValidationRequest`, `validate_cache_against_history`).

### Decision: Enumerate oracle inputs systematically

**Choice:** Conformance generates each oracle input class instead of sampling it by hand.

- Gate matrices, for ranked and exploratory selections, present three classes. The first is every single-field mutation of the published `SelectionDecision` that keeps its `selection_id`. The second is every single-field mutation of honest durable evidence, with each branch fact split into name, head, and generation, and each durability flag cleared. The third is every committed transition of the session, presented for a publication it does not belong to, such as a compare-and-set loser carrying the winner's evidence.
- One shared history catalog holds honest and forged histories. Each entry declares Campaign's answer cold and through each held projection. Load and cache validation both receive every entry, and a `require` check confirms that Campaign's own answer equals the declared answer.
- Promotion claims are checked on planner-computed stops and waits as well as recorded ones.
- Mapping samples cover costs in every declared resource dimension, a multi-dimension cost, units above `u32::MAX`, index draws with pairwise-distinct residues, and guidance whose first levels tie.

**Rationale:** A hand-picked sample lets a shell that checks only the sampled fields conform. The audit rounds found such shells for the gate, load, cache, and mapping paths.

**Code:** `crates/campaign-conformance/src/catalog/fencing/matrix/{tampering,observations}`, `catalog/evidence/histories` (with `lineages/forks`), `catalog/evidence/claims/planned`, `catalog/budgets/reservations`, `catalog/ranking/{levels,tickets}`.

### Decision: Fix one guidance rank shape and rank by a total order

**Choice:** A campaign has one guidance rank-vector length. The first admitted guidance record fixes it. `admit_guidance`, `revise_guidance`, and replay refuse any other length with `InvalidGuidanceShape`. The shape is derived from state, not framed into it, so the state-identity encoding and the golden framing digests are unchanged. The ranked comparator orders candidates by effective ranks, descending and lexicographic over the whole vector, then by ascending candidate identity. Production and the reference planner therefore agree.

**Rationale:** Mixed lengths made the comparison non-transitive. A dominated candidate could win, retiring an unrelated candidate could flip the choice, and sorting more than 20 candidates panicked in planning and replay.

**Code:** `crates/campaign-core/src/frontier/validation/mod.rs` (`validate_guidance_shape`, `guidance_rank_dimensions`), `policy/ranking/mod.rs` (`compare_eligible`).

### Decision: Reserve host control capacity and stop under an adapter-owned reason

**Choice:** The host bounds the control lineage in two ways that core cannot see: the Choregraph projection bound (`maximum_events`) and the branch generation range from the admission's binding up to `u64::MAX`. `plan_control_event_publication` applies the control event first, so Campaign errors take precedence. It then refuses with `ProjectionCapacityExhausted` or `GenerationCapacityExhausted` unless the lineage after the event, plus one settlement per outstanding selection, plus the stop of an open campaign, fits both bounds. The graph's own event limit is a third host bound. The graph-capacity decision below extends the same reserve to it.

`plan_next_from_published_projection` plans against the same effective capacity. Suppose core would select, or wait for guidance or capacity, but no room is left for one more selection, its settlement, and the stop. The adapter then returns `Wait(AwaitingResults)` while selections are outstanding. Otherwise it returns `Stop(ConsumerRequested { reason_id: control_capacity_reason_id() })`, built through core `admit_stop` with the incomplete-search acknowledgement. Settlements and the stop stay publishable at the bound.

The stop is not `ControlEventLimitReached`. Core admits and replays that reason only when `bound_stop_reason` finds `max_control_events` exhausted. Replay never sees the host's projection bound, because it is host configuration and not admission state. A `ControlEventLimitReached` stop under a smaller host capacity would therefore be refused as `InvalidStop` at publication and on every later cold replay. The consumer reason makes no frontier or exhaustive-search claim.

**Rationale:** Without the reserve, a selection could be published as the event that exhausted the host bound. It then executed, but neither its result nor the stop could be published, and the branch no longer loaded.

**Code:** `crates/campaign-choregraph/src/headroom/mod.rs` (`control_capacity_reason_id`, `check_publication`, `plan_within_capacity`), `admission/mod.rs` (`plan_control_event_publication`), `lineage/mod.rs` (`plan_next_from_published_projection`).

### Decision: Bind control envelopes to the admitted identities

**Choice:** Fast-forward checks require the binding's projection schema, adapter, and control-domain identities to equal the admission's projection binding. A mismatch is `Campaign(CrossedIdentity)`. The check covers cold load, advance, publication, and cache validation. `admit_control_root` reconstructs the event first. It refuses another event kind with `MissingAdmission`, a forged admission identity with `MalformedIdentity`, and identities other than the admission's with `CrossedIdentity`.

**Rationale:** The admission of one campaign was accepted as a second control root under foreign envelope identities. That root loaded, published, and executed, which left two valid roots with one state identity.

**Code:** `crates/campaign-choregraph/src/lineage/mod.rs` (`check_fast_forward`), `fast_forward/mod.rs` (`extension_start`), `admission/genesis/mod.rs` (`admit_control_root`).

### Decision: Bind event receipts to the event they certify

**Choice:** `project_event_receipt` requires the presented event to equal `CampaignControlEvent::canonical`, which re-derives every carried identity and orders prune targets and result children canonically. Any other event is `MalformedIdentity`. For `SelectionPublished`, the request profile must equal the event's profile (`CrossedIdentity`), and the receipt binds the event's entropy ticket, guidance record, and cost reservation. Event and refusal receipts refuse reference profiles with `ReferencePolicySelected`. Every receipt projection requires a binding under the admitted control-branch name at generation `admission.projection.generation + control_event_count` (`CrossedIdentity`), and an admitted binding that re-derives (`StaleProjection`).

**Rationale:** One selection event's receipt could name any profile, including a reference profile without a ticket. A prune with forged carried identities received a receipt that replay refused. Receipts also bound a mirror branch or a future generation.

**Code:** `crates/campaign-core/src/model/events/content/mod.rs` (`canonical`), `policy/receipts/mod.rs` (`project_receipt`, `require_state_history`), `policy/receipts/recorded/mod.rs` (`project_event_receipt`, `project_refusal_receipt`).

### Decision: Stop waiting for guidance that the guidance bound cannot admit

**Choice:** Once `guidance_record_count` reaches `max_guidance_records`, no retained candidate counts as awaiting guidance, both for the planner and for `bound_stop_reason`. The planner then reaches a bound stop such as `FrontierExhausted`, and `admit_stop` accepts it. The conformance reference planner mirrors the rule.

**Rationale:** Before this rule, the planner waited for guidance that could never be admitted, and `admit_stop(FrontierExhausted)` was refused as `InvalidStop`. Only a consumer stop could end the campaign.

**Code:** `crates/campaign-core/src/policy/capacity/mod.rs` (`guidance_wait`), `policy/retention/stopping/mod.rs` (`bound_stop_reason`).

### Decision: Complete the conformance generators and choose candidates by suite identity

**Choice:** The generators present the remaining input classes:

- non-selection publications (a stop, a guidance improvement, and a candidate batch), each with its own honest committed evidence, for the published selection, the loser, and an unpublished selection;
- forks at every depth up to `MAX_FORK_DEPTH` beyond the held head, with each catalog entry presented cold and through two held projections;
- cost units `2^33 + d`, `(2^53 | 1) + d`, and `u64::MAX - d` under a `u64::MAX` budget;
- index draws on a `2^63` base with pairwise-distinct residues, and explore draws above `2^32` on both sides of the threshold;
- exploit-side draws whose exploration position differs from the ranked choice, covering every residue;
- a lineage longer than the projection bound, a two-parent merge whose parent choice names neither parent, and a replayed control event;
- a control root under foreign envelope identities.

Every suite-level choice among candidates follows the suite's operation order (`STANDARD_OPERATIONS`), never the adapter's candidate-identity order. Event receipts are requested with the event's own profile.

**Rationale:** Each missing class let a holed shell pass. Choosing by adapter candidate-identity order made the transcript depend on how an adapter labels identities rather than on its behaviour. Two correct adapters could then disagree on the transcript digest.

**Code:** `crates/campaign-conformance/src/catalog/fencing/matrix/non_selection`, `catalog/evidence/lineages/{forks,envelopes}`, `catalog/evidence/histories/shapes`, `catalog/budgets/reservations`, `catalog/ranking/tickets`, `catalog/common` (`STANDARD_OPERATIONS`).

### Decision: Reserve the graph's own event capacity

**Choice:** The host control-capacity reserve also holds against the Choregraph graph's event limit (`HistoryLimits::max_events`). `plan_control_event_publication` refuses with `GraphCapacityExhausted` unless three things fit that limit together: the graph after the event, one settlement per outstanding selection, and the stop of an open campaign. The check runs before Choregraph admission, so the graph never refuses the event with `EventLimitExceeded`. `ValidatedControlProjection` records the graph length and limit (`graph_events`, `graph_max_events`). `plan_next_from_published_projection` plans against the projection bound, the generation range, and the graph limit together. Since round 9, a plan through a projection whose recorded reserve no longer fits is refused with the typed capacity error, in the order projection bound, generation range, graph limit, instead of planning an event that publication would refuse, whenever no control draft is pending (a pending draft answers `PendingControlDraft` first) and core planning accepts the request (core's typed errors answer first); before round 9 only a planned stop was checked this way (see "Reserve every descending draft and recover in chain order" below).

The reserve covers one control event per outstanding selection and the stop. Each such event is a result without children or an unobserved closure. Every pending control draft that descends from the branch head adds its own event and one settlement (see the decisions "Count a pending draft in the exposed reserve" and "Reserve every descending draft and recover in chain order" below). Exploration moments, reported children, and other writers that share the graph, including Campaign campaigns under other envelopes, consume unreserved capacity. Hosts must leave the reserve free and give each campaign its own graph (see "Admit one campaign per graph and envelope" below).

**Rationale:** Under a small graph event limit, a selection was granted and executed, and then the graph refused both its result and every stop. Under the standard limits, the planner planned a `ControlEventLimitReached` stop that publication refused.

**Code:** `crates/campaign-choregraph/src/headroom/mod.rs` (`check_publication`, `plan_within_capacity`, `fits_another_selection`, `check_reserve`), `headroom/limits/mod.rs` (`HostBounds`), `contracts/view/mod.rs` (`ValidatedControlProjection::graph_events`, `graph_max_events`), `admission/mod.rs` (`plan_control_event_publication`).

### Decision: Admit a control root only with room for its stop

**Choice:** Before it admits the draft, `admit_control_root` refuses a root whose lineage leaves no room for its own stop. It checks the graph's own limits in this order:

1. `2 <= max_projection_events`, or `ProjectionCapacityExhausted`.
2. The admission generation plus two without overflow, or `GenerationCapacityExhausted`.
3. `graph.len() + 2 <= max_events`, or `GraphCapacityExhausted`.

`derive_projection_binding` refuses an empty control-branch name, or one longer than the graph's `max_label_bytes`, with `History(InvalidBranchName)`. Such a branch binds nothing at admission, cold load, or advance. Root admission does not see a host `maximum_events` that is smaller than the graph's projection limit. The planner and publication refuse that bound with `ProjectionCapacityExhausted`.

**Rationale:** Three roots loaded and planned a stop that could never be published: one under a projection bound of one event, one at a generation next to `u64::MAX`, and one under a branch name that Choregraph refuses.

**Code:** `crates/campaign-choregraph/src/admission/genesis/mod.rs` (`admit_control_root`), `headroom/mod.rs` (`check_root`), `identity.rs` (`derive_projection_binding`, `check_branch_name`).

### Decision: Make republication of an admitted draft idempotent

**Choice:** Before admission, `plan_control_event_publication` looks up the draft's expected identity (`HistoryGraph::expected_identity`). If the graph already holds an event with that identity and identical content, publication does not admit the draft again. The graph reserve counts the unchanged graph, together with the drafts the graph holds chained below the republished draft (round 9), and the branch plan advances to the existing event. The result equals the original publication in payload, draft, event, branch plan, and control event, over the graph as presented; that graph equals the original publication's graph unless other writers appended to it since. Only the binding's graph and projection identities differ, because the graph now holds the draft. Every other duplicate is refused as Choregraph reports it.

While such a draft is pending, identical republication of a held child of the head is the only publication the branch head accepts. The planner does not re-plan the identical selection: it refuses with `PendingControlDraft` until the host republishes the draft that `pending_control_draft` names or, when that draft cannot complete, the identical control event of another held child of the head (see the pending-draft decision below).

**Rationale:** Suppose a host crashed after Choregraph admitted the event but before the branch moved. Every retry then failed with `History(NoChange)`, and the selection could never be published.

**Code:** `crates/campaign-choregraph/src/admission/mod.rs` (`plan_control_event_publication`).

### Decision: Treat host-capacity decisions as adapter decisions

**Choice:** The capacity transform in `plan_next_from_published_projection` returns either `Wait(AwaitingResults)` or the capacity stop. Neither result has a core plan receipt before publication. `project_receipt` re-plans with core, which cannot see the host bounds, and refuses the transformed plan with `CrossedIdentity`. `project_event_receipt` can project a receipt for the published capacity stop event. A capacity wait publishes nothing and receives no receipt. This decision changes no code. The spec records it as the non-claim `campaign.receipts.host_capacity`, and `risk.md` lists it as a residual risk.

**Rationale:** A re-plannable transform would need the host bounds in core admission state. Replay would then depend on host configuration. The host control-capacity decision above keeps host configuration out of replay.

**Code:** `crates/campaign-choregraph/src/headroom/mod.rs` (`plan_within_capacity`, `capacity_stop`), `crates/campaign-core/src/policy/receipts/mod.rs` (`project_receipt`), `policy/receipts/recorded/mod.rs` (`project_event_receipt`).

### Decision: Validate rank orders against the fixed rank shape

**Choice:** After the first guidance record fixes the rank length, core refuses with `InvalidConfig` any exploration order or retention rank order that names a dimension at or beyond `min(max_rank_dimensions, fixed length)`. Before any guidance exists, only `max_rank_dimensions` applies. Planning validates the profile after its state-currency and request-identifier checks and before its history-binding, request-currency, and entropy checks. Receipt re-planning validates it after its own state-currency, history-binding, and request checks and before it compares the supplied plan. Scheduling re-planning checks each supplied plan and request in turn (profile content, then plan ownership) and then re-plans each request through selection planning. Every path validates the rank order before any recorded stop, bound, or entropy draw, so the refusal never depends on the draw. The rule covers these paths:

- planning, receipt re-planning, and scheduling re-planning;
- replay of `SelectionPublished`;
- `plan_pruning`, and `prune_frontier` with a retention profile;
- replay of `CandidatesPruned`.

**Rationale:** Consider one-level guidance with exploration order `[1]`. An explore draw returned `InvalidGuidanceShape`, and an exploit draw selected a candidate. The planner's answer therefore depended on the draw.

**Code:** `crates/campaign-core/src/policy/validation/mod.rs` (`validate_declared_rank_dimensions`), `policy/decision/checks/mod.rs` (`validate_profile`, `validate_exploration_order`), `policy/retention/planning/mod.rs` (`validate_rank_order`).

### Decision: Keep the reference planner in parity with production

**Choice:** The conformance reference planner fits a cost as production does: `cost <= budget - reserved`, with checked arithmetic, and an unrepresentable remainder does not fit. Like production, it computes exhausted dimensions over every retained candidate, and it waits for guidance under the same rule. It refuses a rank order beyond the fixed shape with `InvalidConfig` before it reads any draw. A missing projected dimension is `InvalidGuidanceShape`. The differential corpus includes both audit states.

**Rationale:** At a `u64::MAX` budget, the reference returned `ArithmeticOverflow` where production stopped with `BudgetExhausted`. Under exploration order `[2]`, the reference selected where production refused. The differential evidence therefore compared two different policies.

**Code:** `crates/campaign-conformance/src/reference/budget/mod.rs` (`blocking_dimensions`, `exhausted_dimensions`, `has_guidable_candidate`), `reference/choice/mod.rs` (`check_rank_order`).

### Decision: Sample conformance inputs at their bounds

**Choice:** The generators present each bounded input class at its bound as well as below it:

- **Forks.** Forks appear at depths 1 to `MAX_FORK_DEPTH`, and also at the greatest depth beyond the held head that the presented projection bound admits. Each fork goes to load and cache validation, cold and through both held projections. The harness projection bound is `HISTORY_MAX_EVENTS = 64`, and the harness graph limit is `GRAPH_MAX_EVENTS = 512`. Round 9 adds depth 0 and presents every depth through every held-matrix holder (see "Present the catalog through every holder and every stop to the claims oracle" below).
- **Settlement order.** Two in-flight selections of one candidate settle in both orders. `results.crossed_selection` settles the older one first. The new negative fixture `results.out_of_order_settlement` settles the newer one first. Each result must settle exactly the executed selection and carry exactly that execution's reserved children. The negative fixture `closures.out_of_order` presents the same two orders for unobserved closures (see the closure-attribution decision below).
- **List lengths.** The suite presents list-valued mappings at length 1, at an intermediate length, and at the largest length the harness can express. Each length runs in its own campaign, and the suite requires exact counts. `selection.level_counts` uses 1, 3, and `min(max_rank_dimensions, 64)` levels. `results.child_counts` uses 1, 3, and `min(max_child_moments, graph room)` children. `results.findings` uses 0, 1, 4, and `min(max_findings, 4096)` findings.
- **Level families.** The positive fixtures `selection.middle_level` and `selection.level_order` present guidance where a middle level decides and where a later level exceeds an earlier one (see the mapping-sampling decision below).

**Rationale:** Each of these shells passed the suite: a resync-on-gap shell with a gap of four, a result mapping keyed to the oldest in-flight selection, and shells that capped rank levels, children, or findings.

**Code:** `crates/campaign-conformance/src/catalog/evidence/lineages/forks/mod.rs` (`dropping`), `lineages/forks/extent/mod.rs` (`maximal_depth`, `depths`), `catalog/evidence/histories/shapes`, `catalog/outcomes/crossed` (`older_first`, `newer_first`), `catalog/outcomes/counts` (`presented_children`), `catalog/outcomes/reports` (`findings`), `catalog/ranking/levels` (`level_counts`), `harness/mod.rs` (`HISTORY_MAX_EVENTS`, `GRAPH_MAX_EVENTS`).

### Decision: Wait for guidance only for a candidate that could be selected

**Choice:** A retained unguided candidate awaits guidance only when two conditions hold: the guidance-record bound still admits guidance, and the candidate's declared costs fit every remaining budget. The decision "Wait for guidance only with room to select" below adds a third condition. `exhausted_dimensions` covers every retained candidate, guided or not. The planner keeps its order:

1. Wait for guidance.
2. Wait for results while work is outstanding.
3. Stop with `FrontierExhausted`, or with `BudgetExhausted` and its sorted blocking dimensions.

`admit_stop` and replay apply the same rule through `bound_stop_reason`.

**Rationale:** An unguided candidate that could never fit the remaining budget kept the planner waiting for guidance forever, and `admit_stop` refused the budget stop.

**Code:** `crates/campaign-core/src/policy/capacity/mod.rs` (`guidance_wait`, `exhausted_dimensions`, `remaining_budget`, `costs_fit`), `policy/decision/bounded/mod.rs` (`plan_without_eligible`), `policy/retention/stopping/mod.rs` (`bound_stop_reason`).

### Decision: Test each typed denial directly

**Choice:** Each of these denials has a direct test that asserts the typed error:

- adapter publication refuses a reference-planned selection with `Campaign(ReferencePolicySelected)`;
- the planner never selects an over-depth candidate, intake refuses it with `CandidateOverDepth`, and replay of a forged selection of it is `MissingCandidate`;
- the infra checker refuses a Weave-named core dependency under `campaign.boundary.history_leak`, the rule that also covers Weave identifiers;
- the result entry at the finding bound returns exactly `BoundExceeded`.

**Rationale:** Before these tests, other paths only implied each denial, or a test asserted that some error occurred without naming it.

**Code:** `crates/campaign-choregraph/tests/integration_repair/publication/mod.rs` (`a_reference_planned_selection_is_refused_by_the_adapter`), `crates/campaign-core/tests/core_policy/lineage/mod.rs` (`an_over_depth_candidate_is_never_selected`), `tools/campaign-infra-checker/src/dependencies/tests.rs` (`rejects_weave_named_crates_as_history_leaks`) with the negative fixture `core-weave-dependency`, and `crates/campaign-core/tests/adversarial/bounds/mod.rs` (`result_entrypoint_must_enforce_the_finding_bound_before_event_publication`).

### Decision: Keep one pending control draft per branch head

**Choice:** A pending control draft is a Campaign control draft in the graph, under the projection's envelope (projection schema, adapter, and control-domain identities), that descends from the presented control-branch head through control-parent links. When this decision landed in round 7, only a draft whose parent is the head counted; since round 9, drafts a pipelining host chained below it and racing sibling drafts count too (see "Reserve every descending draft and recover in chain order" below). `plan_control_event_publication` runs its fast-forward, fence, and Campaign apply checks first. It then refuses any control event whose draft is not a held child of the head with `AdapterError::PendingControlDraft { event_id }`, naming the child of the head first in Choregraph event-identity order, before the host-capacity checks and before admitting anything. A draft chained below another pending draft is refused as well; a selection among them fails its fence check first with `PublicationFenceMismatch`, and every other adapter-built chained control event fails its Campaign apply check first with `StaleState`, whether or not it depends on the uncommitted draft above it, because it names the state after that draft (D91). Identical republication of a held child of the head is still admitted and counts that draft once. `validate_control_projection` and `validate_control_advance` record every descending draft in `ValidatedControlProjection`. `plan_next_from_published_projection` refuses with the same error right after its request-binding check, before core planning, so it never re-plans past a pending draft.

`pending_control_draft(previous, current)` is the recovery entry point. It validates `current` as a fast-forward of the held projection, with that validation's typed errors. It returns `Ok(None)` when no draft is pending. Otherwise it decodes the draft to republish first, the child of the head first in Choregraph event-identity order, into a `PendingDraft` that names its history event, control event, and payload identities. A draft that names a control event the lineage already accepted can never complete and is refused with `Campaign(DuplicateControlEvent)`. The graph holds only digests, so the host looks up the control event in its own journal and republishes it, and publication re-validates payload, fence, Campaign apply, and capacity. Recovery republishes drafts in chain order: once a republished draft's branch update commits, a draft chained below it becomes the child of the new head and is named next. Hosts serialize graph appends per control branch. A graph can still hold several pending drafts on one head, for example from a host that does not serialize appends, a writer outside the adapter, or a graph written before this decision. The decision "Count a pending draft in the exposed reserve" below states how they are named.

`admit_control_root` is idempotent in the same way. If the graph already holds the identical root draft (same expected identity and content) and no other Campaign control event under the admission's envelope, `check_root` counts it once, and admission returns a `ControlRootAdmission` naming the held root event and draft over the graph as presented, equal to the original unless other writers appended. Since round 9, a root on a graph that holds any other Campaign control event under the envelope, including another campaign's root or a later control event of the same campaign, is refused with `SharedControlGraph` (see "Admit one campaign per graph and envelope" below).

**Rationale:** The graph reserve was computed from the publisher's own post-event state. A compare-and-set loser or a stale publisher therefore left its durable draft in the graph and used up the reserve of the lineage the branch adopted, and the granted selection's result, its closure, and every stop were then refused with `GraphCapacityExhausted`. While a stop draft was held, the planner and publication also disagreed at the graph bound. Retrying root admission after a partial commit failed with `History(NoChange)` or `GraphCapacityExhausted`.

**Code:** `crates/campaign-choregraph/src/occupancy/mod.rs` (`scan`), `exclusivity/mod.rs` (`check_none`, `pending_control_draft`, `PendingDraft`), `envelope/mod.rs` (`is_control_child`, `control_event_id`), `admission/mod.rs` (`plan_control_event_publication`, `held_draft`), `lineage/mod.rs` (`validate_control_projection`, `plan_next_from_published_projection`), `fast_forward/mod.rs` (`validate_control_advance`), `admission/genesis/mod.rs` (`admit_control_root`), `headroom/mod.rs` (`check_root`), `contracts.rs` (`AdapterError::PendingControlDraft`).

### Decision: Present conformance forks over earlier history

**Choice:** The history catalog also forks the history before H0's head. That history is captured before H0 is published, so no pending draft exists on it. Forks over it appear at depths 0 to `MAX_FORK_DEPTH` and at the greatest depth that fits beyond the dropped head: `HISTORY_MAX_EVENTS` minus the dropped head's lineage length (59 events for the harness's H0), unless the control-event room or the graph room admits fewer. Each is presented cold, through H0, and through H1, and Campaign answers `Loads`, `NonFastForward`, `NonFastForward`. Forks over H0 keep their answers (`Loads`, `Loads`, `NonFastForward`). The round-7 boundary control was a resync gap of 60. Round 9 replaced it with boundary controls through the held-matrix holders, whose deepest presented forks reach further beyond a short held prefix (see "Present the catalog through every holder and every stop to the claims oracle" below).

Fork fill events are guidance improvements while the guidance-record room lasts, then single-candidate batches while the candidate room lasts. Since round 8 (see "Cover the load and cache oracles by dimension" below), fill then continues with equal-rank coalesced duplicates of a retained candidate's latest guidance record, so the fork depth is limited by the projection bound, the control-event room, the graph room, and, while no retained candidate is or can be guided, the fill's candidate room (`fill :: room`, D99).

**Rationale:** Every fork was built over H0, so a resync shell with a gap of 59 and a shell that went cold when idle (and so reopened a stopped campaign) passed the suite. The maximal fork depth was also capped by the guidance-record room, so resync gaps of 10 to 58 passed under `max_guidance_records = 12`.

**Code:** `crates/campaign-conformance/src/catalog/evidence/lineages/forks/mod.rs` (`Earlier`, `dropping`), `lineages/forks/extent/mod.rs` (`depths`, `maximal_depth`), `lineages/forks/fill/mod.rs` (`event`, `improvement`), `catalog/evidence/histories/mod.rs` (`prepared`, `before_head`), `histories/shapes/forked/mod.rs` (`forks`, `behind_both`), `catalog/common/mod.rs` (`guided_frontier_after`).

### Decision: Attribute each closure to its named selection

**Choice:** The negative fixture `closures.out_of_order` closes two in-flight selections of one candidate, first older-first and then newer-first in a fresh campaign. It requires the named selection to be closed and the other to stay outstanding. The harness `Session::close` refuses a closure whose `selection_id` differs from the named selection with `ForbiddenAccepted { CROSSED_CLOSURE }`.

**Rationale:** Closures were never checked for which selection they closed, so a closure mapping keyed by candidate passed the suite.

**Code:** `crates/campaign-conformance/src/catalog/budgets/mod.rs` (`out_of_order`, `close_in_flight`, `close_exactly`), `harness/settlement/mod.rs` (`CROSSED_CLOSURE`, `Session::close`).

### Decision: Check root packages, exempt members, and dependency sources in the infra checker

**Choice:** The infra checker enforces three more rules:

- A root `[package]`, or a legacy root `[project]`, in the workspace manifest is refused under `campaign.boundary.shared`. It would be an implicit workspace member with no allowlist or source profile. The repository has no root package.
- Exempt members go through `exempt_rule`. Random families (`rand`, `getrandom`, `fastrand`) are `campaign.functional_core.effect`, and Weave, Universal, and Choregraph families are `campaign.boundary.history_leak`. Consumer families and allowlists still do not apply to exempt members.
- `dependencies::origin` checks the source of every declared dependency in every member, exempt members included, after workspace inheritance. It admits a registry release without `git`, `path`, `registry`, or `registry-index` keys; a path that resolves lexically to an admitted member directory whose package name matches; and `git` only for a Choregraph-family package at exactly the inventoried pin repository and revision. Anything else, including an alternate registry and any `[patch]` or `[replace]` entry, is `campaign.boundary.history_leak`. Core's allowlisted `blake3` must come through parsed workspace inheritance, not a matching comment.

The negative fixtures are `root-package`, `exempt-weave-dependency`, `exempt-random-dependency`, `core-worktree-dependency`, `profile-git-dependency`, and `workspace-patch-override`.

**Rationale:** A root package depending on `universal-weave` through a sibling path, on `rand`, and reading `SystemTime::now` passed every boundary rule. The exempt checker crate could depend on Weave through a sibling path, and core could take its allowlisted `blake3` from a Weave worktree path behind a commented inheritance line.

**Code:** `tools/campaign-infra-checker/src/validation/mod.rs` (`reject_root_package`, `require_inherited`), `dependencies/mod.rs` (`exempt_rule`), `dependencies/origin.rs` (`validate_root`, `validate_member`).

### Decision: Sample middle levels, level order, and decoded tickets

**Choice:** Two positive fixtures sample guidance mappings by level position:

- `selection.middle_level` uses three-level scores, or the most levels the profile admits. `[5, 9, 1, ..]` must beat `[5, 1, 9, ..]`, and the other candidate wins after its middle level improves to 10. The fixture is skipped when `max_rank_dimensions < 3`, because no middle level exists.
- `selection.level_order` requires `[1, 30]` against `[20, 3]` to choose the second candidate, and the first after it improves to `[25, 31]`.

Every entropy ticket the adapter issues, including the fencing matrix's stale rivals, is decoded. Its `explore_draw()` and `index_draw()` must equal the scenario's draws exactly, or the fixture reports `Diverged { EXACT_DRAWS }`. `entropy.wide_draws` also presents seven eligible candidates, admitted in batches of three, across every residue modulo 7.

**Rationale:** Mappings that zeroed the middle levels or sorted levels in descending order passed under both profiles. Tickets were never decoded and at most three candidates were eligible, so a mapping that reduced draws to parts per million passed.

**Code:** `crates/campaign-conformance/src/catalog/ranking/families/mod.rs` (`middle_level`, `level_order`, `decides`), `harness/decisions/entropy/mod.rs` (`EXACT_DRAWS`, `decoded`, `Session::ticket`), `catalog/fencing/matrix/mod.rs` (`stale_rival`), `catalog/ranking/tickets/mod.rs` (`explore_each`, `add_candidates`, `wide_draws`), `catalog/common/mod.rs` (`ordered_levels`).

### Decision: Run the reference planner in production order

**Choice:** `reference::plan_next` checks in production order: profile validation, the decay's bounds domain, the rank order, stale state and stale projection, entropy, the recorded stop, the selection bound, the control-event headroom (`control_event_count + outstanding + 3 <= max_control_events` for a selection), the finding limit, and then eligibility. Its guidance wait follows the room-to-select rule below. Each state the audit found joins `tests/differential`.

**Rationale:** The reference planner had no control-event rule, returned a recorded stop before checking entropy, and skipped profile and decay-domain validation. The differential evidence therefore compared two different policies.

**Code:** `crates/campaign-conformance/src/reference/mod.rs` (`plan_next`, `has_room`, `plan_at_limits`, `without_eligible`), `reference/budget/mod.rs` (`has_guidable_candidate`).

### Decision: Wait for guidance only with room to select

**Choice:** A candidate awaits guidance only if its guidance event, a selection, that selection's settlement, and the stop all still fit. In core, three conditions must hold: the guidance-record bound admits a record, the candidate's costs fit, and `control_event_count + outstanding + 4 <= max_control_events`. When only that room is missing and no candidate is eligible, the planner returns `AwaitingResults` while a selection is outstanding and otherwise `Stop(ControlEventLimitReached)`, which precedes the frontier and budget stops. `admit_stop` agrees through `bound_stop_reason`. A used-up guidance-record bound still yields the frontier or budget stop. The conformance reference planner mirrors the rule. The adapter's host capacity keeps a guidance wait only if the guidance event, a selection, its settlement, and the stop fit the host bounds, and otherwise answers `AwaitingResults` or the capacity stop.

**Rationale:** At `control_event_count + outstanding + 3 == max_control_events`, the planner awaited guidance and `admit_stop(ControlEventLimitReached)` was refused as `InvalidStop`. Once the candidate was guided, the planner stopped with `ControlEventLimitReached`, so the candidate was never selectable.

**Code:** `crates/campaign-core/src/policy/capacity/mod.rs` (`guidance_wait`), `frontier/reserve/mod.rs` (`ControlEventEffect::GUIDED_SELECTION`), `policy/decision/bounded/mod.rs` (`plan_without_eligible`), `policy/retention/stopping/mod.rs` (`bound_stop_reason`); `crates/campaign-conformance/src/reference/mod.rs` (`without_eligible`); `crates/campaign-choregraph/src/headroom/mod.rs` (`events_before_selection`, `fits_another_selection`, `bounded_plan`).

### Decision: State the typed answers the code gives in the spec

**Choice:** The delta spec states the typed answers the code gives where the audit executed a contradiction: the refusal order of `campaign.guidance.declared_order`, `BoundExceeded` for over-long guidance in `campaign.guidance.shape`, coalesced duplicates admitted at the guidance bound (`campaign.guidance.invalid`, `campaign.guidance.coalesced`), the earlier checks' errors for settled-selection duplicates (`campaign.observations.duplicate`), and what core and the adapter each check for crossed histories and stop receipts (`campaign.receipts.crossed`). The GIVENs of `campaign.selection.valid`, `campaign.wait.guidance`, `campaign.wait.capacity`, and `campaign.wait.guidance_exhausted` name the earlier checks. New scenarios cover the pending-draft and room-to-select decisions: `campaign.initial_admission.root_retry`, `campaign.publication_fence.pending`, `campaign.publication_fence.pending_recovery`, and `campaign.wait.guidance_room`. This decision changes no code.

**Rationale:** Five amended scenarios contradicted the typed answers the code returns, and several GIVENs omitted checks that come first.

**Code:** None. The text is in `specs/campaign/spec.md`.

### Decision: Apply each event through sorted lookups

**Choice:** Every per-event lookup of children, payloads, discoveries, findings, candidates, budgets, settled and outstanding selections, and event identities is a binary search over the canonical sorted state through `sorted_position`. `record_observation` searches the sorted state as it was before the result and sorts the new entries once. Sibling payload uniqueness is checked by sorting and deduplicating. `reserve_costs` inserts at the sorted position. The canonical sort also orders admission roots, budgets, and schema identities. `apply_candidates` refuses a record count that differs from the number of admitted outcomes with `MalformedIdentity` before it validates any record. The public lookup accessors on `FrontierState` assume canonical order.

**Rationale:** Applying one result was quadratic in its findings, because each lookup was a linear scan. That contradicted the linear per-event claim.

**Code:** `crates/campaign-core/src/identity/collection.rs` (`sorted_position`), `frontier/settlement/mod.rs` (`record_observation`, `record_children`, `record_findings`), `frontier/dispatch/mod.rs` (`reserve_costs`), `frontier/admission/mod.rs` (`apply_candidates`).

### Decision: Check state identity first in refusal receipts

**Choice:** `project_refusal_receipt` checks the state identity first and returns `StaleState`. The shared `require_state_history`, used by the plan, event, and refusal receipts, returns `MissingAdmission` for a state with zero control events before it reads the history binding.

**Rationale:** The refusal receipt checked the history binding before the state identity, and a caller-built state with zero control events panicked a debug build.

**Code:** `crates/campaign-core/src/policy/receipts/mod.rs` (`require_state_history`), `policy/receipts/recorded/mod.rs` (`project_refusal_receipt`).

### Decision: Count a pending draft in the exposed reserve

**Choice:** `validate_control_projection` and `validate_control_advance` count a pending control draft as a selection: its own lineage event plus one settlement. That is the largest demand one control event makes, and the graph holds only the draft's digests, so its kind cannot be read. Since round 9 this holds for every uncommitted draft that descends from the head. The recorded reserve is this demand, plus one settlement per outstanding selection, plus the stop of an open campaign. `ValidatedControlProjection` exposes it through three accessors:

- `reserved_control_events`: lineage events still owed, a count that is not kept within the host projection bound or generation range;
- `reserved_graph_events`: the same, less the pending drafts' own events, which the graph already holds;
- `graph_room`: `graph_max_events - graph_events - reserved_graph_events`, saturating at zero. This is what other writers may still append: a published settlement or stop of the campaign draws on the reserve and leaves `graph_room` unchanged, while any other new control event takes one event of it and a new selection two.

The planner's capacity checks (`after_more`, `fits_another_selection`, and, since round 9, `check_reserve`) start from the same demand (`reserve::held`). Publication and republication still check the actual demand of the draft they admit. For a host that publishes only on durably committed branch values that demand never exceeds the counted reserve; a pipelined chain can exceed it (round 10, see "Publish control events only on a durably committed branch value"). When this decision landed in round 8, no error kind, error order, or publication or planning outcome changed for existing callers. Round 9 changed one planning outcome: once the recorded reserve no longer fits, a plan that core planning accepts through a projection without a pending draft is refused with the capacity error instead of waiting for results.

Hosts must republish a pending draft, the one `pending_control_draft` names, before they plan, publish, or append any other graph event. They must also leave `graph_room` free. `graph_room` describes the graph as it was validated, so hosts re-validate after every append. Suppose a host appends beyond the room anyway. While the draft is still pending, republishing it fails with a capacity error (`ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or `GraphCapacityExhausted`, checked in that order), and the other publications fail with `PendingControlDraft`. If the draft's branch update has already committed, the result, the closure, and the stop fail with the capacity error, and since round 9 so does every plan through the projection that core planning accepts.

If several pending drafts descend from one head, the planner, every refused publication, and `pending_control_draft` name the child of the head first in Choregraph event-identity order, because `HistoryGraph::events` is a `BTreeMap`; since round 9, drafts chained below it are named in chain order as each republication commits. Republishing the identical event of any held child of the head is still allowed. A draft that the host cannot complete blocks the head unless the host completes another held child of the head. Examples are a forged or foreign child under the projection's envelope whose payload is not its named event, a lost journal, and a lowered bound; a Campaign child under another envelope is another writer, not a pending draft. Every later call then returns `PendingControlDraft` or the capacity error, without mutation. No abandonment API exists, and `risk.md` records the residual.

**Rationale:** Any projection validated while a draft was pending left that draft out of its reserve and exposed no room. A host that left the visible reserve free took the pending selection's settlement slot. This happened at a graph limit of 7, and at a limit of 10 after a crashed second selection. Republication, the result, the closure, and the stop then failed with `GraphCapacityExhausted`, or with `PendingControlDraft` forever. An executed selection was stranded, and after a crash the stop could never be recorded.

**Code:** `crates/campaign-choregraph/src/headroom/reserve/mod.rs` (`demand`, `held`, `record`), `contracts/view/mod.rs` (`ValidatedControlProjection::reserved_control_events`, `reserved_graph_events`, `graph_room`), `headroom/mod.rs` (`after_more`, `fits_another_selection`, `check_reserve`), `occupancy/mod.rs` (`scan`), `exclusivity/mod.rs` (`pending_control_draft`), `admission/mod.rs` (`plan_control_event_publication`).

### Decision: Cover the load and cache oracles by dimension

**Choice:** After the shared catalog, `history.load_matrix` and `caches.validation_matrix` continue the campaign past H1: a result with one finding, a second selection, an unobserved closure, and then a stop. This gives a 10-event honest lineage; since round 9 the continuation stops with two selections outstanding and settles one after the stop, which gives 13 events. The held matrix holds every prefix of that lineage, so it covers every held length and every holder kind: idle, outstanding selection, settled by result, settled by closure, and stopped. Through each prefix it presents four shapes, which round 9 extends (see "Present the catalog through every holder and every stop to the claims oracle" below):

- the honest head, a fast-forward that loads;
- every rewind of depth 1 to the prefix length minus one;
- forks of the history before that prefix's head at depths 0, 1, and 2 (round 9: 0 to 3 and the deepest admitted depth), with every head kind the fork state admits;
- through the root, another campaign's root.

Fork heads have six kinds (`HeadKind`): stop, guidance, candidate batch, selection, result, and closure. The catalog forks over H0 and over the history before H0's head also cover every head kind, at depths 0 to `MAX_FORK_DEPTH` and at the maximal depth. A head that needs room for more than its stop is clamped to the maximal depth less that room.

The fixture enforces a covering rule. Every holder kind must meet a refused rewind and a refused fork of each head kind, and every held length and every fork depth must meet a refusal. Otherwise the fixture fails `Diverged`. Once the guidance-record and candidate room is used, fork fill continues with equal-rank coalesced duplicates, so `maximal_depth` is limited by the projection bound, the control-event room, the graph room, and, while no retained candidate is or can be guided, the fill's candidate room (`fill :: room`, D99). The holders are Campaign's cold projections of each prefix under the final graph.

**Rationale:** Forks were presented only through H0 and H1 and ended only in stops, and the fill room capped the maximal fork depth. Under both profiles, several shells passed the suite: a shell that went cold once the held projection was stopped, settled, or longer than six events; a two-event rewind resync; a shell that stayed cold unless the head was a stop; and a resync gap of 24 under a valid `max_candidates = 8` profile. The larger matrix raised a healthy `run_suite` in debug from about 1.1 s to about 1.8 s per pilot.

**Code:** `crates/campaign-conformance/src/catalog/evidence/histories/held/mod.rs` (`matrix`, `HolderKind`, `ShapeKind`), `held/lineage/mod.rs` (`bases`, `holders`), `held/shapes/mod.rs` (`presented`), `histories/shapes/forked/mod.rs` (`forks`), `lineages/forks/mod.rs` (`dropping`), `lineages/forks/extent/mod.rs` (`depths`, `maximal_depth`), `lineages/forks/heads/mod.rs` (`HeadKind`), `lineages/forks/fill/mod.rs` (`event`).

### Decision: Present every planned wait and stop to the claims oracle from both request kinds

**Choice:** `claims/planned` plans every planner wait and stop for a ranked request and for an exploratory request with a ticket. These are frontier exhausted, awaiting guidance, awaiting results, budget exhausted, the selection limit, the control-event limit, and the finding-limit policy stop. The exploratory receipt must bind its ticket. `non_selection_overclaim` adds an exploratory wait receipt and an exploratory receipt of the recorded stop. `claims/duplicates` adds a same-batch duplicate refusal receipt, for one operation named twice, beside the admitted-candidate refusal. The round-8 fixture counted these presentations in a `NON_EVENT_RECEIPTS` constant; round 10 removed it when the claims fixtures became witnessed sessions that check each presented receipt through `require_unclaimed` (see "Present every receipt of every claims session in every later state").

**Rationale:** Two shells passed. One overclaimed only on ticketed wait and stop receipts, and the other only on same-batch refusal receipts.

**Code:** `crates/campaign-conformance/src/catalog/evidence/claims/mod.rs` (`non_selection_overclaim`, `require_unclaimed`), `claims/witness/mod.rs` (`witnessed`), `claims/planned/mod.rs` (`unstopped_receipts`), `claims/planned/limits`, `claims/duplicates`.

### Decision: Check Cargo configuration, lock sources, compiled files, and dependency spellings

**Choice:** The infra checker covers every input Cargo compiles from. Round 9 widened several of these rules; each bullet states the rule as it stands, and the round-9 decisions below give the details.

- **Cargo configuration.** Round 8 read `.cargo/config.toml` and `.cargo/config` at the root and in every member directory and refused `paths`, `[patch]`, `[source]`, and nested `rustflags` or `rustdocflags`. Since round 9 any such file anywhere under the root, outside the checker's fixture roots, is refused whatever it contains, and so is a toolchain-file `path` (see "Allow no Cargo configuration in the repository" below). The rule is `campaign.boundary.history_leak`.
- **Lock sources.** Every `Cargo.lock` package must come from the crates.io registry (Git or sparse spelling), a workspace member, or the pinned Choregraph revision. Anything else is `campaign.boundary.history_leak`.
- **Compiled files.** Each member's compiled file set comes from its manifest: the `[lib]` root, every `[[bin]]` root, and the build script (`package.build`, or an implicit `build.rs`). A root outside `<member>/src` is refused under `campaign.functional_core.effect`. The one exception is `<member>/build.rs`, which is scanned under the member's source profile. Round 8 derived this set for non-exempt members only; since round 9 it is derived for every member, and `[[example]]`, `[[test]]`, and `[[bench]]` targets must lie inside the member (see "Keep every compiled target inside its member" below).
- **Source indirection.** `#[path]`, `include_bytes!`, `include_str!`, `env!`, and `option_env!` are refused in core, adapter, and conformance sources, test code included. `include!` under any path, such as `core::include!`, is refused in every member. Since round 9 these are refused as identifiers in every token tree (see "Refuse effect identifiers in every token tree" below).
- **Names, tables, and features.** Family names treat `-` and `_` as equal. Underscore tables (`build_dependencies`, `dev_dependencies`) and `target.*` tables are read. Allowlisted core dependencies keep their default feature set; since round 9 they are declared once, in the workspace, without features (see "Declare allowlisted core dependencies once" below).
- **Identifier scan.** A `#[cfg(test)]` gate ends at its member's `,` or at its item body. A `tests.rs` file is a test module only when it is declared under `#[cfg(test)]`. The conformance crate's families include Universal Weave.

**Rationale:** A `paths` override built `campaign-choregraph` against an ambient Choregraph worktree with `--locked` and an unchanged `Cargo.lock`, and the checker exited 0. The same held for `[patch]`, `[source]` replacement, and `rustflags --extern`. Core code outside the scanned set also passed. The audit compiled a `[lib]` path outside `src`, a `build.rs` clock read consumed through `env!`, `#[path]` into a Weave worktree, `core::include!`, `include_bytes!`, and `option_env!`. Shape gaps passed as well: `rand_core` in the exempt checker, an underscore build-dependency table, and blake3's file-reading `mmap` feature. The identifier-scan gaps were P3.

**Code:** `tools/campaign-infra-checker/src/dependencies/configuration.rs` (`validate`), `dependencies/lock.rs` (`validate`), `rustscope/targets.rs` (`derive`), `rustscope/mod.rs`, `patterns/effects.rs`, `patterns/gating.rs` (`collect`), `patterns/tokens.rs`, `dependencies/names.rs` (`in_family`), `dependencies/mod.rs`, `dependencies/features/mod.rs` (`validate`).

### Decision: Refuse caller-built states with repeated keys and search sorted keys in loops

**Choice:** `assert_state_identity_current` first runs its existing canonical-order and identity comparison. It then refuses with `StaleState` any keyed vector whose keys are not strictly increasing (`assert_keys_strictly_increasing`). The check covers the admission's roots, resource budgets, and operation schema ids. It also covers the state's candidates, outstanding selections, accepted results, discoveries, known moments, settled selections, reservations, payload bindings, findings, event ids, and reserved resources. Two further guards apply:

- `validate_observation` refuses a result already in `accepted_results` with `DuplicateResult`;
- `apply_control_event` refuses with `StaleState` any output that would repeat a key.

The remaining in-loop scans are now sorted lookups. `planned_targets` and `check_prune_targets` binary-search `outstanding_candidate_ids`. `supports_operation_schema` binary-searches the canonical schema order. `exhausted_dimensions` keeps its list sorted as it inserts.

**Rationale:** A caller-built state with repeated keys and a re-derived identity was accepted. A state that already listed a result as accepted panicked a debug build in `record_observation`. The prune, intake, and budget-stop paths still scanned linearly inside loops.

**Code:** `crates/campaign-core/src/policy/validation/mod.rs` (`assert_state_identity_current`, `assert_keys_strictly_increasing`), `identity/collection.rs` (`strictly_increasing_by_key`), `frontier/settlement/checks/mod.rs` (`validate_observation`), `frontier/replay/mod.rs` (`apply_control_event`), `frontier/mod.rs` (`outstanding_candidate_ids`), `policy/retention/planning/mod.rs` (`planned_targets`), `policy/retention/mod.rs` (`check_prune_targets`), `model/admission/mod.rs` (`supports_operation_schema`), `policy/capacity/mod.rs` (`exhausted_dimensions`).

### Decision: Align spec wording with the code and sample interior levels, wide draws, and request checks

**Choice:** The delta spec states the answers the code gives:

- `campaign.stop.control_event_limit`: `admit_stop` validates state bounds and the frontier only. A reached finding limit is the planner's `PolicyRequested` stop, and `admit_stop` still accepts `ControlEventLimitReached` there.
- `campaign.publication_fence.republication` and `campaign.initial_admission.root_retry`: a retry yields the same publication over the graph as presented. That graph equals the original unless other writers appended. The impossible "same identity, other content" GIVEN is dropped.
- `campaign.receipts.crossed` and `campaign.history_projection.caller_binding`: core receipts project another head or graph at the right generation as supplied, and only the adapter refuses it.
- `campaign.guidance.invalid`: an equal-rank revision at a used-up guidance bound is `BoundExceeded`.
- `campaign.guidance.non_improving`: this now covers only another identity with different ranks, which is consistent with `campaign.guidance.coalesced`.
- `campaign.guidance.declared_order`: receipt and scheduling re-planning return `InvalidConfig` on a stopped campaign.

The spec adds `campaign.publication_fence.{pending_reserve,pending_several,pending_blocked}` for the pending-draft reserve decision. It also states the checker coverage above in `campaign.boundary.history_leak` and `campaign.functional_core.effect`.

Conformance samples four more classes:

- **Interior levels.** The positive fixture `selection.interior_level` decides at the first interior level, the middle level, and the second-to-last level. It does so for 4-level scores and for the most dimensions the profile admits.
- **Wide explore draws.** Explore draws of at least `2^53` whose nearest binary64 value crosses the threshold (`beyond_binary64`) feed `entropy.wide_draws` and `entropy.exploit` wherever such a draw exists.
- **Findings in one campaign.** `results.findings` settles two finding-bearing results in one campaign and requires distinct identities across them.
- **Reference request checks.** `reference::plan_next` runs production's request checks first, in production order (`check_request`): the re-derived state identity, the request-id kind, the profile, the decay, the rank order, binding validation, and the envelope identities.

**Rationale:** Six scenario clauses contradicted the code, or contradicted each other. A shell that zeroed interior levels passed the suite. So did one that mapped explore draws through binary64, and one that keyed findings by position. The reference planner skipped four request checks that production applies.

**Code:** `specs/campaign/spec.md`; `crates/campaign-conformance/src/catalog/ranking/families/mod.rs` (`interior_level`), `catalog/ranking/tickets/draws/mod.rs` (`beyond_binary64`, `binary64`), `catalog/ranking/tickets/mod.rs` (`wide_draws`, `exploit`), `catalog/outcomes/reports/mod.rs` (`findings`, `SAME_CAMPAIGN`), `reference/mod.rs` (`check_request`).

### Decision: Record consumer pilot evidence against pinned suite constants

**Choice:** `provenance/consumers.ncl` records every consumer pilot that runs the shared conformance suite. The record's contract is `ConsumerRecord` in `provenance/contracts.ncl`, and it is exported to `provenance/generated/consumers.json`; `scripts/check-boundaries.sh` typechecks the record and compares a fresh export with the committed one. The record holds two pilots:

- the in-repo Animus-shaped fixture;
- the migrated ChaosControl consumer.

For each pilot, the record holds the fixture count, the default-profile and reordered-profile transcript digests, and the command that produced them. One reordered profile, `exploration_order = [1]` and `retention_rank_order = [1]`, applies to the whole record. For ChaosControl the record also holds these values:

- the adoption commit;
- the Cargo and Nix Campaign pins, which must be equal full revisions;
- the Choregraph pin;
- the parity corpus tests and a parity receipt with its BLAKE3 hash;
- the agreeing decision classes;
- the KVM smoke record and receipt with their BLAKE3 hashes, and the smoke bounds;
- the retained product authority;
- the non-claims.

`crates/campaign-conformance/src/tests/consumers/mod.rs` declares `PINNED_FIXTURE_COUNT`, `PINNED_TRANSCRIPT_DIGEST`, and `PINNED_REORDERED_DIGEST`. Its test `both_healthy_pilots_produce_the_pinned_fixture_count_and_digests` requires `fixture_names().len()` and both healthy Campaign pilots to produce exactly these values. `campaign-infra-checker boundaries` parses that source file with `syn`. It admits each constant only as one top-level, private, non-generic `const` with a plain literal: a positive decimal `usize`, or a lowercase 64-hex `&str`. It then applies these rules:

- The record's reordered profile must be `[1]`/`[1]`.
- Every pilot's fixture count and digests must equal the constants (`campaign.consumer_pilots.valid`).
- The migration sections must satisfy `campaign.chaoscontrol_migration`, `.parity`, and `.authority`.

The consumer check runs last in `validate_boundaries`, so it reports a stale claim only after every other boundary check has passed. A suite or planner change therefore fails the conformance test until the constants are re-pinned. A record that is not refreshed with the new constants fails the checker.

The test does not read the record through `include_str!`. That macro is a compile-time file read, which the checker refuses in conformance sources, test code included (see the checker decision above). Decoding the record would also add a JSON dependency outside the conformance allowlist. The cross-check therefore lives in the `std` checker, which reads both files, and the test pins only literals.

**Rationale:** Consumer evidence had been prose in `verification.md`, and no test pinned the transcript digests. A consumer's recorded digest could therefore drift from the suite without any gate noticing.

**Code:** `provenance/consumers.ncl`, `provenance/contracts.ncl` (`ConsumerRecord`), `crates/campaign-conformance/src/tests/consumers/mod.rs`, `tools/campaign-infra-checker/src/consumers/mod.rs` (`validate`, `validate_migration`), `consumers/evidence.rs` (`validate`, `parse`, `require_profile`, `require_claim`), `validation/mod.rs` (`validate_boundaries`).

### Decision: Admit one campaign per graph and envelope

**Choice:** Binding decision D82. The host-capacity reserve accounts for one campaign per graph and envelope (projection schema, adapter, and control-domain identities). `admit_control_root` refuses a root on a graph that holds any Campaign control event under the admission's envelope other than the identical root draft. It refuses after its identity checks and before its stop-reserve checks, with `AdapterError::SharedControlGraph { event_id }` naming the first such event in Choregraph event-identity order, and leaves the graph unchanged. Cold validation and advance refuse a graph that holds another control root under the envelope with the same error, after their lineage, payload, fence, Campaign replay, and older-graph checks, so no reserve is read and nothing is planned or published through it. Publication, `pending_control_draft`, and cache validation with a held projection all go through advance. Campaigns under other envelopes are other writers of the graph that the reserve does not see, so hosts MUST give each campaign its own graph, and sharded campaigns use separate graphs.

**Rationale:** The reserve and `graph_room` were computed per campaign. A second campaign's root, admitted into the first campaign's graph, booked the same room twice: after the first campaign stopped, every stop the second tried was `GraphCapacityExhausted` forever, and another campaign's stop stranded an executed selection (Audit9 finding 1). Validation requires lineage index 0 to be a parentless `CampaignAdmitted` event, so each campaign has exactly one control root, and refusing every other root under the envelope leaves no second holder of the reserve.

**Code:** `crates/campaign-choregraph/src/occupancy/mod.rs` (`check_root_admission`, `scan`), `admission/genesis/mod.rs` (`admit_control_root`), `lineage/mod.rs` (`validate_control_projection`), `fast_forward/mod.rs` (`validate_control_advance`), `contracts.rs` (`AdapterError::SharedControlGraph`).

### Decision: Reserve every descending draft and recover in chain order

**Choice:** Binding decision D83. The reserve counts every uncommitted Campaign control draft under the envelope that descends from the presented branch head through control-parent links: a draft pending on the head, drafts a pipelining host chained below it before its branch update committed, and sibling drafts that racing publishers left on the head. `occupancy::scan` finds them in one parent-to-children pass, and each counts as a selection, its own event plus one settlement. The planner, every `PendingControlDraft` refusal, and `pending_control_draft` name the draft to republish first: the child of the head first in Choregraph event-identity order. Recovery republishes drafts in chain order. Once a republished draft's branch update commits, the draft chained below it becomes the child of the new head and is named next. Republication checks the demand of the republished draft together with the drafts the graph holds chained below it (`chained_below`). Of racing siblings whose joint demand exceeds the host capacity, only a draft whose actual demand still fits republishes, so the adapter grants no selection that cannot settle. The planner requires the recorded reserve to fit on every plan that reaches that check (`check_reserve`), in the order projection bound, generation range, graph limit, because every control event adds one lineage and one graph event and settles at most one reserved event. The check runs after the request-binding check, the pending-draft check (`PendingControlDraft`), and core planning, whose typed errors answer first, and before the host-capacity transformation of the core decision (`bounded_plan`); no decision is returned while the reserve does not fit, whichever error answers. (Round 11 found that this text placed `check_reserve` right after the pending-draft check while the code runs core planning between them; D98 aligned the text with the code.) `graph_room` bounds only other writers' appends. Since round 10 (D91), chain-order recovery is guaranteed only for drafts that a host keeping the durable-base rule leaves (a partial commit and racing siblings); a pipelined chain is counted conservatively and can be refused at a bound (see "Publish control events only on a durably committed branch value").

A Campaign draft that no control-parent chain links to the head can never enter the lineage, because payload validation requires a control-parent chain, so it is another writer. A branch compare-and-set that a host commits outside the adapter can still strand a racing sibling's selection. The serialization MUST of `campaign.publication_fence.pending_recovery` covers that case, and `risk.md` records it.

**Rationale:** Only the first child of the head was reserved. A host that published a second selection on the first selection's planned branch value before that compare-and-set was durable left the chained draft outside the exposed reserve. A writer that then filled the exposed room blocked the first selection's settlement and the stop forever (Audit9 finding 2). Two drafts raced from one held projection stranded the compare-and-set winner (finding 12). The planner also answered `Wait(AwaitingResults)` while every settlement was `GraphCapacityExhausted`. Counting both racing siblings is conservative: at most one can commit, so the exposed room can be lower than necessary.

**Code:** `crates/campaign-choregraph/src/occupancy/mod.rs` (`scan`, `chained_below`, `Pending`), `headroom/reserve/mod.rs` (`record`, `held`, `demand`), `headroom/mod.rs` (`check_publication`, `check_reserve`, `plan_within_capacity`), `admission/mod.rs` (`plan_control_event_publication`), `exclusivity/mod.rs` (`pending_control_draft`), `contracts/view/mod.rs` (`ValidatedControlProjection::graph_room`, `reserved_control_events`, `reserved_graph_events`).

### Decision: Never fast-forward to an older graph

**Choice:** Binding decision D84. `validate_control_advance` refuses with `AdapterError::StaleGraph` a presented graph that holds fewer events than the held projection recorded, or that lacks a draft the held projection recorded as pending. The check runs after the fast-forward, payload, fence, and Campaign replay checks and before the shared-graph check. Publication, `pending_control_draft`, and cache validation with a held projection all advance through it, and the planner reads only the held projection. A cold load of an older graph still validates, because it has no earlier projection to compare with.

**Rationale:** Advance compared lineages only, so it accepted a lagging graph snapshot that lacked the pending selection draft. A stop published over that snapshot merged with the draft, and once the selection won, its result, closure, and stop were `GraphCapacityExhausted` at a graph limit of 7 (Audit9 finding 12). The held projection records non-draft events only as a count, so a graph of equal or greater length that lacks some foreign non-draft event is not detected. Such an event does not affect the reserve.

**Code:** `crates/campaign-choregraph/src/fast_forward/mod.rs` (`validate_control_advance`), `contracts.rs` (`AdapterError::StaleGraph`).

### Decision: Refuse effect identifiers in every token tree

**Choice:** Binding decision D85. The infra checker reads every token tree of each file, with raw identifiers normalised: attributes and their arguments, such as `#[doc = ...]` and `cfg_attr`, macro arguments, and macro bodies. In core, adapter, and conformance sources, including their test code and build scripts, it refuses these:

- the identifiers `include`, `include_bytes`, `include_str`, `env`, and `option_env`, anywhere;
- any `path` attribute, whether direct, inner, nested in `cfg_attr`, or inside a macro body;
- any `macro_rules!` definition;
- any `use` rename other than `as _`.

In every member, exempt members included, it refuses the bare identifier `include`. Every such refusal reports `campaign.functional_core.effect`. None of these tokens occurs in the real sources.

**Rationale:** The round-8 refusals matched a macro only by its own last path segment, by a `name !` token pair, or by `#[path]` on a module that `syn` parses. A `macro_rules!` body is an unparsed token stream, and an `$m:ident` argument is never followed by `!`. `include!`, `#[path]`, and the effect macros reached through macro bodies, ident arguments, `#[doc = include_str!(..)]`, and renamed imports therefore passed (Audit9 finding 3, and the attribute item of finding 13). A macro cannot be invoked or renamed without naming its identifier, so refusing the identifiers in every token tree closes the class. A `use .. as` rename of a macro cannot be told apart from other renames syntactically, so the shared crates refuse every rename except `as _`.

**Code:** `tools/campaign-infra-checker/src/patterns/indirection.rs` (`find_in`, `Finding`, `ANY_MEMBER`, `SHARED`), `rustscope/mod.rs`.

### Decision: Allow no Cargo configuration in the repository

**Choice:** Binding decision D86. The checker visits every directory under the repository root and skips only its own fixture root (`tools/campaign-infra-checker/fixtures`). It does not descend symbolic links to directories, but it finds a symlinked `.cargo`. It refuses any `.cargo/config` or `.cargo/config.toml` it finds, whatever the file contains, under `campaign.boundary.history_leak`. That covers `rustc`, `rustc-wrapper`, `rustc-workspace-wrapper`, `include`, `paths`, `[patch]`, `[source]`, and every flag. It refuses a `path` key in the `[toolchain]` table of any `rust-toolchain.toml` or `rust-toolchain` file the same way; a legacy file that holds only a channel name is admitted. The walk is bounded by `MAX_REPOSITORY_DIRECTORIES` and fails closed beyond it. Cargo configuration outside the repository, in ancestor directories, `CARGO_HOME`, `--config` arguments, or the environment, is not visible to the checker and stays a residual in `risk.md`.

**Rationale:** The round-8 rule refused a key list and read only the root and member directories. `[build] rustc-wrapper`, `rustc-workspace-wrapper`, and `rustc`, which replace the compiler, passed. So did a configuration file in a directory between a member and the root, which Cargo also reads (Audit9 finding 4), and a toolchain `path` (finding 13). Refusing the file instead of a key list leaves no key to miss.

**Code:** `tools/campaign-infra-checker/src/fsio/configuration.rs` (`discover`), `dependencies/configuration.rs` (`validate`, `validate_toolchain`), `policy/mod.rs` (`TOOLCHAIN_FILES`, `CHECKER_FIXTURE_ROOT`, `MAX_REPOSITORY_DIRECTORIES`).

### Decision: Declare allowlisted core dependencies once

**Choice:** Binding decision D87. A core-allowlisted dependency (`CORE_NORMAL_DEPENDENCIES`, which lists `blake3`) is declared once, in the workspace. Every other declaration, in every member (exempt members included) and every table (normal, build, development, `target.*`, underscore spellings, and renamed packages), must be `workspace = true` without a `features`, `default-features`, or `default_features` key, even an empty one. No `[features]` entry may enable one of its features (`alias/feature` or `alias?/feature`). The workspace declaration must disable default features exactly once and enable no feature.

**Rationale:** The round-8 feature check read only core's manifest. The exempt checker could enable any feature of `blake3`, and Cargo's feature unification then built core's `blake3` with that feature in every workspace build (Audit9 finding 5). `default-features = true` on core's inherited declaration also passed (finding 13). With a single feature-free declaration, no manifest that Cargo reads can add a feature.

**Code:** `tools/campaign-infra-checker/src/dependencies/features/allowlisted.rs` (`validate_workspace`, `validate_member`), `policy/scope.rs` (`CORE_NORMAL_DEPENDENCIES`).

### Decision: Keep every compiled target inside its member

**Choice:** Binding decision D88. The checker derives the compiled targets of every member from its manifest, exempt members included. `[lib]` and `[[bin]]` roots must lie under `<member>/src`, and the build script must be `<member>/build.rs` or lie under `src/`. `[[example]]`, `[[test]]`, and `[[bench]]` paths must resolve inside the member, under `src/`, `tests/`, `examples/`, or `benches/`. Refusals report `campaign.functional_core.effect`.

**Rationale:** Target derivation covered only the `[lib]` root, the `[[bin]]` roots, and the build script of non-exempt members. An `[[example]]` or `[[test]]` target of `campaign-core` compiled from an ambient path therefore passed (Audit9 finding 13).

**Code:** `tools/campaign-infra-checker/src/rustscope/targets.rs` (`derive`, `target_array`).

### Decision: Present the catalog through every holder and every stop to the claims oracle

**Choice:** Binding decision D89. The held-lineage matrix presents the whole history catalog through every holder. The held campaign now stops with two selections outstanding and then settles one of them by a result after the stop, which gives a 13-event honest lineage. Through every held prefix from two events, of every holder kind (idle, outstanding selection, settled by result, settled by closure, stopped), the matrix presents:

- the honest fast-forward, and every rewind;
- forks at depths 0 to 3 and at the deepest admitted depth, with every head kind the base admits;
- every one of the 22 forged, suffix, and request classes that one shared module builds over any tip, including the shared-graph history that D82 refuses;
- through every prefix but the longest, the prefix's lineage on the older graph captured before its later drafts, which D84 refuses;
- through the stopped holder, every post-stop settlement history, as a fast-forward that loads.

The root holder has no earlier history, so it meets only the honest fast-forward, its lineage on the older graph, and another campaign's root. The forged-fence class needs a selection, so it goes only through holders whose planner selects. At the deepest depth, every head kind is presented until a holder of that kind has met them all; after that, only the stop head is presented there, because it is the one head that reaches that depth. The covering rule fails the fixture with `Diverged` unless all of these hold:

- every holder kind meets a refused rewind, every head kind its base admits, depths 0 to 3 and the deepest depth, every class, and an older graph;
- every held length meets a refusal, the shallowest and deepest fork, and every class its head admits;
- every held length but the longest meets an older graph;
- every stopped holder meets an advance that loads.

The new negative fixture `receipts.recorded_stop_overclaim` records each planner stop reason in its own campaign: the frontier stop of an empty frontier, the frontier stop of a frontier whose every candidate was selected, settled, and retired, the budget stop, the selection-limit and control-event-limit stops, and the policy stop of a reached finding limit. For each recorded stop it presents the ranked and exploratory plan receipts and the `StopAccepted` event receipt to the claims oracle under all 16 authority-signal combinations. Unrecorded planner stops stay in `receipts.planner_overclaim`, and the recorded consumer stop stays in `receipts.non_selection_overclaim`, so every stop reason reaches the oracle. The suite now has 79 fixtures. The transcript labels `SharedControlGraph` and `PendingControlDraft` denials by class, so both pilots agree on the new transcript digests.

**Rationale:** The held matrix presented depth-3 and deepest forks and the forged and suffix catalog only cold, through H0, and through H1, and none of those holders had settled or stopped. Three shells therefore passed the suite: one that resynced cold after its held campaign settled, one that kept serving its last good projection once settled, and one that treated a stopped projection as final (Audit9 findings 6 and 10). No recorded frontier or budget stop reached the claims oracle, so a consumer that claimed exhaustive search once Campaign recorded such a stop also passed (finding 9). The larger matrix raised one healthy debug `run_suite` from about 2.1 s to about 3.7 s of CPU time, measured on a heavily loaded machine.

**Code:** `crates/campaign-conformance/src/catalog/evidence/claims/stops/mod.rs` (`recorded_stop_overclaim`), `catalog/evidence/histories/classes/mod.rs` (`Class`, `CLASSES`, `entries`), `catalog/evidence/histories/held/mod.rs` (`matrix`, `HolderKind`), `held/lineage/mod.rs` (`continued`, `OUTSTANDING_AT_STOP`), `held/shapes/mod.rs` (`presented`, `foreign_root`, `STALE_GRAPH`), `held/covering/mod.rs` (`require`, `NAMED_DEPTHS`), `catalog/evidence/lineages/forks/extent/mod.rs` (`MAX_FORK_DEPTH`, `depths`, `maximal_depth`), `harness/labels/mod.rs` (`denial`).

### Decision: Harden the cheap round-9 P3 findings

**Choice:** Binding decision D90 repairs the round-9 P3 findings that were cheap to close:

- **Nested keyed vectors.** `assert_keys_strictly_increasing` also requires a candidate's declared costs, an outstanding selection's reserved costs, and a recorded `BudgetExhausted` stop's dimensions to be strictly increasing. A caller-built state that repeats or misorders them is `StaleState` at every entry point that checks state currency, and after replay. `reserve_costs` now merges costs in one sort pass, and `exhausted_dimensions` marks positions against the sorted budgets, so neither is quadratic in the number of dimensions.
- **Equal-rank revision.** A test pins that an equal-rank revision below the guidance-record bound is admitted as a `Revision` that adds a record.
- **Reserve accessors.** The `graph_room`, `reserved_control_events`, and `reserved_graph_events` documentation states exactly what each counts (see "Count a pending draft in the exposed reserve").
- **Checker panics.** A zero-prefixed pinned fixture count is the typed `campaign.consumer_pilots.valid` denial instead of a panic. An audit of every non-test assertion, index, unwrap, and arithmetic site in the checker fixed three more input-reachable panics: an empty package name, an unnamed workspace, and a crate root declared as a `#[cfg(test)]` module, which is now refused under `campaign.functional_core.effect`.
- **Registered pinned test.** The pinned consumer constants count only while a running test reproduces them. The conformance library's `[lib]` must keep its test harness. Its root must declare a top-level `#[cfg(test)] mod tests;` whose module declares an attribute-free `mod consumers;`. That module must hold a top-level `#[test]` that is not ignored, expected to panic, or configured away, and that names all three `PINNED_*` constants beside a `run_suite` call. Round 10 tightened this rule (D93): the test's body must name the constants and call `run_suite` directly or through a same-module function its body calls, and no inner `#![cfg]` may configure away the library root.
- **Spec wording.** The Audit9 finding 13 wording items now state what the code does, in `campaign.guidance.declared_order`, `campaign.guidance.revision`, `campaign.stop.control_event_limit`, `campaign.budgets.control_event_reserve`, and `campaign.publication_fence.capacity`.

**Rationale:** Each item was a P3 finding of round 9 (findings 7, 8, 11, and 13). Honest histories cannot reach the nested-key state, the checker panic failed closed, and an unregistered or ignored pinned test could back a stale record. Finding 7's other items stay open as residuals in `risk.md`: nothing relates a migrated consumer's Campaign pin to the suite digests, and the migrated-consumer slot accepts any inventoried repository.

**Code:** `crates/campaign-core/src/policy/validation/mod.rs` (`assert_keys_strictly_increasing`), `frontier/dispatch/mod.rs` (`reserve_costs`), `policy/capacity/mod.rs` (`exhausted_dimensions`), `crates/campaign-choregraph/src/contracts/view/mod.rs` (`graph_room`, `reserved_control_events`, `reserved_graph_events`), `tools/campaign-infra-checker/src/consumers/evidence.rs` (`count`), `consumers/registration.rs` (`validate`, `require_running_test`), `specs/campaign/spec.md`.

### Decision: Publish control events only on a durably committed branch value

**Choice:** Binding decision D91. Hosts MUST publish a Campaign control event only on a durably committed branch value: they never plan or publish through a publication's planned branch value (its branch plan's `after`) before that compare-and-set is durable (`campaign.publication_fence.durable_base`). For such a host every uncommitted draft below the branch head is a child of the head: its own publication whose compare-and-set did not commit, or siblings that racing publishers left. Republishing one checks that draft's actual demand, which never exceeds the counted reserve, so a host that keeps its other appends within `graph_room` can republish its pending draft, settle every outstanding selection, and publish the stop. Of racing siblings whose joint demand exceeds the host capacity, only a draft whose actual demand still fits republishes.

D83's reserve is unchanged. It still counts every draft that descends from the head, including drafts a pipelining host chained below another draft, each as a selection, as a conservative defence for hosts that break the rule. For a chained draft that count is exact for a selection, one event over for other new work, and two events over for a chained settlement or stop, whose event the graph already holds and whose slot is still counted. Republishing the draft above a pipelined chain therefore counts each chained draft as a selection again, so the chain can be refused at a bound with a capacity error although no other writer appended, and the head then stays blocked with `PendingControlDraft`. This pipelined-chain case is outside the recovery guarantee and is a documented residual in `risk.md`. From the durable head, a chained selection draft is refused with `PublicationFenceMismatch`, and every other chained control event with the Campaign replay error `StaleState`, because its event names the state after the draft above it. Only a hand-built chained draft whose event names the head's own state reaches `PendingControlDraft`. The spec states the host MUST, and the scenarios `republication`, `pending_recovery`, `pending_reserve`, and `pending_several` state their guarantees only for hosts that keep it.

**Rationale:** The round-10 adapter scout traced a P2 candidate in the D83 class (the class of round-9 finding 2): a host that published the stop on a granted selection's planned branch value before that compare-and-set was durable, or published the stop below the selection's result, could never republish the chain near the graph bound. The executed selection never settled and the stop never recorded, with no other writer appending. The adapter fixer's residual tests reproduce it at graph limits 7 and 8 and at host projection bounds 6 and 7, and show the chain completing at limit 9 and bound 8. The draft's kind cannot be read from the graph, so the reserve cannot count a chained settlement exactly, and serializing publication on durable branch values is already the host's obligation under `campaign.publication_fence.pending_recovery`. Stating it as a MUST, keeping the conservative reserve as defence, and recording the residual closes the class without a new adapter API. The same host that keeps the rule completes the identical events at the same bound.

**Code:** `crates/campaign-choregraph/src/headroom/reserve/mod.rs` (module documentation, `record`), `occupancy/mod.rs` (module documentation), `contracts/view/mod.rs` (`ValidatedControlProjection::graph_room`, `reserved_control_events`), `admission/mod.rs` (`plan_control_event_publication`), `exclusivity/mod.rs` (`pending_control_draft`), `contracts.rs` (`AdapterError` capacity and `PendingControlDraft` documentation); tests in `crates/campaign-choregraph/tests/integration_repair/exclusive_head/pipelined/residual/mod.rs` (`stop_then_result_wedges_at_the_graph_bound`, `stop_then_result_wedges_at_the_projection_bound`, `result_then_stop_wedges_at_the_graph_bound`) and `residual/obligation/mod.rs` (`a_non_pipelining_host_completes_the_same_chain_at_the_bound`, `a_chained_non_selection_draft_fails_replay_from_the_durable_branch`).

### Decision: Present every receipt of every claims session in every later state

**Choice:** Binding decision D92. The claims fixtures (`receipts.non_selection_overclaim`, `receipts.planner_overclaim`, and `receipts.recorded_stop_overclaim`) run witnessed sessions. After the admission and after every publication, the harness has the adapter load the reached state, so the shell holds it, and then asks for the claims on the event receipt of every control event the session has published, projected against that state, under a ranked and an exploratory request (a selection event under its own profile) and all 16 authority-signal combinations. The events cover the admission, candidate batches, guidance, selections, results that report a child and a finding, closures, retirements, prunes, and stops. The states cover the running campaign and every stopped one: each planner stop reason, the recorded consumer stop, and the adapter's host-capacity stop. `recorded_stop_overclaim` presents the ranked and exploratory plan receipts of every planner stop before it is recorded (including the frontier stop of a frontier whose every candidate was selected, settled, and retired) and after, and it records the host-capacity stop planned through `plan_next_from_published_projection` under a host graph bound of 11 (`Session::open_hosted`). Refusal, wait, and stop plan receipts are presented as soon as they are projected. The fixtures that settle a reported result require the accepted result to carry its child and finding, so a mapping that drops or fabricates findings diverges there too. The fixture count stays at 79; the transcript digests changed (see `verification.md`).

The host-capacity stop has no plan receipt before it is recorded: core refuses to project it with `CrossedIdentity` (`campaign.receipts.host_capacity`), so only its recorded plan receipts and its event receipt reach the oracle. Refusal receipts are presented only against running states, because core refuses intake after a stop.

**Rationale:** Round-10 conformance scouts traced four P2 candidates by reading code: after a recorded non-consumer stop and after the host-capacity stop, no earlier event receipt reached the claims oracle (ConformanceClasses F1); no receipt of a result with findings or children did (F2); no event receipt was presented against a running state (F3); and the frontier stop of a searched and retired frontier was never presented before recording (SpecAdapterChecker A). The conformance fixer reproduced each one: holed shells `ExhaustedResult`, `StoppedEvent` (every stop class but the consumer stop), `CapacityStop`, `FindingResult`, `ChildResult`, `RunningDeletion`, `HeldRunningResult`, and `UnrecordedSearchedFrontier` passed all 79 fixtures under both profiles at the round-9 head. Presenting every receipt against every later state removes the class instead of the named instances. One healthy debug `run_suite` measured 3.1 to 3.4 s, within noise of the round-9 binary on the same machine.

**Code:** `crates/campaign-conformance/src/catalog/evidence/claims/witness/mod.rs` (`witnessed`, `present`), `claims/mod.rs` (`non_selection_overclaim`, `require_unclaimed`), `claims/stops/mod.rs` (`recorded_stop_overclaim`, `unrecorded`, `recorded`, `retired_frontier`, `budget`), `claims/stops/limits/mod.rs` (`selection_limit`, `control_event_limit`, `finding_limit`, `host_capacity`, `capacity_stop`), `harness/mod.rs` (`Witness`), `harness/history/mod.rs` (`Session::open_hosted`), `harness/history/origin/mod.rs` (`admission`, `cold_start`); tests in `crates/campaign-conformance/src/tests/receipt_classes/mod.rs`.

### Decision: Review the checker as a lexical guard over ordinary changes

**Choice:** Binding decision D93. The infra checker is a lexical guard over ordinary repository changes, not a proof against deliberate evasion (`campaign.boundary.lexical_guard`). An ordinary change is a dependency, a source file, or a manifest entry written the ordinary way. A checker gap counts as P2 only when such a change introduces a forbidden dependency, ambient source, or effect and passes. Deliberately evasive constructions are P3: contradictory or dead `cfg` attributes, disabled test harnesses, compiler or toolchain redirection, and macro or procedural-macro tricks outside the enumerated forms. Three cheap closures landed anyway:

- **Test-only rule.** `patterns::gating::collect` counts a module as test-only only if every declaration naming its module path, a file `mod x;` or an inline `mod x {}` anywhere in the member, is gated: it carries exactly `#[cfg(test)]` and no other attribute that applies `cfg`, directly or through `cfg_attr`, or it sits inside a gated inline module or a test-only file. A twin ungated declaration, or an extra `cfg` on the gated one, makes the file production code, which then fails the history, consumer, or replay family scan with the ordinary denial (`campaign.boundary.test_code`). Nothing new is refused.
- **Library-root `cfg`.** `consumers::registration::validate` applies the inner-`cfg` check to the conformance library root as well as to the tests module and the pinned module, including `cfg` reached through `cfg_attr`. The real root's `#![cfg_attr(dylint_lib = "octet", feature(register_tool))]` is still admitted.
- **Running pinned test.** An armed top-level `#[test]` whose body names all three `PINNED_*` constants must call `run_suite` in its body, or call a top-level function of the same module whose body calls `run_suite` (one hop; `consumers::calls::reaches_suite`). A call is a path expression matched on its last segment or, inside macro arguments, an identifier followed by a parenthesised group. This replaces the round-9 module-wide check that the module names `run_suite`.

**Rationale:** The round-10 scouts traced checker candidates B (a library root disabled by `#![cfg(not(test))]`, and an armed test that names the pins while `run_suite` runs only in an ignored test) and C (a twin or never-compiled `#[cfg(test)]` declaration that exempts a production module from the family scans). By the round-9 rubric these were P2; under D93 they are deliberately evasive constructions, so P3. The checker fixer reproduced each one before closing it (overlays `consumers-disabled-library-tests`, `consumers-suite-free-armed-test`, `core-twin-declared-tests-module`, and `core-never-compiled-test-twin` passed at the round-9 head). The rubric keeps later rounds on the checks that matter for ordinary maintenance instead of an open-ended evasion search.

**Code:** `tools/campaign-infra-checker/src/patterns/gating.rs` (`collect`, `visit_item_mod`, `is_test_gate`), `consumers/registration.rs` (`validate`, `require_running_test`), `consumers/calls.rs` (`reaches_suite`, `armed_tests`); tests `patterns/gating/tests.rs` (`a_module_is_test_only_when_every_declaration_naming_it_is_exactly_cfg_test`), `consumers/tests/reach.rs` (`refuses_an_armed_test_that_never_runs_the_suite`), `consumers/tests/registration.rs` (`refuses_a_disabled_harness_or_configured_away_module`, `admits_a_registered_armed_test_in_either_module_layout`), `validation/scope/mod.rs` (`refuses_each_overlay_with_its_exact_denial`), `validation/consumers.rs` (`refuses_each_consumer_overlay_with_its_exact_denial`).

### Decision: Re-validate the admission in state currency

**Choice:** Binding decision D94. `assert_state_identity_current` also re-derives and validates the admission: its campaign identity, its bounds identity, the bound values that identity frames, and its admitted projection binding. A caller-built state whose admission was edited, such as a bound raised without re-deriving `bounds_id` and `campaign_id`, bounds re-derived under the admitted campaign identity, or an admitted binding replaced under an unchanged campaign identity, is refused with `StaleState` by every entry point that checks state currency (planning, receipt projection, stop, result, and closure intake, cache derivation, and replay) before any policy reads it, without mutation (`campaign.identity.admission_current`). `require_state_history` relies on that check and keeps only a debug assertion on the admitted binding. Other caller-built inconsistencies between a state's counters and records, such as an edited selection count, reserved resources, or candidate statuses, are not rechecked: in-contract states come from `reconstruct` and `apply_control_event`, and `risk.md` records the residual.

**Rationale:** The state identity frames the admission as stored, not whether its identities re-derive from its values. A round-10 core scout traced a P3 candidate: raising the admitted outstanding bound from one to two and recomputing only the state identity gave a current state on which the planner selected a second candidate and replay accepted it: two outstanding selections under an admitted bound of one. The adapter fixer reproduced it before the repair. Honest histories cannot reach such a state, so this is P3, and the admission check is cheap.

**Code:** `crates/campaign-core/src/policy/validation/mod.rs` (`assert_state_identity_current`), `policy/receipts/mod.rs` (`require_state_history`), `frontier/replay/mod.rs` (`apply_control_event` documentation); tests in `crates/campaign-core/tests/core_policy/admission_currency/mod.rs` (`an_edited_admitted_bound_is_stale_state_at_every_entry_point`, `rederived_bounds_under_the_admitted_campaign_identity_are_stale_state`, `an_edited_admitted_binding_is_stale_state`, `an_honest_reconstructed_state_is_current`).

### Decision: Align round-10 wording

**Choice:** Binding decision D95 applies the round-10 scouts' wording items:

- `campaign.publication_fence.pending_several` states that every chained non-selection event is refused from the durable head with `StaleState`, and that `PendingControlDraft` naming the same draft answers only an event computed at the head's own state, which only a hand-built chained draft carries.
- `campaign.conformance.bounds` states the covering rule as `held/covering/mod.rs` implements it (the forged fence excepted for the stopped holder kind; every length meets a refusal and, except the longest, an older graph; the root meets its lineage on the older graph and another campaign's root).
- `campaign.functional_core.effect` names the `src/` sources of the core, adapter, and conformance members, including their `#[cfg(test)]` modules, and their build scripts.
- The stale `risk.md` rows on caller-built states and pipelined chains are rewritten in the round-10 document update. The lifecycle documents for D82-D90 that the scouts found missing were recorded in `uvutrwpw`, after the audited head.

**Rationale:** Each item was a P3 wording or documentation finding of the round-10 scouts (SpecAdapterChecker D, E, F, and G; CoreClasses F2; SpecCoreFod A).

**Code:** `specs/campaign/spec.md`; `risk.md`; `crates/campaign-conformance/src/catalog/evidence/histories/held/covering/mod.rs` (`require`, unchanged).

### Decision: Frame review rounds as specification-compliance and coverage review

**Choice:** Binding decision D96. From round 11 on, a review round is a specification-compliance and coverage review against the audit targets in **Adversarial Audit**, under the P1/P2/P3 rubric recorded there (D73, D93, D96). P1 and P2 findings need an executed reproducer; code-read candidates are reproduced before they are classified. The infra checker is reviewed only for ordinary repository changes (D93), not for evasion techniques. The findings of the final clean round are recorded in `risk.md` as residual risks with owners.

**Rationale:** The round-10 lead reviewer stopped early, and its scouts reported code-read candidates only. Classification then depended on each fixer's reproduction, and two checker candidates were P2 only because the round-9 rubric counted evasive constructions. Requiring executed reproducers and fixing the checker's scope makes a clean round decidable.

**Code:** none; this decision frames the review.

### Decision: Cover every four-way combination of declared state features

**Choice:** Binding decision D97 (user, 2026-09-25). Conformance coverage of state-keyed adapter mappings is stated over declared state features and checked against bounded reachability, instead of being repaired one state-fact pairing per round (`campaign.conformance.state_coverage`). The code lives in `crates/campaign-conformance/src/catalog/evidence/coverage/` (paths below are relative to it) and runs as one merged negative fixture, `history.state_coverage` (`coverage/mod.rs :: four_way_states`), which brings the suite to 80 fixtures.

- **Declared features.** `features` (`Status`, `StateClass`, `of`) maps a `FrontierState` to its status and ten booleans. The status is running or the reason of the accepted stop: `FrontierExhausted`, `BudgetExhausted`, `SelectionLimitReached`, `ControlEventLimitReached`, `PolicyRequested`, `ConsumerRequested` under the adapter's host-capacity reason (`HostCapacity`), or `ConsumerRequested` under any other reason (`Consumer`). The booleans are guided (a guidance record exists), selected, outstanding, results, findings, discoveries, closures (a selection settled by unobserved closure), retired, pruned, and retained.
- **Declared classes.** `classes :: admits` states the constraints the Campaign rules imply, each documented with its spec scenario: selected implies guided; outstanding, results, and closures each imply selected, and selected implies one of them; findings and discoveries imply results; guided implies a retained, retired, or pruned candidate; outstanding implies a retained or retired candidate; the four planner stops imply nothing outstanding; the selection-limit stop implies selected; the budget stop implies selected and retained; and at the frontier stop a retained candidate implies guided, while guided implies a retired or pruned candidate. `classes :: all` enumerates the 793 feature vectors these constraints admit.
- **Bounded reachability proof.** `reach` (`cfg(test)`) searches breadth-first over `campaign-core`, over every control event a consumer can publish (`reach/steps`), up to `DEPTH = 8` control events after the admission. It starts from each of the seven admission configurations of `CONFIGURATIONS` (ample budgets, budgets of one unit, two units with one retained candidate, one selection, `max_control_events = 6`, one guidance record, and one retained candidate), with at most three unit-cost candidates A, B, and C below the root. States are deduplicated by their identity with path-derived identities erased (`reach/erased`); `the_erased_key_reaches_the_classes_exact_state_identity_reaches` checks, to depth 3, that exact `state_id` deduplication reaches the same classes. `every_state_reached_within_the_bound_lies_in_a_declared_class` requires every reached class to be declared. When D97 landed, the search explored 109692 states and reached 478 of the 793 declared classes; the test does not pin these counts.
- **Canonical recipes.** `recipes :: recipe` gives every declared class an admission configuration and abstract steps that reach it through the adapter, choosing among candidates only in suite operation order. Each recipe also holds every event sub-kind compatible with its class (`kinds :: compatible`): candidate batches with and without a coalesced duplicate; discovery, strictly improving, coalesced, and revising guidance; ranked and exploratory selections; plain, child, finding, and child-and-finding results; closures; consumer- and policy-origin retirements; prunes; and the class's stop. Recipes publish the host-capacity stop (a consumer stop under `control_capacity_reason_id`) and the policy stop (under the finding-limit policy reason) directly, which core admits, instead of reaching them through a small host graph or a reached finding limit. `recipes/tests :: every_declared_class_is_reached_by_its_recipe` replays every declared class through the adapter (`recipes/replay`) and requires it to reach its class.
- **Pinned covering table.** `table :: TABLE` pins the greedy covering (`table/greedy`) of the declared classes: 167 classes under D97, and 176 since D99 made planner-stop plan receipts compatible with every class. It carries every 4-way combination of feature values that a declared class carries (7789) and every compatible pair of a receipt kind and a 3-way combination (40197 under D97, 40976 under D99), over the 24 receipt kinds of `kinds` (`KIND_COUNT`). `table/tests :: the_pinned_selection_is_the_greedy_covering_of_the_declared_classes` recomputes the table, and `the_pinned_selection_carries_every_declared_combination` checks the covering; `combos` counts the combinations.
- **Presentations.** For every table class, `four_way_states` builds the state once through the adapter from its recipe. `held :: present` then holds Campaign's projection of that state and presents to the load and cache paths the stale graph (the state's lineage on the graph before a later exploration moment), the fast-forward (the consumer stop of a running state) or the post-stop advance (the settlement of an outstanding selection after the stop), every rewind, forks of the history before the head at depths 0 and 1 with every head kind, and every catalog class the head admits; a bare admission meets another campaign's root instead (`held/shapes`). `claims :: present` presents to the claims oracle, under all 16 signal combinations, the event receipt of every lineage event for a ranked and an exploratory request, the plan receipt of every plan made along the lineage (projected when planned, including each planner stop before it was recorded), the state's own ranked and exploratory plan receipts, and, at a running state, refusal receipts.
- **Covering rule in code.** `covering :: require` fails the fixture with `Diverged` unless the presented states carry every declared 4-way combination and every compatible (receipt kind, 3-way combination) pair.

**Rationale:** Rounds 6 to 11 each found conformance shells keyed on a state fact that the suite never paired with a presentation, and each round repaired only the pairings it found; round 11's three P2 findings were of that kind. Declaring the features and the bounds makes coverage checkable: the bounded search shows that the declared classes contain every class reachable within the bounds, replay shows that every declared class is reached, and the covering rule is checked on what the fixture presented. The cost was measured on the healthy `run_suite`: 12.8 s in debug against 3.6 s at Q3 (3.4×), and 3.5 s in release against 0.92 s (3.5×); the conformance library tests took 438 s in debug. The planning budget was about 2.2×. The user chose 4-way coverage over the full product of features, which was estimated at about 990 classes and 7× the Q3 cost. Mappings that need a conjunction of five or more feature values, facts the features merge, or states beyond the bounds stay outside the scenario and are P3 under the rubric (**Adversarial Audit**).

**Code:** `crates/campaign-conformance/src/catalog/evidence/coverage/{features,classes,kinds,combos,reach,recipes,table,held,claims,covering}`, registered in `catalog/evidence/mod.rs`; the round-11 shells are kept as negative controls in `crates/campaign-conformance/src/tests/covering_classes/`.

### Decision: Dispose of the round-11 P3 findings

**Choice:** Binding decision D98 disposes of the round-11 P3 findings. The three round-11 P2 conformance coverage gaps are repaired by binding decision D97 (bounded 4-way state coverage), recorded above.

- **Specification and document wording (Review11Spec 1-9).** `risk.md`, this document, and `verification.md` now state what the code does: a pending draft answers `PendingControlDraft` before core planning and before the recorded-reserve check, so only a plan through a projection without a pending draft that core planning accepts gets the capacity error; every adapter-built chained non-selection event fails replay with `StaleState`; recovery republishes the named draft or another held child of the head; only drafts under the projection's envelope are pending; forks over the history before H0's head start at depth 0; `plan_pruning` prunes at most `max_prune_batch` per plan. Stale citations and residual gaps in `verification.md` are re-pointed or removed, `risk.md` holds the canonical non-claim list that the other lists match, and `risk.md` gains residual rows for receipt bindings, the interior-level sampling cap, the checker's fixture-root exception, the consumer record's scope, and resource observations. In the spec, `campaign.publication_fence.pending_reserve` limits its capacity clause to projections without a pending draft, `campaign.history_projection.caller_binding` adds the `StaleProjection` refusal of a binding that does not re-derive, `campaign.boundary.lexical_guard` also names `campaign.boundary.shared` and `campaign.choregraph_dependency.moving`, `campaign.boundary.shared` states the per-member allowlist refusal, and `campaign.boundary.test_code` states the inner `#![cfg(test)]` and gated-item exemptions.
- **Checker scope (Review11Spec 10).** `campaign.boundary.history_leak`, `campaign.replay_boundary.invalid`, and the replay-boundary task are narrowed to the sources of the core, adapter, and conformance crates. `campaign-profile` sources are not identifier-scanned, and consumer-authority type names are enforced through dependency families only; `risk.md` records both, owned by the infrastructure checker maintainers.
- **Core wording (Review11Core 3, 4, 5).** `campaign.guidance.declared_order` states that each scheduling stage runs for both pairs before the next, so the first pair's error answers only when both fail at the same stage. `campaign.stop.control_event_limit` scopes its accept-or-`InvalidStop` clause to open campaigns; once a stop is recorded, stop admission and replay answer `CampaignStopped`. `campaign.stop.host_capacity`, `campaign.publication_fence.pending_reserve`, and the text of "Reserve every descending draft and recover in chain order" state the adapter planner's order: request binding, pending draft, core planning, recorded reserve, capacity transformation. The code is unchanged.
- **Core residuals (Review11Core 1, 2, 6).** `risk.md` records, owned by the Campaign core maintainers: an admission with empty or all-zero budgets is accepted but admits no candidate; core cache validation does not tie the binding to the state's own history, which the adapter path does; a supplied guidance identity that does not re-derive is replaced by the re-derived one instead of refused.
- **Conformance P3s.** Review11Conformance 4, 7, 8, 9, and 10 are repaired in the suite: self-consistent two-field evidence mutations in the gate matrices, gate questions without a publication or evidence repeated after the host loads the publication, a role sample of a result with a child and a finding, a revision after a coalesced duplicate, and a ticket counter that must equal the next selection ordinal. Review11Conformance 5 and 6 are covered by D97's receipt kinds and plan-receipt timing.

**Rationale:** Round 11 found no P1 and no P2 in the core, adapter, checker, and spec scopes, and three P2 coverage gaps in the conformance suite, all keyed on state facts the suite never paired with a presentation; D97 closes that class. Every other finding was P3: wording that contradicted the code or another scenario, stale citations, missing residual rows, and narrow sampling gaps. Wording and residual rows close the P3s that need no code change; the conformance P3s are cheap suite repairs.

**Code:** `specs/campaign/spec.md`, `risk.md`, `verification.md`, `tasks.md`; conformance repairs in `crates/campaign-conformance/src/catalog/fencing/matrix`, `catalog/evidence/roles`, `catalog/guidance`, and `harness/decisions/entropy`, with their regression tests in `crates/campaign-conformance/src/tests/pairing_classes/`.

### Decision: Declare the presented history and close after round 12

**Choice:** Binding decision D99 (user, 2026-09-26). Round 12 reviewed R4 (change `syrlrqrx`, `65eee6e7`) in the conformance scope only, with two reviewers (construction and shells), and found no P1, three P2, and eight P3 findings. D99 repairs the three P2 findings, verifies each repair with a regression test built from the round-12 shell that exposed it, and closes the review without a round 13.

- **Presented-history dimension (Review12Shells 1 and 2).** D97 declared the features of the held state but not the history presented against it: the fork shapes of `coverage/held` were fixed at `FORK_DEPTHS = [0, 1]` beyond the dropped head, with the six head kinds of `lineages/forks/heads` (consumer stop, guidance, single-candidate batch, ranked selection, result without findings, closure). A resync that reloads cold once the presented branch is two or more events ahead of a held state with one declared feature value (finished, retired, pruned, discovered, planner-stopped, or nothing retained), or once the new events contain a retirement, prune, revision, finding result, or exploratory selection, passed the whole suite, although the probes showed such hosts selecting after a recorded stop, selecting retired or pruned candidates, and re-settling a recorded selection, which Campaign refuses with `NonFastForwardBranch`. D99 declares two dimensions of the presented history: the fork depth, with the values 0, 1, 2, 3, and the deepest depth that the projection bound, the control-event room, the graph room, and the fork fill's room admit; and the fork head's event sub-kind, with every sub-kind the fork state admits (candidate batches with and without a coalesced duplicate; discovery, strictly improving, coalesced, and revising guidance; ranked and exploratory selections; results with and without child moments and findings; closures; consumer- and policy-origin retirements; prunes; consumer, host-capacity, policy, and planner stops). The deepest value is the maximal depth less the head's reserve (0 for stops, 2 for selections, 1 otherwise). The 20 head sub-kinds are `Batch`, `CoalescedBatch`, `Discovery`, `Improvement`, `CoalescedGuidance`, `Revision`, `RankedSelection`, `ExploratorySelection`, `PlainResult`, `ChildResult`, `FindingResult`, `ReportedResult`, `Closure`, `ConsumerRetirement`, `PolicyRetirement`, `Prune`, `ConsumerStop`, `HostCapacityStop`, `PolicyStop`, and `PlannerStop` (`lineages/forks/subkinds`). Each value is paired with every 3-way combination of held features that a table state admitting the value carries (`coverage/forked`). Through each table state, `coverage/held/forks` presents to the load and cache paths the D97 forks (the six head kinds at depths 0 and 1) and then, greedily, forks of every (depth, head sub-kind) shape whose value some 3-way combination of that state still lacks, built one shape at a time over a shared fill chain (`lineages/forks/shaped`). `covering :: require` fails the fixture with `Diverged` on a missing pair. A healthy run presents 921 greedy forks and all 9658 admitted (fork depth, 3-way) pairs and 34415 admitted (head sub-kind, 3-way) pairs. The fixed shapes of D97 stay unchanged. Two construction fixes support the deep forks: a fill improvement whose candidate's latest record is already at or past the step's generation takes the generation after that record (`fill :: improved`), and the fill's own room (`fill :: room`) bounds the maximal depth (`extent :: maximal_depth`).
- **Planner-stop plan receipts in every class (Review12Construction 1).** `kinds :: compatible` made the plan receipt of an unrecorded planner stop compatible only with the classes of the four planner stops, so a claims shell that attaches exhaustive search to a stale planner-stop plan receipt once a consumer or policy stop is recorded passed the whole suite. Such lineages exist: a policy lineage that planned the finding-limit stop, and any lineage whose host recorded its own stop after the planner reported one. D99 makes that receipt kind compatible with every class, and every recipe keeps the bare admission's ranked and exploratory planner-stop plan receipts (`recipes/replay/steps :: admission_stop`), so that the claims oracle meets them at states of every class, consumer- and policy-stopped ones included. This adds 779 compatible pairs (40197 to 40976), and the greedy freshness test re-pinned the table from 167 to 176 classes.
- **Verification.** `crates/campaign-conformance/src/tests/forked_classes/` keeps the twelve round-12 P2 shells as regression tests: six resyncs two events ahead of a held state that is finished, has a retired or pruned candidate, has a discovery, is planner-stopped, or retains nothing; five rebuilds on an advanced deletion head, an advanced suffix deletion, a finding-result head, a revision head, and a forward exploratory-selection head; and a stale planner-stop receipt claim after a requested stop. Each fails only `history.state_coverage` (`NegativeFixturePassed`) under the default and reordered profiles, and with the repairs disabled all twelve passed the whole suite; `the_healthy_fork_wrapper_passes_every_fixture` is the healthy control. The suite keeps 80 fixtures, and the healthy transcript digests are `28b873e9…` and `37b9ff95…` (R4: `cbad4c2a…` and `199cef7e…`).
- **Closure (D73 amended).** The user amended the closeout criterion for this round: the final round's P2 findings are repaired and verified by regression tests without a further review round. Every round-12 P3 finding, and every dimension the suite does not declare, is recorded in `risk.md` as a residual risk with an owner: the bounded search's single-candidate batch alphabet and the 124 reachable classes it misses, wait and unpublished-selection plan receipts at stopped states, costs of candidates below discovered children, claims facts beyond the receipt-kind and 3-way pairing, role samples of finding-only and child-only results, gate questions after cache validation and settlement, post-stop results with findings and children, a second revision of one candidate, and the undeclared dimensions in general.

**Rationale:** Round 12 found that D97's declared features covered the held state but that the history presented against it was still fixed by the suite, which is the class of gap D97 replaced for state facts; declaring fork depth and head sub-kind, paired with 3-way held combinations and checked by `covering :: require`, closes it in the same way. The kind-table repair corrects a compatibility rule that was narrower than the lineages it describes. The P3 findings are weaker coverage gaps, liveness-only refusals, evidence gaps of the reachability proof, or facts the declared features merge; each is owned in `risk.md`. The user chose to close on repaired and regression-tested P2 findings with owned residuals instead of a further round. The cost was measured on the healthy `run_suite`: 21.3 s in debug (`history.state_coverage` 17.3 s) against 12.5 s at R4, and 5.2 s in release (4.0 s) against 2.9 s, about 70% above R4 where about 30 to 50% was planned; the deep forks' fill chains dominate. `risk.md` records the cost with its owner.

**Code:** change `uwzryzpt`. In `crates/campaign-conformance/src/catalog/evidence/`: `coverage/forked` (new), `coverage/held/forks`, `coverage/kinds` (`compatible`), `coverage/recipes/replay/steps` (`admission_stop`), `coverage/covering` (`require`), `coverage/table` (`TABLE`, 176 classes), `lineages/forks/shaped` and `lineages/forks/subkinds` (new), `lineages/forks/heads` (`latest_outstanding`), `lineages/forks/fill` (`improved`, `room`), and `lineages/forks/extent` (`maximal_depth`); regression tests in `crates/campaign-conformance/src/tests/forked_classes/`, listed in `verification.md`.

## Dependency Direction

```text
campaign-core <---- campaign-profile (std loader for the exported Nickel profile)
      ^
      |
campaign-choregraph ----> published choregraph-history
      ^
      |
campaign-conformance
```

No dependency returns from Choregraph into Campaign. No Campaign crate depends on Universal Weave. `campaign-core`, `campaign-choregraph`, and `campaign-conformance` stay `no_std + alloc`. `campaign-conformance` also depends on `campaign-core` directly, and its tests load profiles through `campaign-profile` as a dev-dependency. `tools/campaign-infra-checker` depends on no Campaign crate. It checks the others from source and manifests.

## Decision Flow

```text
consumer candidates, guidance, entropy, and results
  -> campaign-choregraph validates exact history bindings
  -> campaign-core reconstructs frontier and returns select, wait, or stop
  -> campaign-choregraph plans an immutable control event and fenced branch move
  -> consumer shell persists both through Choregraph-owned contracts
  -> consumer shell executes the selected operation
  -> consumer records the resulting events and child moments
```

## Adversarial Audit

The audit must try to show that Campaign:

- implements another event DAG or branch-reference store;
- selects from stale or ambiguous history;
- consumes a reusable candidate after one selection;
- accepts duplicate selection ordinals or exceeds the outstanding bound;
- selects a retired candidate or treats retirement as history deletion;
- executes an operation before durable selection publication;
- treats a retry as a new exploration fork;
- deletes immutable history through pruning;
- reads ambient randomness or accepts unbound exploratory entropy;
- interprets product guidance or finding meaning;
- stops for frontier or budget exhaustion while results remain outstanding;
- claims exhaustive search after frontier or budget stop;
- depends on Universal Weave or an ambient Choregraph worktree;
- bypasses Choregraph production replay or machine semantics.

The closeout criterion (binding decisions D73, D93, and D96, amended for the final round by D99) is:

> Closeout requires a review round with no P1 and no P2 finding. P1/P2 findings are repaired and reviewed again. The findings of the final clean round are recorded in risk.md as residual risks with owners (and repaired when cheap).

Binding decision D99 (user, 2026-09-26) amends this criterion for round 12, the final round: its P2 findings are repaired and verified by regression tests without a further review round, and its P3 findings, together with every dimension the conformance suite does not declare, are recorded in `risk.md` as residual risks with owners.

Each round classifies its findings with a severity rubric. Rounds 9 and 10 used the rubric of D73 and D80, which clarified P2 for conformance shells: a shell counts as P2 only when it models a plausible consumer implementation strategy, and a shell keyed to suite internals is a P3 sampling gap. From round 11 on, the rubric (D73, D93, D96) also limits checker findings to ordinary repository changes, counts deliberately evasive constructions as P3, and requires an executed test for every P1 and P2:

> P1 = a public core/adapter/profile API lets a consumer or a malformed/inconsistent history reach a behaviour the design's audit targets forbid (incl. panics); P2 = a public API violates a normative spec MUST, or a consumer implementation following a PLAUSIBLE strategy (not keyed to suite internals such as fixture names, suite-chosen identities, or thresholds beyond the maxima the suite presents) behaves in a forbidden way and still passes the whole conformance suite, or the checker admits an ORDINARY repository change (a dependency, source file or manifest entry written the ordinary way) that introduces a forbidden dependency, ambient source or effect; P3 = weaker coverage gaps, suite-internal-keyed or deliberately evasive constructions, wording gaps. P1/P2 need an executed test.

Under D93, a deliberately evasive checker construction is a P3 finding: contradictory or dead `cfg` attributes, disabled test harnesses, compiler or toolchain redirection, and macro or procedural-macro tricks outside the enumerated forms. The checker is a lexical guard over ordinary repository changes (`campaign.boundary.lexical_guard`), and reviews do not search for evasion techniques.

Under D97, the conformance suite's coverage of state-keyed mappings is stated over the declared state features and bounds of `campaign.conformance.state_coverage`: eight control events after the admission, at most three candidates, and the suite's seven admission configurations. From round 12 on, the rubric (D73, D93, D96, D97) therefore also counts as P3 a shell whose misbehaviour needs a conjunction of five or more declared feature values, depends on facts the declared features merge, or needs a state reachable only beyond the declared bounds. Such a shell is keyed to the suite's declared bounds, as a threshold beyond the maxima the suite presents is.

D99 adds two declared dimensions of the history presented to the load and cache paths: the fork depth (0, 1, 2, 3, and the deepest admitted depth) and the fork head's event sub-kind, each paired with every 3-way combination of held features that a table state carries. A shell keyed on a dimension the suite does not declare, such as execution-gate or cache timing against the held state, receipt facts beyond a receipt kind and three features, or held facts the declared features merge, is outside `campaign.conformance.state_coverage`; `risk.md` lists these dimensions with owners.

Each review round is a specification-compliance and coverage review of the integrated head (binding decision D96). It checks the public core, adapter, and profile APIs, the conformance suite, and the infra checker against these audit targets and the normative spec, including entry and replay parity, identity framing, checked arithmetic, receipt crossing, and conformance coverage. A candidate found by reading code is classified as P1 or P2 only once an executed test reproduces it. Through round 11, a round with any P1 or P2 finding blocked closeout: its findings were repaired under numbered binding decisions, and a further round reviewed the repaired head. Rounds 1 to 12 each found at least one P1 or P2 finding or candidate. Round 12, which reviewed the conformance suite at R4 (`65eee6e7`), found no P1 and three P2 coverage gaps; under D99 they are repaired and verified by regression tests, and the review closes without a round 13. Every reproducer of a repaired finding is kept as a named regression test that asserts the typed denial, and holed conformance shells stay as negative controls. `verification.md` lists each round's findings and where its regression tests live, and `risk.md` lists every finding accepted as a documented limit, every round-12 P3 finding, and the dimensions the suite does not declare.

## Risks and Trade-offs

- The adapter must preserve the exact published history contract and pass its own conformance checks.
- Opaque rank vectors can hide poor consumer guidance. Adapter conformance checks shape, not usefulness.
- A linear control branch serializes policy decisions. Consumers can shard work across campaigns when needed; each shard is a separate campaign with its own identity and its own Choregraph graph, because the adapter's reserve accounts for one campaign per graph and envelope.
- History reconstruction can cost more than loading a cache. Bounded projections and validated caches limit this cost.
- Resource observations can be false or stale. Campaign only evaluates supplied values.

## Non-Claims

This design makes none of the claims that `risk.md` lists under **Non-claims**, the change's canonical non-claim list: release readiness (including production use and release eligibility), exhaustive search, test sufficiency, fresh acquisition, event truth and payload authenticity, operation success and deterministic external execution, replay success, finding correctness, authorization, safe deletion, consumer correctness, and attribution of a stop to the profile a stop receipt binds.
