//! Adapter-independent decision transcript with a BLAKE3 digest.
//!
//! Entries name decision kinds and abstract handles only, never consumer identities, so two
//! conforming adapters that run equal scenarios produce equal transcripts and equal digests.

const LABEL_TRANSCRIPT: &[u8] = b"campaign-conformance-decision-transcript-v1";

/// One decision kind label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EntryKind(&'static str);

/// Decision kind labels recorded by the suite.
pub(crate) mod kinds {
    /// Start of one named fixture; `detail` is the fixture name.
    pub(crate) const FIXTURE: super::EntryKind = super::EntryKind("fixture");
    /// Candidate batch admitted; `operations` are the admitted operations.
    pub(crate) const ADMITTED: super::EntryKind = super::EntryKind("admitted");
    /// Candidate operation coalesced into an admitted identity.
    pub(crate) const COALESCED: super::EntryKind = super::EntryKind("coalesced");
    /// Guidance admitted; `detail` is the dynamic.
    pub(crate) const GUIDED: super::EntryKind = super::EntryKind("guided");
    /// Selection planned; `ordinal` is the selection ordinal.
    pub(crate) const SELECTED: super::EntryKind = super::EntryKind("selected");
    /// Planner wait; `detail` is the wait reason.
    pub(crate) const WAITED: super::EntryKind = super::EntryKind("waited");
    /// Planner or admitted stop; `detail` is the stop class.
    pub(crate) const STOPPED: super::EntryKind = super::EntryKind("stopped");
    /// Result accepted; `moments` are the children Campaign accepted with it.
    pub(crate) const SETTLED: super::EntryKind = super::EntryKind("settled");
    /// Unobserved closure accepted.
    pub(crate) const CLOSED: super::EntryKind = super::EntryKind("closed");
    /// Candidate retired.
    pub(crate) const RETIRED: super::EntryKind = super::EntryKind("retired");
    /// Candidates pruned by the declared retention plan.
    pub(crate) const PRUNED: super::EntryKind = super::EntryKind("pruned");
    /// Forbidden input denied; `detail` is the typed denial.
    pub(crate) const DENIED: super::EntryKind = super::EntryKind("denied");
    /// Execution gate decision; `detail` is granted or refused.
    pub(crate) const GATED: super::EntryKind = super::EntryKind("gated");
    /// Differential comparison; `detail` names the separated answers.
    pub(crate) const COMPARED: super::EntryKind = super::EntryKind("compared");
    /// Fixture verdict; `detail` is the verdict class.
    pub(crate) const VERDICT: super::EntryKind = super::EntryKind("verdict");
}

/// One canonical transcript entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptEntry {
    /// Decision kind label.
    pub kind: &'static str,
    /// Adapter-independent detail (fixture name, reason class, typed denial).
    pub detail: alloc::string::String,
    /// Abstract operation handles in ascending order.
    pub operations: alloc::vec::Vec<crate::vocabulary::OperationHandle>,
    /// Abstract moment handles in ascending order.
    pub moments: alloc::vec::Vec<crate::vocabulary::MomentHandle>,
    /// Selection ordinal or zero.
    pub ordinal: u64,
}

impl TranscriptEntry {
    /// Entry with a kind and detail only.
    pub(crate) fn labelled(kind: EntryKind, detail: &str) -> Self {
        Self {
            kind: kind.0,
            detail: alloc::string::String::from(detail),
            operations: alloc::vec::Vec::new(),
            moments: alloc::vec::Vec::new(),
            ordinal: 0,
        }
    }

    /// Entry naming operations in canonical ascending order.
    pub(crate) fn with_operations(
        mut self,
        mut operations: alloc::vec::Vec<crate::vocabulary::OperationHandle>,
    ) -> Self {
        operations.sort();
        self.operations = operations;
        self
    }

    /// Entry naming moments in canonical ascending order.
    pub(crate) fn with_moments(
        mut self,
        mut moments: alloc::vec::Vec<crate::vocabulary::MomentHandle>,
    ) -> Self {
        moments.sort();
        self.moments = moments;
        self
    }

    /// Entry carrying a selection ordinal.
    pub(crate) const fn with_ordinal(mut self, ordinal: u64) -> Self {
        self.ordinal = ordinal;
        self
    }
}

/// Canonical sequence of decision kinds and abstract handles.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
// r[impl campaign.consumer_pilots]
pub struct DecisionTranscript {
    /// Entries in decision order.
    pub entries: alloc::vec::Vec<TranscriptEntry>,
}

impl DecisionTranscript {
    /// Appends one entry.
    pub(crate) fn record(&mut self, entry: TranscriptEntry) {
        self.entries.push(entry);
    }

    /// BLAKE3 digest over the canonical framing of every entry.
    ///
    /// # Errors
    ///
    /// Returns `ArithmeticOverflow` when a length cannot be framed.
    pub fn digest(
        &self,
    ) -> Result<[u8; campaign_core::DIGEST_LENGTH], campaign_core::CampaignError> {
        let mut writer =
            campaign_core::FrameWriter::new(campaign_core::IdKind::DerivedRecord, LABEL_TRANSCRIPT)
                .seq_len(length(self.entries.len())?);
        for entry in &self.entries {
            writer = write_entry(writer, entry)?;
        }
        Ok(writer.finish(campaign_core::IdKind::DerivedRecord).digest)
    }
}

fn length(len: usize) -> Result<u64, campaign_core::CampaignError> {
    u64::try_from(len).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)
}

fn write_entry(
    writer: campaign_core::FrameWriter,
    entry: &TranscriptEntry,
) -> Result<campaign_core::FrameWriter, campaign_core::CampaignError> {
    let mut writer = writer
        .bytes(entry.kind.as_bytes())
        .bytes(entry.detail.as_bytes())
        .u64(entry.ordinal)
        .seq_len(length(entry.operations.len())?);
    for operation in &entry.operations {
        writer = writer.u32(operation.0);
    }
    writer = writer.seq_len(length(entry.moments.len())?);
    for moment in &entry.moments {
        writer = writer.u32(moment.0);
    }
    Ok(writer)
}
