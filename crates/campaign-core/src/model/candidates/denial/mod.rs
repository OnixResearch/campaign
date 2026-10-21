//! Explicit denial of a candidate batch by the declared `Reject` duplicate rule.

const LABEL_ADMISSION_REFUSAL: &[u8] = b"candidate-admission-refusal";
const REFUSAL_DUPLICATE_OF_ADMITTED: u32 = 1;
const REFUSAL_DUPLICATE_IN_BATCH: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Where a refused duplicate candidate identity was already present.
pub enum DuplicateOrigin {
    /// The identity is already admitted to the campaign.
    AdmittedCandidate,
    /// The identity was submitted more than once in the same batch.
    SameBatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Reason a declared `Reject` rule refused a whole candidate batch.
pub struct AdmissionRefusalReason {
    /// First (canonical order) submitted candidate identity that is a duplicate.
    pub candidate_id: crate::identity::Id,
    /// Where the identity was already present.
    pub duplicate_of: DuplicateOrigin,
}

impl AdmissionRefusalReason {
    /// Frames the reason as its fixed wire code followed by the duplicate candidate identity.
    #[must_use]
    pub fn write(self, writer: crate::identity::FrameWriter) -> crate::identity::FrameWriter {
        let code = if self.duplicate_of == DuplicateOrigin::SameBatch {
            REFUSAL_DUPLICATE_IN_BATCH
        } else {
            REFUSAL_DUPLICATE_OF_ADMITTED
        };
        writer.u32(code).id(self.candidate_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Policy-visible refusal of one submitted candidate batch; no control event is emitted.
pub struct AdmissionRefusal {
    /// Content-derived `DerivedRecord` identity of the refusal.
    pub refusal_id: crate::identity::Id,
    /// Campaign identity.
    pub campaign_id: crate::identity::Id,
    /// State the batch was submitted against; it is unchanged by the refusal.
    pub state_id: crate::identity::Id,
    /// Submitted candidates, derived and in canonical identity order.
    pub candidates: alloc::vec::Vec<super::CandidateRecord>,
    /// Declared admission rule that refused the batch.
    pub admission_rule: super::AdmissionRule,
    /// Admission policy identity derived from `admission_rule`.
    pub admission_policy_id: crate::identity::Id,
    /// Declared coalescing policy identity of the submission.
    pub coalescing_policy_id: crate::identity::Id,
    /// Rejection reason.
    pub reason: AdmissionRefusalReason,
}

impl AdmissionRefusal {
    /// Derives the refusal identity from every bound fact.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when a bound identity is missing or framing fails.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.campaign_id
            .require_kind(crate::identity::IdKind::Campaign)?;
        self.state_id.require_kind(crate::identity::IdKind::State)?;
        self.admission_policy_id
            .require_kind(crate::identity::IdKind::AdmissionPolicy)?;
        self.coalescing_policy_id
            .require_kind(crate::identity::IdKind::CoalescingPolicy)?;
        self.refusal_id = crate::identity::derive_id_with(
            crate::identity::IdKind::DerivedRecord,
            LABEL_ADMISSION_REFUSAL,
            |writer| {
                let mut writer = writer
                    .id(self.campaign_id)
                    .id(self.state_id)
                    .id(self.admission_policy_id)
                    .id(self.coalescing_policy_id)
                    .seq_len(crate::identity::usize_to_u64(self.candidates.len())?);
                for candidate in &self.candidates {
                    writer = writer.id(candidate.candidate_id);
                }
                Ok(self.reason.write(writer))
            },
        )?;
        debug_assert_eq!(self.refusal_id.kind, crate::identity::IdKind::DerivedRecord);
        Ok(self)
    }
}
