//! `campaign.non_vm_candidate_fixture.authority`: a high rank, a selected candidate, a zero exit
//! status, or a validator reference never yields execution, promotion, deletion, or acceptance
//! authority through the Animus-shaped adapter.

const ALL_SIGNALS: crate::port::AuthoritySignals = crate::port::AuthoritySignals {
    is_top_ranked: true,
    is_selected: true,
    has_zero_exit_status: true,
    has_check_reference: true,
};

/// Forbidden effect of every gate question the authority tests ask.
const UNPUBLISHED: &str = "a selection executed before durable publication";

fn standard_operations() -> [crate::vocabulary::ScenarioOperation; 2] {
    [
        crate::vocabulary::ScenarioOperation {
            handle: crate::vocabulary::OperationHandle(1),
            parent: crate::harness::history::ROOT,
            costs: alloc::vec![crate::vocabulary::AbstractCost {
                dimension: 0,
                units: 1,
            }],
        },
        crate::vocabulary::ScenarioOperation {
            handle: crate::vocabulary::OperationHandle(2),
            parent: crate::harness::history::ROOT,
            costs: alloc::vec![crate::vocabulary::AbstractCost {
                dimension: 0,
                units: 1,
            }],
        },
    ]
}

fn guided(session: &mut crate::harness::Session<'_, '_>) -> crate::verdicts::ConformanceResult<()> {
    let operations = standard_operations();
    session.admit(&operations)?;
    for (operation, level) in operations.iter().zip([90_u32, 10]) {
        session.guide(&crate::vocabulary::ScenarioPreference {
            operation: operation.handle,
            generation: 1,
            score: crate::vocabulary::AbstractScore {
                levels: alloc::vec![level, level],
            },
        })?;
    }
    Ok(())
}

#[test]
// r[verify campaign.non_vm_candidate_fixture.authority]
// r[verify campaign.publication_fence.unpublished]
fn a_selected_top_ranked_candidate_is_not_executable_before_durable_publication()
-> crate::verdicts::ConformanceResult<()> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, 8, |_| {})?;
    guided(&mut session)?;
    let selection = session.select(None)?;
    assert_eq!(
        session.operation_of(selection.candidate_id)?,
        crate::vocabulary::OperationHandle(1)
    );
    assert_eq!(
        session.gate(&selection, None, None, UNPUBLISHED)?,
        crate::port::ExecutionAuthority::Refused
    );
    let publication = session.publish_selection(&selection)?;
    let partial = crate::harness::decisions::durable_evidence(
        &publication,
        crate::harness::decisions::Durability {
            is_event_durable: true,
            is_branch_update_durable: false,
        },
    );
    assert_eq!(
        session.gate(&selection, Some(&publication), Some(&partial), UNPUBLISHED)?,
        crate::port::ExecutionAuthority::Refused
    );
    let durable = crate::harness::decisions::durable_evidence(
        &publication,
        crate::harness::decisions::Durability::COMMITTED,
    );
    assert_eq!(
        session.gate(&selection, Some(&publication), Some(&durable), UNPUBLISHED)?,
        crate::port::ExecutionAuthority::Granted
    );
    Ok(())
}

#[test]
// r[verify campaign.non_vm_candidate_fixture.authority]
// r[verify campaign.receipts.non_claims]
fn a_zero_exit_status_and_validator_reference_grant_no_promotion_deletion_or_acceptance()
-> crate::verdicts::ConformanceResult<()> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, 8, |_| {})?;
    guided(&mut session)?;
    let selection = session.execute(None)?;
    let result = session.settle(
        &selection,
        &crate::vocabulary::ScenarioOutcome {
            children: alloc::vec::Vec::new(),
            findings: 0,
        },
    )?;
    let validator = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::validator_reference(
        selection.selection_id,
    )?;
    assert_eq!(result.derived_records, alloc::vec![validator]);
    let accepted = session.state.candidate(selection.candidate_id)?;
    assert_eq!(accepted.status, campaign_core::CandidateStatus::Retained);
    let planned = session.plan(None)?;
    let receipt = campaign_core::project_receipt(
        &session.state,
        &planned.plan,
        &planned.request,
        &planned.binding,
    )?;
    let claims = session
        .harness
        .adapter
        .promotion_claims(&receipt, &ALL_SIGNALS);
    assert_eq!(claims, campaign_core::ReceiptClaims::no_claims());
    campaign_core::validate_receipt(&receipt, &claims)?;
    for withheld in [
        campaign_core::NonClaim::OperationSuccess,
        campaign_core::NonClaim::Authorization,
        campaign_core::NonClaim::ReleaseReadiness,
        campaign_core::NonClaim::SafeDeletion,
    ] {
        assert!(receipt.non_claims.contains(&withheld));
    }
    Ok(())
}
