//! Named positive and negative fixtures and the suite runner.

mod budgets;
mod common;
mod discipline;
mod evidence;
mod fencing;
mod frontier;
mod guidance;
mod intake;
mod outcomes;
pub(crate) mod ranking;
mod retention;

/// One named fixture of the suite.
pub(crate) struct Fixture {
    /// Stable fixture name.
    pub(crate) name: &'static str,
    /// Declared polarity.
    pub(crate) polarity: crate::verdicts::FixturePolarity,
    /// Fixture body; `Ok` when the fixture behaved as declared.
    pub(crate) body: fn(&mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()>,
}

/// Fixture groups in suite order.
// r[impl campaign.conformance]
// r[impl campaign.fd.verification]
const GROUPS: [&[Fixture]; 10] = [
    intake::FIXTURES,
    guidance::FIXTURES,
    ranking::FIXTURES,
    fencing::FIXTURES,
    budgets::FIXTURES,
    outcomes::FIXTURES,
    frontier::FIXTURES,
    retention::FIXTURES,
    evidence::FIXTURES,
    discipline::FIXTURES,
];

/// Names of every fixture in suite order.
#[must_use]
pub fn fixture_names() -> alloc::vec::Vec<&'static str> {
    GROUPS
        .iter()
        .flat_map(|group| group.iter().map(|fixture| fixture.name))
        .collect()
}

/// Runs every named fixture against one adapter under one caller-supplied profile.
///
/// Every fixture opens its own campaign over fresh Choregraph history, so one failing fixture
/// never hides another. The report keeps one named verdict per fixture and the decision
/// transcript of the whole run.
// r[impl campaign.conformance]
// r[impl campaign.fd.verification]
pub fn run_suite(
    adapter: &mut dyn crate::port::ConformanceAdapter,
    profile: &crate::inputs::SuiteProfile,
) -> crate::verdicts::ConformanceReport {
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let checked = profile.validate();
    let fixture_count = GROUPS.iter().map(|group| group.len()).sum();
    let mut outcomes = alloc::vec::Vec::with_capacity(fixture_count);
    for fixture in GROUPS.iter().flat_map(|group| group.iter()) {
        transcript.record(crate::ledger::TranscriptEntry::labelled(
            crate::ledger::kinds::FIXTURE,
            fixture.name,
        ));
        let run = checked.clone().and_then(|()| {
            let mut harness = crate::harness::Driver {
                adapter: &mut *adapter,
                profile,
                transcript: &mut transcript,
            };
            (fixture.body)(&mut harness)
        });
        let verdict = crate::verdicts::FixtureVerdict::classify(fixture.polarity, run);
        transcript.record(crate::ledger::TranscriptEntry::labelled(
            crate::ledger::kinds::VERDICT,
            verdict.class(),
        ));
        outcomes.push(crate::verdicts::FixtureOutcome {
            name: fixture.name,
            polarity: fixture.polarity,
            verdict,
        });
    }
    debug_assert_eq!(outcomes.len(), fixture_count);
    debug_assert!(transcript.entries.len() >= fixture_count);
    crate::verdicts::ConformanceReport {
        outcomes,
        transcript,
    }
}

/// Verdict of the one named fixture run against one adapter under one profile.
#[cfg(test)]
pub(crate) fn run_fixture(
    adapter: &mut dyn crate::port::ConformanceAdapter,
    profile: &crate::inputs::SuiteProfile,
    name: &str,
) -> Option<crate::verdicts::FixtureVerdict> {
    let fixture = GROUPS
        .iter()
        .flat_map(|group| group.iter())
        .find(|fixture| fixture.name == name)?;
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter,
        profile,
        transcript: &mut transcript,
    };
    let run = (fixture.body)(&mut harness);
    Some(crate::verdicts::FixtureVerdict::classify(
        fixture.polarity,
        run,
    ))
}
