#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(dylint_lib = "octet", feature(register_tool))]
#![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]

//! Reusable Campaign conformance port.
//!
//! A consumer implements [`ConformanceAdapter`]: the mapping from the suite's abstract scenario
//! vocabulary (moments, operations, ordered scores, draws, outcomes) to its own Campaign records,
//! plus the consumer shell points that must delegate to Campaign (history loading, cache
//! validation, the execution gate, receipt claims, and role identities). [`run_suite`] drives
//! every named positive and negative fixture through `campaign-core` and `campaign-choregraph`
//! against real Choregraph history and returns one named verdict per fixture together with an
//! adapter-independent [`DecisionTranscript`] whose digest lets two consumers compare decisions.
//!
//! The crate models no consumer execution, VM, replay, minimization, or evidence authority. The
//! readable reference planner is reachable only through the explicit [`reference`] path.

extern crate alloc;

/// Conforming fixture adapters: the Animus-shaped non-VM adapter and a float-scoring adapter.
pub mod fixtures;
/// Independent readable reference planner used only as differential evidence.
pub mod reference;

mod catalog;
mod harness;
mod inputs;
mod ledger;
mod port;
#[cfg(test)]
mod tests;
mod verdicts;
mod vocabulary;

pub use catalog::{fixture_names, run_suite};
pub use inputs::SuiteProfile;
pub use ledger::{DecisionTranscript, TranscriptEntry};
pub use port::{
    AdapterIdentities, AuthoritySignals, CandidateContext, ClosureContext, ConformanceAdapter,
    ConsumerShell, EntropyContext, ExecutionAuthority, ExecutionGateRequest, GuidanceContext,
    MomentIdentity, MomentMapping, RecordMapping, ResultContext, RevisionContext,
};
pub use verdicts::{
    ConformanceError, ConformanceReport, ConformanceResult, FixtureOutcome, FixturePolarity,
    FixtureVerdict,
};
pub use vocabulary::{
    AbstractCost, AbstractScore, MomentHandle, OperationHandle, SCENARIO_RANK_DIMENSIONS,
    ScenarioDraw, ScenarioMoment, ScenarioOperation, ScenarioOutcome, ScenarioPreference,
    ScenarioReason,
};
