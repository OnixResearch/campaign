const FRAME_DOMAIN: &[u8] = b"onix.campaign.id";
const FRAME_BYTES: u8 = 1;
const FRAME_U64: u8 = 2;
const FRAME_I64: u8 = 3;
const FRAME_U32: u8 = 4;
const FRAME_ID: u8 = 5;
const FRAME_BOOL: u8 = 6;
const FRAME_SEQ_START: u8 = 7;
const FRAME_OPTION_NONE: u8 = 8;
const FRAME_OPTION_SOME: u8 = 9;

/// Immutable canonical identity transcript.
pub struct FrameWriter {
    hasher: blake3::Hasher,
}

impl FrameWriter {
    /// Creates a transcript with the fixed Campaign identity domain and version.
    #[must_use]
    #[cfg_attr(
        dylint_lib = "octet",
        allow(
            tigerstyle::concrete_construction_in_core,
            reason = "Campaign MODEL owner: BLAKE3 is a deterministic local primitive with no I/O or ambient state, not an external capability"
        )
    )]
    // r[impl campaign.identity]
    pub fn new(kind: super::IdKind, label: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(FRAME_DOMAIN);
        hasher.update(&[kind.code()]);
        hasher.update(&super::ID_VERSION_ONE.to_le_bytes());
        hasher.update(&(label.len() as u64).to_le_bytes());
        hasher.update(label);
        Self { hasher }
    }

    /// Appends a framed byte string.
    #[must_use]
    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        self.hasher.update(&[FRAME_BYTES]);
        self.hasher.update(&(bytes.len() as u64).to_le_bytes());
        self.hasher.update(bytes);
        self
    }

    /// Appends an unsigned 64-bit value.
    #[must_use]
    pub fn u64(mut self, value: u64) -> Self {
        self.hasher.update(&[FRAME_U64]);
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Appends a signed 64-bit value.
    #[must_use]
    pub fn i64(mut self, value: i64) -> Self {
        self.hasher.update(&[FRAME_I64]);
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Appends an unsigned 32-bit value.
    #[must_use]
    pub fn u32(mut self, value: u32) -> Self {
        self.hasher.update(&[FRAME_U32]);
        self.hasher.update(&value.to_le_bytes());
        self
    }

    /// Appends a Boolean value.
    #[must_use]
    pub fn bool(mut self, value: bool) -> Self {
        self.hasher.update(&[FRAME_BOOL]);
        self.hasher.update(&[u8::from(value)]);
        self
    }

    /// Appends an identity kind and digest.
    #[must_use]
    pub fn id(mut self, id: super::Id) -> Self {
        self.hasher.update(&[FRAME_ID]);
        self.hasher.update(&[id.kind.code()]);
        self.hasher.update(&id.digest);
        self
    }

    /// Appends an optional identity with an explicit discriminant.
    #[must_use]
    pub fn option_id(mut self, id: Option<super::Id>) -> Self {
        if let Some(value) = id {
            self.hasher.update(&[FRAME_OPTION_SOME]);
            self.id(value)
        } else {
            self.hasher.update(&[FRAME_OPTION_NONE]);
            self
        }
    }

    /// Appends the fixed-width length of a following sequence.
    #[must_use]
    pub fn seq_len(mut self, len: u64) -> Self {
        self.hasher.update(&[FRAME_SEQ_START]);
        self.u64(len)
    }

    /// Finishes the BLAKE3 transcript with its declared identity kind.
    #[must_use]
    pub fn finish(self, kind: super::IdKind) -> super::Id {
        let mut digest = [0; super::DIGEST_LENGTH];
        digest.copy_from_slice(self.hasher.finalize().as_bytes());
        super::Id { kind, digest }
    }
}

/// Derives an identity from a canonical ordered identity sequence.
#[must_use]
pub fn derive_id(kind: super::IdKind, label: &[u8], fields: &[super::Id]) -> super::Id {
    let mut writer = FrameWriter::new(kind, label).seq_len(fields.len() as u64);
    for field in fields {
        writer = writer.id(*field);
    }
    writer.finish(kind)
}

/// Derives an identity from a pure, ordered transcript builder.
///
/// # Errors
///
/// Returns a typed `CampaignError` from the field writer.
pub fn derive_id_with(
    kind: super::IdKind,
    label: &[u8],
    f: impl FnOnce(FrameWriter) -> crate::error::CampaignResult<FrameWriter>,
) -> crate::error::CampaignResult<super::Id> {
    let writer = f(FrameWriter::new(kind, label))?;
    Ok(writer.finish(kind))
}

#[cfg(test)]
mod tests {

    const PRE_REFACTOR_GOLDEN_LABEL: &[u8] = b"golden";
    const PRE_REFACTOR_GOLDEN_BYTES: &[u8] = b"payload";
    const PRE_REFACTOR_GOLDEN_U64: u64 = 42;
    const PRE_REFACTOR_GOLDEN_I64: i64 = -7;
    const PRE_REFACTOR_GOLDEN_U32: u32 = 9;
    const PRE_REFACTOR_GOLDEN_SEQUENCE_LENGTH: u64 = 2;
    const PRE_REFACTOR_GOLDEN_DIGEST_BYTE: u8 = 0xa5;
    const PRE_REFACTOR_POSITIVE_DIGEST: [u8; crate::identity::DIGEST_LENGTH] = [
        0xdc, 0x11, 0x19, 0xe7, 0x9a, 0x53, 0x63, 0x97, 0x53, 0x4c, 0x81, 0x79, 0xfa, 0x2c, 0x17,
        0xa8, 0x2a, 0x2e, 0xe0, 0xfd, 0x37, 0xe2, 0xfd, 0xa3, 0x88, 0xd4, 0xac, 0xe8, 0xc8, 0x06,
        0x7f, 0x91,
    ];
    const PRE_REFACTOR_NEGATIVE_DIGEST: [u8; crate::identity::DIGEST_LENGTH] = [
        0x1c, 0xf4, 0x5a, 0xeb, 0xff, 0xd9, 0x03, 0xee, 0x81, 0xcb, 0x1f, 0xd4, 0x07, 0x56, 0xc2,
        0xb8, 0x8b, 0x70, 0xfd, 0x11, 0xaf, 0x7f, 0xbc, 0x61, 0xe7, 0x71, 0xdb, 0x2a, 0x35, 0x2c,
        0x11, 0x32,
    ];

    fn golden_writer(value: bool) -> super::FrameWriter {
        super::FrameWriter::new(crate::identity::IdKind::State, PRE_REFACTOR_GOLDEN_LABEL)
            .bytes(PRE_REFACTOR_GOLDEN_BYTES)
            .u64(PRE_REFACTOR_GOLDEN_U64)
            .i64(PRE_REFACTOR_GOLDEN_I64)
            .u32(PRE_REFACTOR_GOLDEN_U32)
            .bool(value)
            .id(crate::identity::Id {
                kind: crate::identity::IdKind::Reason,
                digest: [PRE_REFACTOR_GOLDEN_DIGEST_BYTE; crate::identity::DIGEST_LENGTH],
            })
            .option_id(None)
            .seq_len(PRE_REFACTOR_GOLDEN_SEQUENCE_LENGTH)
    }

    #[test]
    fn matches_the_pre_refactor_positive_golden_value() {
        let identity = golden_writer(true).finish(crate::identity::IdKind::State);

        assert_eq!(identity.digest, PRE_REFACTOR_POSITIVE_DIGEST);
    }

    #[test]
    fn rejects_failed_field_encoding() {
        let result = super::derive_id_with(
            crate::identity::IdKind::State,
            PRE_REFACTOR_GOLDEN_LABEL,
            |_| Err(crate::error::CampaignError::ArithmeticOverflow),
        );
        assert_eq!(result, Err(crate::error::CampaignError::ArithmeticOverflow));
    }

    #[test]
    fn matches_the_pre_refactor_negative_golden_value() {
        let identity = golden_writer(false).finish(crate::identity::IdKind::State);

        assert_eq!(identity.digest, PRE_REFACTOR_NEGATIVE_DIGEST);
        assert_ne!(identity.digest, PRE_REFACTOR_POSITIVE_DIGEST);
    }
}
