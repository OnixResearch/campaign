//! Core entry outside src.
#![no_std]
extern crate std;
/// Wall clock.
#[must_use]
pub fn now() -> std::time::SystemTime {
    std::time::SystemTime::now()
}
