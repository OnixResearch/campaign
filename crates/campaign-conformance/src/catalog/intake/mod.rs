//! Identity-stability and admission fixtures.

/// Admission fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "identities.stable",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: stable_identities,
    },
    super::Fixture {
        name: "admission.batch",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: batch,
    },
    super::Fixture {
        name: "admission.duplicate_rejected",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: duplicate_rejected,
    },
    super::Fixture {
        name: "admission.duplicate_coalesced",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: duplicate_coalesced,
    },
    super::Fixture {
        name: "admission.hidden_admission",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: hidden_admission,
    },
];

/// Asks the adapter for its identities again after the session admitted a campaign under them:
/// every call must return the identities the campaign binds.
fn stable_identities(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let session = super::common::ample(harness)?;
    let again = session.harness.adapter.identities()?;
    let third = session.harness.adapter.identities()?;
    super::common::require(
        again == session.identities && third == session.identities,
        "the adapter's identities are equal on every call",
    )
}

fn root_operations() -> [crate::vocabulary::ScenarioOperation; 3] {
    [
        super::common::operation(
            super::common::OP_C,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
        super::common::operation(
            super::common::OP_A,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
        super::common::operation(
            super::common::OP_B,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
    ]
}

fn batch(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.admit(&root_operations())?;
    let is_retained = session
        .state
        .candidates
        .iter()
        .all(|candidate| candidate.status == campaign_core::CandidateStatus::Retained);
    super::common::require(
        session.state.candidates.len() == 3,
        "three candidates entered",
    )?;
    super::common::require(is_retained, "admitted candidates are retained")
}

fn duplicate_rejected(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Reject;
    session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    let before = session.state.state_id;
    let published = session.payloads.len();
    super::common::require(
        session.state.candidates.len() == 1 && published == session.state.event_ids.len(),
        "one candidate was admitted and every event published",
    )?;
    debug_assert_eq!(session.state.candidates.len(), 1);
    debug_assert_eq!(published, session.state.event_ids.len());
    let duplicate = session.batch(&[
        super::common::operation(
            super::common::OP_B,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
        super::common::operation(
            super::common::OP_A,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
    ]);
    session.harness.deny(
        duplicate,
        campaign_core::CampaignError::DuplicateIdentity,
        "a rejected duplicate candidate produced an admission event",
    )?;
    super::common::require(
        session.state.state_id == before,
        "the rejected batch left the state unchanged",
    )?;
    super::common::require(
        session.payloads.len() == published,
        "the rejected batch published nothing",
    )
}

fn duplicate_coalesced(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Coalesce;
    let plain = session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    let coalesced = session.admit(&[
        super::common::operation(
            super::common::OP_B,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
        super::common::operation(
            super::common::OP_A,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
    ])?;
    let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) = &coalesced else {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: "a coalescing batch is a candidate batch",
        });
    };
    let existing = session.candidate_id(super::common::OP_A)?;
    let is_recorded = batch.outcomes.contains(&campaign_core::AdmissionOutcome {
        candidate_id: existing,
        outcome: campaign_core::AdmissionDecision::CoalescedDuplicate {
            existing_candidate_id: existing,
        },
    });
    super::common::require(is_recorded, "the batch records the coalesced duplicate")?;
    debug_assert_eq!(
        batch.admission_rule.duplicate_candidate,
        campaign_core::DuplicateCandidateRule::Coalesce
    );
    super::common::require(
        batch.candidates.len() < batch.outcomes.len(),
        "the coalesced duplicate adds no candidate record",
    )?;
    debug_assert!(batch.candidates.len() < batch.outcomes.len());
    let coalesced_receipt = event_receipt(&session, &coalesced)?;
    let plain_receipt = event_receipt(&session, &plain)?;
    let recorded_outcomes = campaign_core::derive_admission_outcomes_id(&batch.outcomes)?;
    super::common::require(
        coalesced_receipt.admission_outcomes_id == Some(recorded_outcomes),
        "the receipt binds the coalescing outcome derived from the event",
    )?;
    super::common::require(
        coalesced_receipt.admission_outcomes_id != plain_receipt.admission_outcomes_id,
        "a coalescing receipt differs from a plain admission receipt",
    )
}

/// Event receipt of one accepted control event: under the profile a selection event carries, and
/// under the ranked suite profile for every other event.
///
/// # Errors
///
/// Returns the Campaign error of the projection.
pub(super) fn event_receipt(
    session: &crate::harness::Session<'_, '_>,
    event: &campaign_core::CampaignControlEvent,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignReceipt> {
    let profile = if let campaign_core::CampaignControlEvent::SelectionPublished(selection) = event
    {
        selection.profile.as_ref().clone()
    } else {
        session.selection_profile(false)?
    };
    let request = campaign_core::EventReceiptRequest {
        request_id: crate::harness::suite_id(campaign_core::IdKind::Request, "event-receipt")?,
        event: event.clone(),
        profile,
    };
    Ok(campaign_core::project_event_receipt(
        &session.state,
        &request,
        &session.binding()?,
    )?)
}

fn hidden_admission(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.rule.duplicate_candidate = campaign_core::DuplicateCandidateRule::Coalesce;
    session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    let records = session.candidates(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    let coalesced = session.batch(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    let campaign_core::CampaignControlEvent::CandidatesAdmitted(mut hidden) = coalesced else {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: "a coalescing batch is a candidate batch",
        });
    };
    for outcome in &mut hidden.outcomes {
        outcome.outcome = campaign_core::AdmissionDecision::Admitted;
    }
    hidden.candidates = records;
    hidden.admission_outcomes_id = campaign_core::derive_admission_outcomes_id(&hidden.outcomes)?;
    hidden.event_id = hidden.expected_event_id()?;
    debug_assert_eq!(hidden.candidates.len(), hidden.outcomes.len());
    debug_assert!(!hidden.outcomes.is_empty());
    let forged = campaign_core::CampaignControlEvent::CandidatesAdmitted(hidden);
    let published = session.publish(forged);
    session.harness.deny(
        published,
        campaign_core::CampaignError::MalformedIdentity,
        "a duplicate was admitted without the declared coalescing outcome",
    )
}
