const LABEL_ENTROPY: &[u8] = b"entropy-ticket";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.entropy]
pub struct EntropyTicket {
    /// Public Campaign core API value.
    pub entropy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub profile_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub source_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub counter: u64,
    /// Public Campaign core API value.
    pub bytes: [u8; crate::identity::DIGEST_LENGTH],
}

impl EntropyTicket {
    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    pub fn derive(mut self) -> crate::error::CampaignResult<Self> {
        self.campaign_id
            .require_kind(crate::identity::IdKind::Campaign)?;
        self.state_id.require_kind(crate::identity::IdKind::State)?;
        self.profile_id
            .require_kind(crate::identity::IdKind::SelectionProfile)?;
        self.source_id
            .require_kind(crate::identity::IdKind::EntropyProfile)?;
        self.entropy_id = crate::identity::derive_id_with(
            crate::identity::IdKind::EntropyTicket,
            LABEL_ENTROPY,
            |mut writer| {
                writer = writer.id(self.campaign_id);
                writer = writer.id(self.state_id);
                writer = writer.id(self.profile_id);
                writer = writer.id(self.source_id);
                writer = writer.u64(self.counter);
                writer = writer.bytes(&self.bytes);
                Ok(writer)
            },
        )?;
        Ok(self)
    }

    /// Public Campaign core API operation.
    ///
    /// # Errors
    ///
    /// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
    // r[impl campaign.entropy.invalid]
    pub fn validate(
        &self,
        campaign_id: crate::identity::Id,
        state_id: crate::identity::Id,
        profile_id: crate::identity::Id,
    ) -> crate::error::CampaignResult<()> {
        if self.campaign_id != campaign_id
            || self.state_id != state_id
            || self.profile_id != profile_id
        {
            return Err(crate::error::CampaignError::InvalidEntropy);
        }
        let derived = self.clone().derive()?;
        if derived.entropy_id != self.entropy_id {
            return Err(crate::error::CampaignError::InvalidEntropy);
        }
        Ok(())
    }

    /// Explore-branch draw: little-endian `u64` over ticket bytes `0..8`.
    #[must_use]
    pub fn explore_draw(&self) -> u64 {
        let mut prefix = [0u8; super::ENTROPY_PREFIX_BYTES];
        prefix.copy_from_slice(&self.bytes[..super::ENTROPY_PREFIX_BYTES]);
        u64::from_le_bytes(prefix)
    }

    /// Explore-index draw: little-endian `u64` over ticket bytes `8..16`.
    #[must_use]
    pub fn index_draw(&self) -> u64 {
        let mut window = [0u8; super::ENTROPY_PREFIX_BYTES];
        window.copy_from_slice(
            &self.bytes[super::ENTROPY_PREFIX_BYTES..super::ENTROPY_INDEX_DRAW_END],
        );
        u64::from_le_bytes(window)
    }
}
