mod decay;
#[cfg(test)]
mod tests;

const LABEL_SELECTION_PROFILE: &[u8] = b"selection-profile";
const MODE_RANKED: u32 = 1;
const MODE_EXPLORATORY: u32 = 2;
const DECAY_NONE: u32 = 1;
const DECAY_SUBTRACT_FROM_FIRST_RANK: u32 = 2;
const STOPPING_NO_FINDING_LIMIT: u32 = 1;
const STOPPING_FINDING_LIMIT: u32 = 2;
const IMPLEMENTATION_PRODUCTION_COMPILED: u32 = 1;
const IMPLEMENTATION_REFERENCE_EVIDENCE: u32 = 2;
/// Frame label of the Reason identity for the finding-limit policy stop.
const LABEL_FINDING_LIMIT_REASON: &[u8] = b"stopping-rule-finding-limit";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.entropy]
pub enum SelectionMode {
    /// Public Campaign core API variant.
    Ranked,
    /// Public Campaign core API variant.
    Exploratory {
        /// Exploration threshold in parts per million.
        threshold_parts_per_million: u64,
        /// Required explicit entropy source identity.
        entropy_source_id: crate::identity::Id,
        /// Declared rank-dimension indices that order candidates on the explore branch.
        exploration_order: alloc::vec::Vec<u32>,
    },
}

// r[impl campaign.selection.decay]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value: named selection-count decay arithmetic with fixed wire codes.
pub enum SelectionCountDecay {
    /// Ranks are used unchanged.
    None,
    /// The first rank is lowered by `penalty_per_selection * selection_count` (checked).
    SubtractFromFirstRank {
        /// Positive penalty applied per prior selection of the candidate.
        penalty_per_selection: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Declared stopping rule bound into the selection profile identity.
// r[impl campaign.fd.scheduling]
pub struct StoppingRule {
    /// Optional positive campaign finding count that requests a policy stop.
    pub finding_limit: Option<u64>,
}

impl StoppingRule {
    /// Reason identity of the policy stop this rule requests.
    ///
    /// The identity is a `Reason`-kind BLAKE3 frame with label `stopping-rule-finding-limit` over the
    /// declared `finding_limit` (`u64`), so equal rules always name the same stop reason. Returns
    /// `None` when the rule declares no finding limit: findings then never stop the campaign.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` if identity framing fails.
    pub fn policy_reason_id(&self) -> crate::error::CampaignResult<Option<crate::identity::Id>> {
        let Some(limit) = self.finding_limit else {
            return Ok(None);
        };
        let reason_id = crate::identity::derive_id_with(
            crate::identity::IdKind::Reason,
            LABEL_FINDING_LIMIT_REASON,
            |writer| Ok(writer.u64(limit)),
        )?;
        debug_assert_eq!(reason_id.kind, crate::identity::IdKind::Reason);
        Ok(Some(reason_id))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Content-addressed selection profile.
pub struct SelectionProfile {
    /// Content-derived profile identity.
    pub profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub scheduling_policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub admission_policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub coalescing_policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub mode: SelectionMode,
    /// Public Campaign core API value.
    pub decay: SelectionCountDecay,
    /// Declared stopping rule.
    pub stopping: StoppingRule,
    /// Selected policy implementation; production planning rejects reference evidence.
    pub implementation: crate::discipline::PolicyImplementation,
}

impl SelectionProfile {
    /// Derives the content identity from every declared profile fact.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when a declared fact is missing, crossed, or invalid.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.validate_without_profile_id()?;
        self.profile_id = crate::identity::derive_id_with(
            crate::identity::IdKind::SelectionProfile,
            LABEL_SELECTION_PROFILE,
            |writer| {
                let writer = writer
                    .id(self.scheduling_policy_id)
                    .id(self.admission_policy_id)
                    .id(self.coalescing_policy_id);
                let writer = write_mode(writer, &self.mode)?;
                let writer = match &self.decay {
                    SelectionCountDecay::None => writer.u32(DECAY_NONE),
                    SelectionCountDecay::SubtractFromFirstRank {
                        penalty_per_selection,
                    } => writer
                        .u32(DECAY_SUBTRACT_FROM_FIRST_RANK)
                        .u64(*penalty_per_selection),
                };
                let writer = match self.stopping.finding_limit {
                    None => writer.u32(STOPPING_NO_FINDING_LIMIT),
                    Some(limit) => writer.u32(STOPPING_FINDING_LIMIT).u64(limit),
                };
                Ok(match &self.implementation {
                    crate::discipline::PolicyImplementation::ProductionCompiled { policy_id } => {
                        writer
                            .u32(IMPLEMENTATION_PRODUCTION_COMPILED)
                            .id(*policy_id)
                    }
                    crate::discipline::PolicyImplementation::ReferenceEvidence { reference_id } => {
                        writer
                            .u32(IMPLEMENTATION_REFERENCE_EVIDENCE)
                            .id(*reference_id)
                    }
                })
            },
        )?;
        debug_assert_eq!(
            self.profile_id.kind,
            crate::identity::IdKind::SelectionProfile
        );
        debug_assert!(!self.profile_id.is_absent());
        Ok(self)
    }

    /// Validates every declared fact and re-derives the profile identity.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when a fact is invalid or `MalformedIdentity` when
    /// the supplied identity differs from the content-derived one.
    pub fn validate(&self) -> crate::error::CampaignResult<()> {
        self.profile_id
            .require_kind(crate::identity::IdKind::SelectionProfile)?;
        let derived = self.clone().derive()?;
        if derived.profile_id != self.profile_id {
            return Err(crate::error::CampaignError::MalformedIdentity);
        }
        Ok(())
    }

    fn validate_without_profile_id(&self) -> crate::error::CampaignResult<()> {
        self.scheduling_policy_id
            .require_kind(crate::identity::IdKind::SchedulingProfile)?;
        self.admission_policy_id
            .require_kind(crate::identity::IdKind::AdmissionPolicy)?;
        self.coalescing_policy_id
            .require_kind(crate::identity::IdKind::CoalescingPolicy)?;
        validate_mode(&self.mode)?;
        if matches!(
            self.decay,
            SelectionCountDecay::SubtractFromFirstRank {
                penalty_per_selection: 0
            }
        ) {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        if self.stopping.finding_limit == Some(0) {
            return Err(crate::error::CampaignError::InvalidConfig);
        }
        match &self.implementation {
            crate::discipline::PolicyImplementation::ProductionCompiled { policy_id } => {
                policy_id.require_kind(crate::identity::IdKind::Policy)?;
            }
            crate::discipline::PolicyImplementation::ReferenceEvidence { reference_id } => {
                reference_id.require_kind(crate::identity::IdKind::Policy)?;
            }
        }
        Ok(())
    }
}

fn validate_mode(mode: &SelectionMode) -> crate::error::CampaignResult<()> {
    let SelectionMode::Exploratory {
        threshold_parts_per_million,
        entropy_source_id,
        exploration_order,
    } = mode
    else {
        return Ok(());
    };
    entropy_source_id.require_kind(crate::identity::IdKind::EntropyProfile)?;
    if *threshold_parts_per_million > super::ENTROPY_SCALE {
        return Err(crate::error::CampaignError::InvalidConfig);
    }
    if exploration_order.is_empty() {
        return Err(crate::error::CampaignError::InvalidConfig);
    }
    let mut sorted = exploration_order.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != exploration_order.len() {
        return Err(crate::error::CampaignError::InvalidConfig);
    }
    debug_assert!(*threshold_parts_per_million <= super::ENTROPY_SCALE);
    debug_assert!(!exploration_order.is_empty());
    Ok(())
}

fn write_mode(
    writer: crate::identity::FrameWriter,
    mode: &SelectionMode,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    match mode {
        SelectionMode::Ranked => Ok(writer.u32(MODE_RANKED)),
        SelectionMode::Exploratory {
            threshold_parts_per_million,
            entropy_source_id,
            exploration_order,
        } => {
            let mut writer = writer
                .u32(MODE_EXPLORATORY)
                .u64(*threshold_parts_per_million)
                .id(*entropy_source_id)
                .seq_len(crate::identity::usize_to_u64(exploration_order.len())?);
            for index in exploration_order {
                writer = writer.u32(*index);
            }
            Ok(writer)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct SelectionRequest {
    /// Public Campaign core API value.
    pub request_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub profile: SelectionProfile,
    /// Public Campaign core API value.
    pub entropy: Option<super::entropy::EntropyTicket>,
    /// Public Campaign core API value.
    pub expected_generation: u64,
    /// Public Campaign core API value.
    pub expected_head_id: crate::identity::Id,
}
