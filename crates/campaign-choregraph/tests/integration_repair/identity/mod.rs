pub(crate) fn observed<T, E: std::fmt::Debug>(value: Result<T, E>) -> Result<T, String> {
    value.map_err(|error| format!("{error:?}"))
}
pub(crate) fn derive(kind: campaign_core::IdKind, label: &[u8]) -> campaign_core::Id {
    campaign_core::derive_id(kind, label, &[])
}
pub(crate) fn schema() -> campaign_core::Id {
    derive(campaign_core::IdKind::Projection, b"repair-loop-schema")
}
pub(crate) fn adapter() -> campaign_core::Id {
    derive(campaign_core::IdKind::Adapter, b"repair-loop-adapter")
}
pub(crate) fn domain() -> campaign_core::Id {
    derive(campaign_core::IdKind::ControlDomain, b"repair-loop-control")
}
