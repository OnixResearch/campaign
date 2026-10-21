//! Conforming fixture adapters used by Campaign's own conformance tests and by consumers that
//! compare their transcript digest against a Campaign-side pilot.

/// Animus-shaped non-VM candidate adapter.
pub mod animus_shaped;
/// Minimal float-scoring adapter with a declared fixed-point conversion profile.
pub mod float_scores;

mod shell;
