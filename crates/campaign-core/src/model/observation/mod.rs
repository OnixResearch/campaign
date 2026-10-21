const LABEL_RESULT: &[u8] = b"result";

const LABEL_RESERVATION: &[u8] = b"child-reservation";

const LABEL_DISCOVERY: &[u8] = b"discovery";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct ChildObservation {
    /// Public Campaign core API value.
    pub reservation_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub child_moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub discovery_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub event_payload_id: crate::identity::Id,
}

impl ChildObservation {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    // r[impl campaign.fd.reservation]
    // r[impl campaign.fork]
    pub fn derive(
        mut self,
        campaign_id: crate::identity::Id,
        selection_id: crate::identity::Id,
    ) -> crate::error::CampaignResult<Self> {
        self.child_moment_id
            .require_kind(crate::identity::IdKind::ChildMoment)?;
        self.event_payload_id
            .require_kind(crate::identity::IdKind::EventPayload)?;
        self.reservation_id = crate::identity::derive_id_with(
            crate::identity::IdKind::ChildReservation,
            LABEL_RESERVATION,
            |mut writer| {
                writer = writer.id(campaign_id);
                writer = writer.id(selection_id);
                writer = writer.id(self.child_moment_id);
                writer = writer.id(self.event_payload_id);
                Ok(writer)
            },
        )?;
        self.discovery_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Discovery,
            LABEL_DISCOVERY,
            |mut writer| {
                writer = writer.id(campaign_id);
                writer = writer.id(self.child_moment_id);
                writer = writer.id(self.event_payload_id);
                Ok(writer)
            },
        )?;
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.findings]
pub struct FindingRef {
    /// Public Campaign core API value.
    pub finding_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub priority_facts_id: crate::identity::Id,
}

impl FindingRef {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn validate(&self) -> crate::error::CampaignResult<()> {
        self.finding_id
            .require_kind(crate::identity::IdKind::Finding)?;
        self.priority_facts_id
            .require_kind(crate::identity::IdKind::SourceFacts)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.observations]
pub struct ResultObservation {
    /// Public Campaign core API value.
    pub result_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub selection_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub parent_moment_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub operation_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub adapter_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub payload_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub children: alloc::vec::Vec<ChildObservation>,
    /// Public Campaign core API value.
    pub findings: alloc::vec::Vec<FindingRef>,
    /// Public Campaign core API value.
    pub derived_records: alloc::vec::Vec<crate::identity::Id>,
}

impl ResultObservation {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.children.sort_by_key(|child| child.child_moment_id);
        self.findings.sort_by_key(|finding| finding.finding_id);
        self.derived_records.sort();
        self.result_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Result,
            LABEL_RESULT,
            |mut writer| {
                writer = writer.id(self.campaign_id);
                writer = writer.id(self.selection_id);
                writer = writer.id(self.parent_moment_id);
                writer = writer.id(self.operation_id);
                writer = writer.id(self.adapter_id);
                writer = writer.id(self.payload_id);
                writer = writer.seq_len(crate::identity::usize_to_u64(self.children.len())?);
                for child in &self.children {
                    writer = writer.id(child.reservation_id);
                    writer = writer.id(child.child_moment_id);
                    writer = writer.id(child.discovery_id);
                    writer = writer.id(child.event_payload_id);
                }
                writer = writer.seq_len(crate::identity::usize_to_u64(self.findings.len())?);
                for finding in &self.findings {
                    writer = writer.id(finding.finding_id);
                    writer = writer.id(finding.priority_facts_id);
                }
                writer = writer.seq_len(crate::identity::usize_to_u64(self.derived_records.len())?);
                for record in &self.derived_records {
                    writer = writer.id(*record);
                }
                Ok(writer)
            },
        )?;
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct ResultEvent {
    /// Public Campaign core API value.
    pub event_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_before_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub observation: ResultObservation,
}
