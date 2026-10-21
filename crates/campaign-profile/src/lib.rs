#![forbid(unsafe_code)]
#![cfg_attr(dylint_lib = "octet", feature(register_tool))]
#![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]

//! Strict loader for the exported `campaign.profile.v2` runtime profile.
//!
//! Human-authored profiles are typed Nickel under `config/`; their JSON export is the only runtime input.
//! This crate rejects any JSON the Nickel contracts would reject, then builds the Campaign core values
//! through their own constructors. It does not read files, the environment, or clocks: the embedded default
//! is a crate-local copy of `config/generated/default-profile.json` that `scripts/check-config.sh` and the
//! infra checker (`campaign.configuration.stale`) require to be byte-identical to the projection.

mod checks;
mod choice;
mod closed;
mod failure;
mod load;
mod wire;

#[cfg(test)]
mod tests;

pub use choice::{DeclaredSelectionMode, SelectionRuntime};
pub use failure::{DecodeCategory, ProfileError, ProfileResult};
pub use load::{CampaignProfile, PROFILE_SCHEMA, ProfileOwners, default_profile};
