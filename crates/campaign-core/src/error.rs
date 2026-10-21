#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub enum CampaignError {
    /// Public Campaign core API variant.
    MissingIdentity,
    /// Public Campaign core API variant.
    MalformedIdentity,
    /// Public Campaign core API variant.
    CrossedIdentity,
    /// Public Campaign core API variant.
    UnsupportedVersion,
    /// Public Campaign core API variant.
    EmptyRoots,
    /// Public Campaign core API variant.
    DuplicateRoot,
    /// Public Campaign core API variant.
    DuplicateIdentity,
    /// Public Campaign core API variant.
    DuplicateControlEvent,
    /// Public Campaign core API variant.
    MissingAdmission,
    /// Public Campaign core API variant.
    MissingCandidate,
    /// Public Campaign core API variant.
    MissingGuidance,
    /// Public Campaign core API variant.
    MissingSelection,
    /// Public Campaign core API variant.
    MissingParentMoment,
    /// Public Campaign core API variant.
    StaleState,
    /// Public Campaign core API variant.
    StaleGuidance,
    /// Public Campaign core API variant.
    StaleProjection,
    /// Public Campaign core API variant.
    AmbiguousLineage,
    /// Public Campaign core API variant.
    InvalidControlEventOrder,
    /// Public Campaign core API variant.
    InvalidConfig,
    /// Public Campaign core API variant.
    InvalidCostProfile,
    /// Public Campaign core API variant.
    InvalidGuidanceShape,
    /// Public Campaign core API variant.
    InvalidEntropy,
    /// Public Campaign core API variant.
    InvalidPublication,
    /// Public Campaign core API variant.
    InvalidChildReservation,
    /// Public Campaign core API variant.
    InvalidChildMoment,
    /// Public Campaign core API variant.
    InvalidFinding,
    /// Public Campaign core API variant.
    InvalidPruning,
    /// Public Campaign core API variant.
    InvalidRetirement,
    /// Public Campaign core API variant.
    InvalidStop,
    /// Public Campaign core API variant.
    InvalidCache,
    /// Public Campaign core API variant.
    InvalidReceiptClaim,
    /// Public Campaign core API variant.
    InvalidRetryBoundary,
    /// Public Campaign core API variant.
    ReferencePolicySelected,
    /// Public Campaign core API variant.
    RoleCollapse,
    /// Public Campaign core API variant.
    ConflatedEvidenceQuestion,
    /// Public Campaign core API variant.
    NonImprovingRelaxation,
    /// Public Campaign core API variant.
    ConflictingResult,
    /// Public Campaign core API variant.
    DuplicateResult,
    /// Public Campaign core API variant.
    DuplicateChildMoment,
    /// Public Campaign core API variant.
    CandidateRetired,
    /// Public Campaign core API variant.
    CandidatePruned,
    /// Public Campaign core API variant.
    CandidateOverDepth,
    /// Public Campaign core API variant.
    OutstandingLimitReached,
    /// Public Campaign core API variant.
    BudgetExhausted,
    /// Public Campaign core API variant.
    ControlEventLimitReached,
    /// Public Campaign core API variant.
    BoundExceeded,
    /// Public Campaign core API variant.
    ArithmeticOverflow,
    /// Public Campaign core API variant.
    CampaignStopped,
    /// Public Campaign core API variant.
    UnchangedState,
    /// A candidate names an operation schema outside the admitted campaign schema set.
    UnsupportedSchema,
    /// A candidate depth is not exactly one more than its parent moment depth.
    InvalidDepth,
    /// An unobserved closure did not acknowledge its uncertain outcome.
    MissingUncertaintyAcknowledgement,
    /// Two plans with equal declared scheduling rules and inputs selected different work.
    UndeclaredSchedulingDifference,
}

/// Public Campaign core API value.
pub type CampaignResult<T> = Result<T, CampaignError>;
