//! Minimal float-scoring adapter.
//!
//! The consumer scores candidates with non-negative `f64` values (the suite's float view). Its
//! adapter converts every score to a bounded integer rank with a declared fixed-point conversion
//! profile (`floor(score * 2^FRACTION_BITS)`, computed exactly from the IEEE-754 bits) and binds
//! that profile into both its adapter identity and its guidance ordering profile.

mod conversion;
mod mapping;

pub use conversion::{FRACTION_BITS, fixed_point_rank};

/// Frame domain of every identity this adapter derives.
const DOMAIN: &[u8] = b"campaign-conformance-float-scores";

/// Version of the declared conversion profile.
const CONVERSION_VERSION: u32 = 1;

/// Float-scoring adapter with a fixed-point conversion profile.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FixedPointAdapter;

impl FixedPointAdapter {
    /// Creates the adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Identity of the declared score conversion profile.
    ///
    /// # Errors
    ///
    /// Returns a typed Campaign error when framing fails.
    pub fn conversion_profile_id() -> crate::verdicts::ConformanceResult<campaign_core::Id> {
        derived(
            campaign_core::IdKind::GuidanceProfile,
            "fixed-point-conversion",
            |writer| writer.u32(CONVERSION_VERSION).u32(FRACTION_BITS),
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

// r[impl campaign.guidance.float]
fn identities() -> crate::verdicts::ConformanceResult<crate::port::AdapterIdentities> {
    let conversion = FixedPointAdapter::conversion_profile_id()?;
    let identities = crate::port::AdapterIdentities {
        adapter_id: derived(campaign_core::IdKind::Adapter, "adapter", |writer| {
            writer.id(conversion)
        })?,
        projection_schema_id: named(campaign_core::IdKind::Projection, "projection-schema")?,
        control_domain_id: named(campaign_core::IdKind::ControlDomain, "control-domain")?,
        guidance_profile_id: conversion,
        entropy_source_id: named(campaign_core::IdKind::EntropyProfile, "scripted-draws")?,
        operation_schema_ids: alloc::vec![named(
            campaign_core::IdKind::OperationSchema,
            "scored-step"
        )?],
        resource_dimension_ids: alloc::vec![named(
            campaign_core::IdKind::ResourceDimension,
            "step-budget"
        )?],
    };
    debug_assert_eq!(identities.guidance_profile_id, conversion);
    debug_assert_ne!(identities.adapter_id, conversion);
    Ok(identities)
}
