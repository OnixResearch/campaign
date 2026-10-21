//! Typed conformance errors, per-fixture verdicts, and the suite report.

/// Conformance result type.
pub type ConformanceResult<T> = Result<T, ConformanceError>;

/// Errors observed while a fixture runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceError {
    /// The Campaign core denied a value.
    Campaign(campaign_core::CampaignError),
    /// The Choregraph adapter denied a value.
    Adapter(campaign_choregraph::AdapterError),
    /// Choregraph history denied a value.
    History(campaign_choregraph::HistoryError),
    /// A fixture observed a decision other than the declared one.
    Diverged {
        /// Declared expectation that did not hold.
        expected: &'static str,
    },
    /// A forbidden input or effect was denied with an undeclared typed error.
    UndeclaredDenial {
        /// Forbidden input or effect the fixture presented.
        forbidden: &'static str,
        /// The undeclared denial.
        denial: alloc::boxed::Box<ConformanceError>,
    },
    /// A negative fixture's forbidden input or effect was accepted.
    ForbiddenAccepted {
        /// Forbidden input or effect that went through.
        forbidden: &'static str,
    },
    /// The suite profile cannot express the declared scenarios.
    InvalidProfile {
        /// Violated profile requirement.
        requirement: &'static str,
    },
    /// The adapter declared an unusable identity set.
    InvalidIdentities,
}

impl ConformanceError {
    /// The typed Campaign error, whether Campaign or the Choregraph adapter reported it.
    #[must_use]
    pub fn campaign(&self) -> Option<campaign_core::CampaignError> {
        if let Self::Campaign(error)
        | Self::Adapter(campaign_choregraph::AdapterError::Campaign(error)) = self
        {
            return Some(error.clone());
        }
        None
    }
}

impl From<campaign_core::CampaignError> for ConformanceError {
    fn from(error: campaign_core::CampaignError) -> Self {
        Self::Campaign(error)
    }
}

impl From<campaign_choregraph::AdapterError> for ConformanceError {
    fn from(error: campaign_choregraph::AdapterError) -> Self {
        Self::Adapter(error)
    }
}

impl From<campaign_choregraph::HistoryError> for ConformanceError {
    fn from(error: campaign_choregraph::HistoryError) -> Self {
        Self::History(error)
    }
}

/// Declared polarity of one fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixturePolarity {
    /// An accepted flow must be accepted and decide as declared.
    Positive,
    /// A forbidden input or effect must be denied as declared.
    Negative,
}

/// Named result of one fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixtureVerdict {
    /// The fixture behaved as declared.
    Passed,
    /// A positive fixture's accepted flow was denied or decided otherwise.
    PositiveFixtureFailed(ConformanceError),
    /// A negative fixture's forbidden input or effect was accepted.
    NegativeFixturePassed {
        /// Forbidden input or effect that went through.
        forbidden: &'static str,
    },
    /// A negative fixture could not set up its forbidden input or denied it with an
    /// undeclared error.
    NegativeFixtureFailed(ConformanceError),
}

impl FixtureVerdict {
    /// Classifies a fixture run by its polarity.
    #[must_use]
    pub fn classify(polarity: FixturePolarity, run: ConformanceResult<()>) -> Self {
        let Err(error) = run else {
            return Self::Passed;
        };
        match (polarity, error) {
            (FixturePolarity::Negative, ConformanceError::ForbiddenAccepted { forbidden }) => {
                Self::NegativeFixturePassed { forbidden }
            }
            (FixturePolarity::Negative, denied) => Self::NegativeFixtureFailed(denied),
            (FixturePolarity::Positive, failed) => Self::PositiveFixtureFailed(failed),
        }
    }

    /// Adapter-independent verdict class recorded in the transcript.
    #[must_use]
    pub const fn class(&self) -> &'static str {
        if matches!(self, Self::Passed) {
            "passed"
        } else if matches!(self, Self::PositiveFixtureFailed(_)) {
            "positive-fixture-failed"
        } else if matches!(self, Self::NegativeFixturePassed { .. }) {
            "negative-fixture-passed"
        } else {
            "negative-fixture-failed"
        }
    }

    /// True when the fixture behaved as declared.
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}

/// Named verdict of one fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureOutcome {
    /// Stable fixture name.
    pub name: &'static str,
    /// Declared fixture polarity.
    pub polarity: FixturePolarity,
    /// Verdict of this run.
    pub verdict: FixtureVerdict,
}

/// Complete conformance run: one named verdict per fixture and the decision transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceReport {
    /// Named fixture outcomes in suite order.
    pub outcomes: alloc::vec::Vec<FixtureOutcome>,
    /// Adapter-independent decision transcript of the run.
    pub transcript: crate::ledger::DecisionTranscript,
}

impl ConformanceReport {
    /// Returns the outcome of the named fixture.
    #[must_use]
    pub fn outcome(&self, name: &str) -> Option<&FixtureOutcome> {
        self.outcomes.iter().find(|outcome| outcome.name == name)
    }

    /// Outcomes whose verdict is not `Passed`.
    #[must_use]
    pub fn failures(&self) -> alloc::vec::Vec<&FixtureOutcome> {
        self.outcomes
            .iter()
            .filter(|outcome| !outcome.verdict.is_passed())
            .collect()
    }
}
