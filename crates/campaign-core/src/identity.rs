//! Canonical Campaign identity values and framed BLAKE3 derivation.

pub(crate) mod collection;
mod framing;
mod kind;

pub use collection::{sorted_copy, unique_sorted_ids};
pub use framing::{FrameWriter, derive_id, derive_id_with};
pub use kind::{DIGEST_LENGTH, ID_VERSION_ONE, Id, IdKind};

/// Converts a platform length to the fixed-width protocol representation.
pub(crate) fn usize_to_u64(value: usize) -> crate::error::CampaignResult<u64> {
    u64::try_from(value).map_err(|_| crate::error::CampaignError::ArithmeticOverflow)
}
