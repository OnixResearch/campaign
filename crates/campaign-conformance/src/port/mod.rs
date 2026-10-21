//! The consumer mapping the conformance suite drives.
//!
//! A consumer implements three small capability traits; [`ConformanceAdapter`] is their union and
//! is implemented automatically. The suite owns orchestration, history publication, planning, and
//! every Campaign call; the adapter owns only consumer identities, record mapping, and the shell
//! points where a consumer must delegate to Campaign.

mod contexts;

pub use contexts::{
    AdapterIdentities, AuthoritySignals, CandidateContext, ClosureContext, EntropyContext,
    ExecutionAuthority, ExecutionGateRequest, GuidanceContext, MomentIdentity, ResultContext,
    RevisionContext,
};

/// Consumer identities and exploration-moment mapping.
pub trait MomentMapping {
    /// Returns the consumer identity set; must be equal on every call.
    ///
    /// # Errors
    ///
    /// Returns an error when an identity cannot be derived.
    fn identities(&self) -> crate::verdicts::ConformanceResult<AdapterIdentities>;

    /// Builds the consumer's Choregraph exploration event draft for one abstract moment.
    /// `parent_event` is the admitted event of the moment's parent, if any.
    ///
    /// # Errors
    ///
    /// Returns an error when the consumer cannot represent the moment.
    fn exploration_draft(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        parent_event: Option<campaign_choregraph::EventId>,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::EventDraft>;

    /// Maps one admitted exploration event to the consumer's Campaign moment identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the consumer cannot identify the moment.
    fn moment_identity(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        event: campaign_choregraph::EventId,
    ) -> crate::verdicts::ConformanceResult<MomentIdentity>;

    /// Maps one abstract consumer reason to a bounded `Reason` identity.
    ///
    /// # Errors
    ///
    /// Returns an error when identity derivation fails.
    fn reason(
        &mut self,
        reason: crate::vocabulary::ScenarioReason,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id>;
}

/// Mapping of abstract operations, preferences, draws, and outcomes to Campaign records.
pub trait RecordMapping {
    /// Maps one abstract operation to a consumer candidate record. Its declared costs must name,
    /// for each abstract cost, the declared resource dimension at the cost's index with exactly
    /// its units (the suite presents every declared dimension, every dimension at once when
    /// several are declared, and units `2^33 + d`, `2^53 + 1 + d` (not exact in `f64`), and
    /// `u64::MAX - d` under a `u64::MAX` budget). The suite never chooses among candidates by
    /// their identities, so any candidate identity derivation is accepted.
    ///
    /// # Errors
    ///
    /// Returns an error when the consumer cannot represent the operation.
    fn candidate(
        &mut self,
        operation: &crate::vocabulary::ScenarioOperation,
        context: &CandidateContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::CandidateRecord>;

    /// Converts one abstract preference to a guidance record with bounded integer ranks, one
    /// strictly monotone rank per level, keeping every level in its position (the suite presents
    /// first levels that tie so a later level decides; scores of one level, three levels, and the
    /// profile's `max_rank_dimensions` levels, each in its own campaign, whose leading levels tie
    /// so only the last level decides; scores of three and the most levels whose middle level
    /// decides against every later level; and scores whose later level exceeds an earlier one).
    ///
    /// # Errors
    ///
    /// Returns an error when the score cannot be converted.
    fn guidance(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &GuidanceContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRecord>;

    /// Builds an explicit revision whose successor carries the converted preference.
    ///
    /// # Errors
    ///
    /// Returns an error when the score cannot be converted.
    fn revision(
        &mut self,
        preference: &crate::vocabulary::ScenarioPreference,
        context: &RevisionContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::GuidanceRevisionRequest>;

    /// Issues one explicit entropy ticket carrying both scenario draws exactly at full width and
    /// `context.counter`, the next selection ordinal, as its counter: the suite decodes every
    /// issued ticket and requires `explore_draw()` and `index_draw()` to equal the scenario draw
    /// and `counter` to equal `context.counter` (it presents index draws on a `2^63` base in every
    /// residue of two, three, and seven eligible candidates, explore draws above `2^32` whose low
    /// 32-bit word lies on the other side of the threshold, and exploit-side draws whose
    /// exploration position under the declared exploration order differs from the ranked choice).
    ///
    /// # Errors
    ///
    /// Returns an error when ticket derivation fails.
    fn entropy_ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        context: &EntropyContext,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket>;

    /// Maps one observed outcome of the executed selection to a result observation that settles
    /// exactly `context.selection` (never another in-flight selection of the same candidate: the
    /// suite settles two in-flight executions of one candidate older first and newer first) and
    /// reports exactly `context.children` and exactly `outcome.findings` distinct findings (the
    /// suite presents one, three, and the most children one result may carry, up to the
    /// profile's `max_child_moments`, and one, four, and the most findings, up to the profile's
    /// `max_findings` and at most 4096).
    ///
    /// # Errors
    ///
    /// Returns an error when the consumer cannot represent the outcome.
    fn result(
        &mut self,
        outcome: &crate::vocabulary::ScenarioOutcome,
        context: &ResultContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::ResultObservation>;

    /// Builds an acknowledged unobserved closure of exactly `context.selection` (never another
    /// in-flight selection of the same candidate: the suite closes two in-flight executions of
    /// one candidate older first and newer first and refuses a closure naming another selection).
    ///
    /// # Errors
    ///
    /// Returns an error when identity derivation fails.
    fn closure(
        &mut self,
        context: &ClosureContext<'_>,
    ) -> crate::verdicts::ConformanceResult<campaign_core::UnobservedClosure>;
}

/// Consumer shell points that must delegate to Campaign.
///
/// Wherever Campaign decides the same question, the suite compares the shell's answer with
/// Campaign's own answer for the exact presented input and fails the fixture on any difference.
///
/// Hosts that hold a validated projection must advance through it on every path: loads (below),
/// cache validation (`CacheValidationRequest::previous`), and publication, where
/// `campaign_choregraph::ControlPublicationRequest::previous` is required and always validated
/// through `campaign_choregraph::validate_control_advance`. The only cold-start entry is
/// `campaign_choregraph::validate_control_projection`.
pub trait ConsumerShell {
    /// Loads the control projection the consumer plans from. The answer must equal Campaign's
    /// own answer for the exact input: `campaign_choregraph::validate_control_advance(previous,
    /// request)` when the host holds a previously validated projection (`previous` is `Some`),
    /// otherwise the cold-start `campaign_choregraph::validate_control_projection(request)`,
    /// however far the presented lineage is ahead of the held projection and under the request's
    /// own `maximum_events`. A branch moved onto a fork of the previous lineage, at the same depth,
    /// one to three events deeper, or as deep as the presented projection bound admits (the fork
    /// fills `maximum_events`), is therefore refused even when the fork is valid history on its
    /// own; so is a fork of the history before an earlier held head presented through that head
    /// (the suite forks the history before H0's head and presents it cold, through H0, and through
    /// H1, including when the held projection has nothing outstanding); and a lineage longer than
    /// the projection bound is refused rather than answered with the held projection.
    ///
    /// # Errors
    ///
    /// Returns Campaign's adapter error of a stale, rewound, reset, forked (same depth, one to
    /// three events deeper, or as deep as the projection bound admits), renamed, lagging,
    /// rewritten, altered, ambiguous, merged (without a parent choice, with either parent chosen,
    /// or with a choice naming neither parent), forged-fence, stale-state, post-stop,
    /// replayed-event, or forged-envelope (another adapter's metadata, a non-application class, a
    /// foreign schema or projection identity) history, or of a lineage longer than the projection
    /// bound, cold, through a held projection older than the head, or through the held projection
    /// of the head.
    fn load_history(
        &mut self,
        request: &campaign_choregraph::ControlProjectionRequest<'_>,
        previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    ) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection>;

    /// Validates a derived cache. The answer must equal
    /// `campaign_choregraph::validate_cache_against_history(request, cache)`: a fast-forward of
    /// `request.previous` when the host holds a validated projection, otherwise a cold start. It
    /// rejects a cache whose binding names another graph, branch, head, or generation even when
    /// its state is current, a cache of another state under the current binding, and a cache of
    /// any history the load path refuses (every class listed there, cold and through either held
    /// projection), including a state the refused payloads replay to; it accepts a fresh cache
    /// through a held projection older than the head.
    ///
    /// # Errors
    ///
    /// Returns Campaign's adapter error of a stale or incomplete cache.
    fn validate_cache(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
    ) -> campaign_choregraph::AdapterResult<campaign_core::FrontierState>;

    /// Decides whether the consumer may execute a selection. The answer must equal Campaign's:
    /// `Granted` exactly when a publication and evidence are both presented and
    /// `campaign_choregraph::can_execute_selection_after_publication` returns `Ok(true)`, that is
    /// for the exact decision the durable publication published, ranked or exploratory (every
    /// field, not only its selection identity), with durable evidence whose event identity and
    /// branch name, head, and generation before and after all name that publication (a real
    /// transition of another publication, such as a compare-and-set winner's, never does). A
    /// publication of any other control event (a guidance, candidate-batch, or stop publication)
    /// never grants a selection, not even with that publication's own honest committed
    /// evidence.
    fn execution_gate(&mut self, request: &ExecutionGateRequest<'_>) -> ExecutionAuthority;

    /// Projects the promotion claims the consumer attaches to a Campaign receipt of any kind
    /// (ranked or exploratory selection, a wait for results, guidance, or capacity, a planner
    /// stop of any reason or a recorded stop, admission refusal, or an event of any variant);
    /// every claim must stay false under every combination of consumer signals.
    fn promotion_claims(
        &mut self,
        receipt: &campaign_core::CampaignReceipt,
        signals: &AuthoritySignals,
    ) -> campaign_core::ReceiptClaims;

    /// Reports the consumer identities of its unvisited candidate, queued work item, and
    /// computed result for one executed operation: exactly `candidate.candidate_id`,
    /// `work_item.selection_id`, and `result.result_id`, three distinct stored records.
    fn role_identities(
        &mut self,
        candidate: &campaign_core::CandidateRecord,
        work_item: &campaign_core::SelectionDecision,
        result: &campaign_core::ResultObservation,
    ) -> campaign_core::RoleCollapseFixture;
}

/// Complete consumer mapping the conformance suite drives.
// r[impl campaign.conformance]
pub trait ConformanceAdapter: MomentMapping + RecordMapping + ConsumerShell {}

impl<T: MomentMapping + RecordMapping + ConsumerShell + ?Sized> ConformanceAdapter for T {}
