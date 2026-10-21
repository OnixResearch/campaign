//! Suite orchestration: one session per fixture over real Choregraph history.
//!
//! A session owns the graph, the generation-fenced control branch, the published control payloads,
//! and the incrementally applied Campaign state. Every Campaign call goes through
//! `campaign-core` or `campaign-choregraph`; the adapter is consulted only through the port.

pub(crate) mod decisions;
pub(crate) mod history;
mod labels;
pub(crate) mod oracle;
pub(crate) mod records;
pub(crate) mod settlement;

/// Frame domain of every suite-owned identity.
const SUITE_DOMAIN: &[u8] = b"campaign-conformance-suite";

/// Choregraph control branch name of every session.
const CONTROL_BRANCH: &str = "campaign-conformance-control";

/// Projection bound of every session: the Choregraph graph's projection limit and the
/// `maximum_events` of every projection request the suite builds.
pub(crate) const HISTORY_MAX_EVENTS: u32 = 64;
/// Event bound of every session graph: room for results that report as many child moments as the
/// default profile's `maximum_children_per_result` beside the control lineage.
pub(crate) const GRAPH_MAX_EVENTS: u32 = 512;
const HISTORY_MAX_PARENTS: u32 = 4;
const HISTORY_MAX_METADATA: u32 = 4;
const HISTORY_MAX_LABEL_BYTES: u32 = 64;

/// Generation of the admission branch before and after the campaign root is published.
const ROOT_GENERATION: u64 = 0;
const ADMITTED_GENERATION: u64 = 1;

/// The adapter, profile, and transcript one suite run shares across fixtures.
pub(crate) struct Driver<'a> {
    /// Consumer mapping under test.
    pub(crate) adapter: &'a mut dyn crate::port::ConformanceAdapter,
    /// Caller-supplied suite profile.
    pub(crate) profile: &'a crate::inputs::SuiteProfile,
    /// Transcript of the whole run.
    pub(crate) transcript: &'a mut crate::ledger::DecisionTranscript,
}

impl Driver<'_> {
    /// Records one transcript entry.
    pub(crate) fn record(&mut self, entry: crate::ledger::TranscriptEntry) {
        self.transcript.record(entry);
    }

    /// Requires a forbidden input or effect to be denied with a declared error.
    ///
    /// # Errors
    ///
    /// Returns `ForbiddenAccepted` when the input went through (including an adapter shell that
    /// accepted it where Campaign refuses it) and `UndeclaredDenial` when it was denied with
    /// another error.
    pub(crate) fn deny_with<T>(
        &mut self,
        result: crate::verdicts::ConformanceResult<T>,
        forbidden: &'static str,
        is_declared: impl Fn(&crate::verdicts::ConformanceError) -> bool,
    ) -> crate::verdicts::ConformanceResult<()> {
        let Err(error) = result else {
            return Err(crate::verdicts::ConformanceError::ForbiddenAccepted { forbidden });
        };
        if matches!(
            error,
            crate::verdicts::ConformanceError::ForbiddenAccepted { .. }
        ) {
            return Err(error);
        }
        if !is_declared(&error) {
            return Err(crate::verdicts::ConformanceError::UndeclaredDenial {
                forbidden,
                denial: alloc::boxed::Box::new(error),
            });
        }
        let label = labels::denial(&error);
        self.record(crate::ledger::TranscriptEntry::labelled(
            crate::ledger::kinds::DENIED,
            &label,
        ));
        Ok(())
    }

    /// Requires a forbidden input or effect to be denied with one typed Campaign error.
    ///
    /// # Errors
    ///
    /// Returns `ForbiddenAccepted` or `UndeclaredDenial` as `deny_with` does.
    pub(crate) fn deny<T>(
        &mut self,
        result: crate::verdicts::ConformanceResult<T>,
        expected: campaign_core::CampaignError,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<()> {
        self.deny_with(result, forbidden, move |error| {
            error.campaign().as_ref() == Some(&expected)
        })
    }
}

/// One abstract moment admitted to the session graph.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MomentEntry {
    pub(crate) handle: crate::vocabulary::MomentHandle,
    pub(crate) event: campaign_choregraph::EventId,
    pub(crate) identity: crate::port::MomentIdentity,
}

/// Presentation run after every publication of a witnessed session, against the state the
/// publication reached.
pub(crate) type Witness = fn(&mut Session<'_, '_>) -> crate::verdicts::ConformanceResult<()>;

/// One campaign over real history, driven through the adapter.
pub(crate) struct Session<'s, 'a> {
    pub(crate) harness: &'s mut Driver<'a>,
    pub(crate) identities: crate::port::AdapterIdentities,
    pub(crate) graph: campaign_choregraph::HistoryGraph,
    pub(crate) branch: campaign_choregraph::BranchRef,
    pub(crate) payloads: alloc::vec::Vec<campaign_choregraph::ControlPayloadEnvelope>,
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) rule: campaign_core::AdmissionRule,
    moments: alloc::vec::Vec<MomentEntry>,
    operations: alloc::vec::Vec<(crate::vocabulary::OperationHandle, campaign_core::Id)>,
    /// Committed evidence of every branch transition the session published (the admitted control
    /// root first), in publication order.
    pub(crate) transitions: alloc::vec::Vec<campaign_choregraph::DurablePublicationEvidence>,
    /// Profile and ticket of every request the session built, for selection publication events.
    policy_inputs: alloc::vec::Vec<(
        campaign_core::SelectionProfile,
        Option<campaign_core::EntropyTicket>,
    )>,
    /// Campaign's validation of the history the session last loaded; later loads and
    /// publications advance through it.
    projection: Option<campaign_choregraph::ValidatedControlProjection>,
    /// Campaign's cold-start validation of the admitted control root; publications before the
    /// first load advance through it.
    origin: campaign_choregraph::ValidatedControlProjection,
    /// Presentation run after every publication (none outside the claims fixtures).
    pub(crate) witness: Option<Witness>,
}

/// Derives one suite-owned identity.
///
/// # Errors
///
/// Returns a typed Campaign error when framing fails.
pub(crate) fn suite_id(
    kind: campaign_core::IdKind,
    name: &str,
) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
    Ok(campaign_core::identity::derive_id_with(
        kind,
        SUITE_DOMAIN,
        |writer| Ok(writer.bytes(name.as_bytes())),
    )?)
}

impl Session<'_, '_> {
    /// Candidate identity of an admitted operation handle.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` for an unknown handle.
    pub(crate) fn candidate_id(
        &self,
        handle: crate::vocabulary::OperationHandle,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        self.operations
            .iter()
            .find(|(known, _)| *known == handle)
            .map(|(_, id)| *id)
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "operation handle was admitted",
            })
    }

    /// Operation handle of a candidate identity.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` for a candidate the scenario never admitted.
    pub(crate) fn operation_of(
        &self,
        candidate_id: campaign_core::Id,
    ) -> crate::verdicts::ConformanceResult<crate::vocabulary::OperationHandle> {
        self.operations
            .iter()
            .find(|(_, id)| *id == candidate_id)
            .map(|(handle, _)| *handle)
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "candidate belongs to a scenario operation",
            })
    }

    /// Admitted moment of a handle.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` for an unknown handle.
    pub(crate) fn moment(
        &self,
        handle: crate::vocabulary::MomentHandle,
    ) -> crate::verdicts::ConformanceResult<MomentEntry> {
        self.moments
            .iter()
            .find(|entry| entry.handle == handle)
            .copied()
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "moment handle was admitted",
            })
    }

    /// Handle of an admitted moment identity.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` for a moment identity the scenario never admitted.
    pub(crate) fn moment_of(
        &self,
        moment_id: campaign_core::Id,
    ) -> crate::verdicts::ConformanceResult<crate::vocabulary::MomentHandle> {
        self.moments
            .iter()
            .find(|entry| entry.identity.moment_id() == moment_id)
            .map(|entry| entry.handle)
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "the accepted child is a scenario moment",
            })
    }

    /// Records one transcript entry.
    pub(crate) fn record(&mut self, entry: crate::ledger::TranscriptEntry) {
        self.harness.record(entry);
    }
}
