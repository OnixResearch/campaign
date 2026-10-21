// campaign-source: branchcontext
//! Animus-shaped non-VM candidate adapter.
//!
//! The consumer explores candidate edits of workspace snapshots. Every consumer fact is an opaque
//! identity: candidates, workspace references (one per exploration moment), and validator
//! references (one per observed execution, reported as an opaque derived record). Ranks are the
//! suite's integer levels unchanged. Execution is granted only through Campaign's publication
//! gate, and no consumer signal (rank, selection, exit status, validator reference) ever becomes
//! a receipt claim: acceptance and promotion authority stay with the consumer outside Campaign.

/// Frame domain of every identity this adapter derives.
const DOMAIN: &[u8] = b"campaign-conformance-animus-shaped";

/// Choregraph parent role linking a discovered workspace to the workspace it was explored from.
const WORKSPACE_PARENT_ROLE: &str = "workspace-parent";

/// Consumer reason labels.
const REASON_LABELS: [(crate::vocabulary::ScenarioReason, &str); 5] = [
    (
        crate::vocabulary::ScenarioReason::Retirement,
        "reason-retire-candidate",
    ),
    (
        crate::vocabulary::ScenarioReason::Pruning,
        "reason-prune-candidates",
    ),
    (
        crate::vocabulary::ScenarioReason::Revision,
        "reason-revise-ranking",
    ),
    (
        crate::vocabulary::ScenarioReason::Closure,
        "reason-workspace-lost",
    ),
    (
        crate::vocabulary::ScenarioReason::ConsumerStop,
        "reason-operator-stop",
    ),
];

/// Animus-shaped non-VM candidate adapter with opaque identities.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
// r[impl campaign.consumer_pilots]
// r[impl campaign.non_vm_candidate_fixture]
pub struct OpaqueCandidateAdapter;

impl OpaqueCandidateAdapter {
    /// Creates the adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Opaque workspace reference of one abstract moment.
    ///
    /// # Errors
    ///
    /// Returns a typed Campaign error when framing fails.
    pub fn workspace_reference(
        moment: &crate::vocabulary::ScenarioMoment,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        derived(
            campaign_core::IdKind::SourceFacts,
            "workspace-reference",
            |writer| {
                writer
                    .u32(moment.handle.0)
                    .u32(moment.parent.map_or(moment.handle.0, |parent| parent.0))
            },
        )
    }

    /// Opaque validator reference recorded for one executed selection.
    ///
    /// # Errors
    ///
    /// Returns a typed Campaign error when framing fails.
    pub fn validator_reference(
        selection_id: campaign_core::Id,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        derived(
            campaign_core::IdKind::DerivedRecord,
            "validator-reference",
            |writer| writer.id(selection_id),
        )
    }
}

fn derived(
    kind: campaign_core::IdKind,
    label: &str,
    fields: impl FnOnce(campaign_core::FrameWriter) -> campaign_core::FrameWriter,
) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
    Ok(campaign_core::identity::derive_id_with(
        kind,
        DOMAIN,
        |writer| Ok(fields(writer.bytes(label.as_bytes()))),
    )?)
}

fn named(
    kind: campaign_core::IdKind,
    label: &str,
) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
    derived(kind, label, |writer| writer)
}

fn identities() -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
    Ok(crate::port::AdapterIdentities {
        adapter_id: named(campaign_core::IdKind::Adapter, "adapter")?,
        projection_schema_id: named(campaign_core::IdKind::Projection, "projection-schema")?,
        control_domain_id: named(campaign_core::IdKind::ControlDomain, "control-domain")?,
        guidance_profile_id: named(campaign_core::IdKind::GuidanceProfile, "integer-levels")?,
        entropy_source_id: named(campaign_core::IdKind::EntropyProfile, "explicit-draws")?,
        operation_schema_ids: alloc::vec![named(
            campaign_core::IdKind::OperationSchema,
            "candidate-edit"
        )?],
        resource_dimension_ids: alloc::vec![named(
            campaign_core::IdKind::ResourceDimension,
            "validation-run"
        )?],
    })
}

impl crate::port::MomentMapping for OpaqueCandidateAdapter {
    fn identities(&self) -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
        identities()
    }

    fn exploration_draft(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        parent_event: Option<campaign_choregraph::EventId>,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::EventDraft> {
        let workspace = Self::workspace_reference(moment)?;
        let parents = parent_event.map_or_else(alloc::vec::Vec::new, |event| {
            alloc::vec![campaign_choregraph::ParentLink {
                role: alloc::string::String::from(WORKSPACE_PARENT_ROLE),
                event,
            }]
        });
        Ok(campaign_choregraph::EventDraft::v1(
            campaign_choregraph::EventClass::Application,
            campaign_choregraph::digest_from_id(workspace),
            parents,
            campaign_choregraph::Digest::derive(DOMAIN, b"workspace-moment"),
        ))
    }

    fn moment_identity(
        &mut self,
        moment: &crate::vocabulary::ScenarioMoment,
        event: campaign_choregraph::EventId,
    ) -> crate::verdicts::ConformanceResult<crate::port::MomentIdentity> {
        let workspace = Self::workspace_reference(moment)?;
        if moment.parent.is_none() {
            let moment_id = derived(
                campaign_core::IdKind::RootMoment,
                "root-workspace",
                |writer| writer.bytes(&event.bytes()).id(workspace),
            )?;
            return Ok(crate::port::MomentIdentity::Root { moment_id });
        }
        Ok(crate::port::MomentIdentity::Child {
            moment_id: derived(
                campaign_core::IdKind::ChildMoment,
                "child-workspace",
                |writer| writer.bytes(&event.bytes()).id(workspace),
            )?,
            event_payload_id: derived(
                campaign_core::IdKind::EventPayload,
                "workspace-event",
                |writer| writer.id(workspace),
            )?,
        })
    }

    fn reason(
        &mut self,
        reason: crate::vocabulary::ScenarioReason,
    ) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        let label = REASON_LABELS
            .iter()
            .find(|(known, _)| *known == reason)
            .map(|(_, label)| *label)
            .ok_or(crate::verdicts::ConformanceError::InvalidIdentities)?;
        named(campaign_core::IdKind::Reason, label)
    }
}

mod records;
