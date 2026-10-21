#![no_std]

// campaign-source: choregraph-history
use choregraph_history::linear_lineage;

pub enum AdapterError {
    ParentLineageMismatch,
}

pub fn adapter_admitted() -> bool {
    let _ = linear_lineage;
    true
}

pub fn observed_at() -> std::time::Instant {
    std::time::Instant::now()
}
