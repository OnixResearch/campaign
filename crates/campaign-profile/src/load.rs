//! Validated `campaign.profile.v2` value and the embedded default profile.

/// Exact schema string of the supported profile export.
pub const PROFILE_SCHEMA: &str = "campaign.profile.v2";

/// Crate-local copy of `config/generated/default-profile.json`; `scripts/check-config.sh` and the infra checker
/// fail when it is not byte-identical to the projection.
pub(crate) const DEFAULT_PROFILE_JSON: &str = include_str!("../profiles/default.json");

/// Named owner of every profile value group; each owner is non-empty.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileOwners {
    /// Owner of `bounds`.
    pub bounds: String,
    /// Owner of `selection`.
    pub selection: String,
    /// Owner of `stopping`.
    pub stopping: String,
    /// Owner of `retention`.
    pub retention: String,
    /// Owner of `admission`.
    pub admission: String,
    /// Owner of `implementation`.
    pub implementation: String,
    /// Owner of `versions`.
    pub versions: String,
    /// Owner of `receipt_non_claims`.
    pub receipt_non_claims: String,
}

impl ProfileOwners {
    fn validate(&self) -> crate::failure::ProfileResult<()> {
        let owners = [
            ("bounds", &self.bounds),
            ("selection", &self.selection),
            ("stopping", &self.stopping),
            ("retention", &self.retention),
            ("admission", &self.admission),
            ("implementation", &self.implementation),
            ("versions", &self.versions),
            ("receipt_non_claims", &self.receipt_non_claims),
        ];
        if let Some((group, _)) = owners.iter().find(|(_, owner)| owner.is_empty()) {
            return Err(crate::failure::ProfileError::EmptyOwner { group });
        }
        Ok(())
    }
}

/// A validated `campaign.profile.v2` runtime profile expressed in Campaign core values.
#[derive(Clone, Debug, Eq, PartialEq)]
// r[impl campaign.configuration]
pub struct CampaignProfile {
    /// Owner of every value group.
    pub owners: ProfileOwners,
    /// Exploration bounds with their content-derived `bounds_id`.
    pub bounds: campaign_core::ExplorationBounds,
    /// Declared selection mode.
    pub selection_mode: crate::choice::DeclaredSelectionMode,
    /// Declared selection-count decay.
    pub decay: campaign_core::SelectionCountDecay,
    /// Declared stopping rule.
    pub stopping: campaign_core::StoppingRule,
    /// Retention rank order template: unique rank-dimension indices below `max_rank_dimensions`.
    pub retention_rank_order: Vec<u32>,
    /// Declared candidate admission rule.
    pub admission: campaign_core::AdmissionRule,
    /// Receipt non-claims in canonical `NonClaim::ALL` order.
    pub receipt_non_claims: [campaign_core::NonClaim; campaign_core::NON_CLAIM_COUNT],
}

impl CampaignProfile {
    /// Decodes and validates an exported `campaign.profile.v2` JSON document.
    ///
    /// # Errors
    ///
    /// Returns `ProfileError::Decode` for text outside the closed profile shape (unknown key, missing key,
    /// `null`, float or negative integer, unknown tag, a record that is not a JSON object, a tag that is not
    /// a JSON string), and a specific `ProfileError` for every value the Nickel contracts reject.
    // r[impl campaign.configuration]
    pub fn from_json(text: &str) -> crate::failure::ProfileResult<Self> {
        let mut deserializer = serde_json::Deserializer::from_str(text);
        let document: crate::wire::ProfileDocument = crate::closed::record(&mut deserializer)?;
        deserializer.end()?;
        Self::from_document(document)
    }

    // r[impl campaign.configuration.invalid]
    fn from_document(
        document: crate::wire::ProfileDocument,
    ) -> crate::failure::ProfileResult<Self> {
        if document.schema != PROFILE_SCHEMA {
            return Err(crate::failure::ProfileError::UnsupportedProfileSchema);
        }
        if document.versions.campaign_schema != u64::from(campaign_core::CAMPAIGN_SCHEMA_VERSION) {
            return Err(crate::failure::ProfileError::UnsupportedCampaignSchema);
        }
        if document.versions.identity_version != u64::from(campaign_core::identity::ID_VERSION_ONE)
        {
            return Err(crate::failure::ProfileError::UnsupportedIdentityVersion);
        }
        if let crate::wire::ImplementationTag::ReferenceEvidence = document.implementation {
            return Err(crate::failure::ProfileError::ReferenceImplementation);
        }
        document.owners.validate()?;
        let bounds = document.bounds.into_bounds()?;
        let selection = document.selection.into_declared(&bounds)?;
        let stopping = match document.stopping.finding_limit {
            Some(0) => return Err(crate::failure::ProfileError::ZeroFindingLimit),
            Some(limit) if limit > bounds.max_findings => {
                return Err(crate::failure::ProfileError::FindingLimitExceedsFindingBound);
            }
            finding_limit => campaign_core::StoppingRule { finding_limit },
        };
        let retention_rank_order = crate::checks::ordering::validate(
            crate::checks::ordering::RETENTION_RANK_ORDER,
            document.retention.rank_order,
            bounds.max_rank_dimensions,
        )?;
        let duplicate_candidate = if document.admission.duplicate_candidate
            == crate::wire::DuplicateCandidateTag::Coalesce
        {
            campaign_core::DuplicateCandidateRule::Coalesce
        } else {
            campaign_core::DuplicateCandidateRule::Reject
        };
        let receipt_non_claims = crate::checks::claims::validate(&document.receipt_non_claims)?;
        Ok(Self {
            owners: document.owners,
            bounds,
            selection_mode: selection.mode,
            decay: selection.decay,
            stopping,
            retention_rank_order,
            admission: campaign_core::AdmissionRule {
                duplicate_candidate,
            },
            receipt_non_claims,
        })
    }

    /// Derives the `SelectionProfile` for this profile and the consumer's runtime identities.
    ///
    /// The admission policy identity is derived from the declared admission rule; the implementation is the
    /// supplied production policy.
    ///
    /// # Errors
    ///
    /// Returns `MissingEntropySource` or `UnexpectedEntropySource` when the entropy source does not match the
    /// declared mode, and `ProfileError::Campaign` when `SelectionProfile::derive` rejects a runtime identity.
    pub fn selection_profile(
        &self,
        runtime: crate::choice::SelectionRuntime,
    ) -> crate::failure::ProfileResult<campaign_core::SelectionProfile> {
        let profile = campaign_core::SelectionProfile {
            profile_id: campaign_core::Id::absent(),
            scheduling_policy_id: runtime.scheduling_policy_id,
            admission_policy_id: self.admission.admission_policy_id()?,
            coalescing_policy_id: runtime.coalescing_policy_id,
            mode: self
                .selection_mode
                .with_entropy_source(runtime.entropy_source_id)?,
            decay: self.decay.clone(),
            stopping: self.stopping.clone(),
            implementation: campaign_core::PolicyImplementation::ProductionCompiled {
                policy_id: runtime.production_policy_id,
            },
        }
        .derive()?;
        debug_assert!(!profile.profile_id.is_absent());
        Ok(profile)
    }
}

/// Loads the embedded default profile, a checked copy of `config/generated/default-profile.json`.
///
/// # Errors
///
/// Returns a `ProfileError` only if the embedded copy is invalid, which the crate tests rule out.
// r[impl campaign.configuration]
pub fn default_profile() -> crate::failure::ProfileResult<CampaignProfile> {
    CampaignProfile::from_json(DEFAULT_PROFILE_JSON)
}
