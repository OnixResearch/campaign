//! Eleventh closeout-audit pairing loophole adapters kept as negative controls: each shell that
//! once passed every fixture because the suite never paired two facts (an evidence branch name
//! with its heads, or both generations; a gate question without evidence with a loaded
//! publication; a sampled role identity with a reported child and finding; a revision with a
//! preceding coalesced update; a ticket's counter with the next selection ordinal) now fails
//! exactly the named fixtures that present the pairing, with their declared verdicts, under the
//! default and the reordered profile.

mod wrapper;

type Failures = alloc::vec::Vec<(&'static str, crate::verdicts::FixtureVerdict)>;

fn failures_under(holes: &[wrapper::Hole], profile: &crate::inputs::SuiteProfile) -> Failures {
    let mut adapter = wrapper::EleventhAuditAdapter::new(holes);
    super::failures(&crate::catalog::run_suite(&mut adapter, profile))
}

/// Failures under the default profile, which must equal those under the reordered profile
/// (`exploration_order = [1]`, `retention_rank_order = [1]`).
fn failures(holes: &[wrapper::Hole]) -> crate::verdicts::ConformanceResult<Failures> {
    let profile = super::default_profile()?;
    let mut reordered = profile.clone();
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    reordered.validate()?;
    let default = failures_under(holes, &profile);
    assert_eq!(failures_under(holes, &reordered), default, "reordered");
    Ok(default)
}

/// Negative fixtures that let through the named forbidden effect.
fn passed(names: &[&'static str], forbidden: &'static str) -> Failures {
    names
        .iter()
        .map(|name| {
            (
                *name,
                crate::verdicts::FixtureVerdict::NegativeFixturePassed { forbidden },
            )
        })
        .collect()
}

const GATE_MATRICES: [&str; 2] = [
    "fencing.gate_matrix_ranked",
    "fencing.gate_matrix_exploratory",
];

#[test]
// r[verify campaign.conformance.positive]
fn the_healthy_wrapper_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(failures(&[])?, Failures::new());
    Ok(())
}

/// A gate that checks one branch name before and after and one generation step, but never
/// compares them with the publication's plan, grants the published transition moved onto a
/// mirror branch; comparing names with the plan but generations only with each other grants it
/// at shifted generations. Both gate matrices refuse both moves, as Campaign does with
/// `PublicationEvidenceMismatch`.
#[test]
// r[verify campaign.publication_fence.unpublished]
// r[verify campaign.conformance.negative]
fn self_consistent_foreign_transitions_fail_the_gate_matrices()
-> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::RelativeCasGate])?,
        passed(
            &GATE_MATRICES,
            "evidence of the published transition on another branch executed the published \
             selection"
        )
    );
    assert_eq!(
        failures(&[wrapper::Hole::RelativeGenerationGate])?,
        passed(
            &GATE_MATRICES,
            "evidence of the published transition at shifted generations executed the published \
             selection"
        )
    );
    Ok(())
}

/// A recovering gate that grants a selection presented without a publication or evidence once
/// its last loaded projection carries the publication fails both gate matrices, which ask those
/// questions again after the session loads that history.
#[test]
// r[verify campaign.publication_fence.unpublished]
// r[verify campaign.conformance.negative]
fn a_loaded_recovery_gate_fails_the_gate_matrices() -> crate::verdicts::ConformanceResult<()> {
    assert_eq!(
        failures(&[wrapper::Hole::LoadedRecoveryGate])?,
        passed(
            &GATE_MATRICES,
            "a loaded published selection executed without a publication or evidence"
        )
    );
    Ok(())
}

/// A consumer that files a result reporting a child moment or a finding under its candidate
/// fails the role fixture, which samples such a result.
#[test]
// r[verify campaign.fd.roles.collapsed]
fn a_reported_result_role_collapse_fails_the_role_fixture() -> crate::verdicts::ConformanceResult<()>
{
    assert_eq!(
        failures(&[wrapper::Hole::ReportedResultCollapse])?,
        passed(
            &["roles.collapsed_identity"],
            "the adapter stored candidate, work item, and result as one record"
        )
    );
    Ok(())
}

/// A revision naming the adapter's own last mapped record instead of the context's invalidated
/// record is refused as stale once an equal-rank update coalesced into the kept record: the
/// coalesced duplicate fixture fails, and so does the state coverage fixture, whose recipes
/// revise guidance after an equal-rank update.
#[test]
// r[verify campaign.guidance.revision]
// r[verify campaign.guidance.coalesced]
fn a_revision_from_own_memory_fails_the_coalesced_duplicate_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let stale =
        || crate::verdicts::ConformanceError::Campaign(campaign_core::CampaignError::StaleGuidance);
    assert_eq!(
        failures(&[wrapper::Hole::RevisionOwnMemory])?,
        crate::tests::with_state_coverage(
            [(
                "guidance.coalesced_duplicate",
                crate::verdicts::FixtureVerdict::PositiveFixtureFailed(stale()),
            )],
            crate::verdicts::FixtureVerdict::NegativeFixtureFailed(stale()),
        )
    );
    Ok(())
}

/// Tickets counted by an adapter-owned sequence instead of the next selection ordinal fail
/// every ticket-issuing fixture at the harness's ticket decoding.
#[test]
// r[verify campaign.entropy.valid]
// r[verify campaign.non_vm_candidate_fixture.reproducible]
fn an_adapter_owned_ticket_counter_fails_every_ticket_fixture()
-> crate::verdicts::ConformanceResult<()> {
    let expected: Failures = super::TICKET_FIXTURES
        .iter()
        .map(|(name, polarity)| {
            let diverged = crate::verdicts::ConformanceError::Diverged {
                expected: crate::harness::decisions::entropy::NEXT_ORDINAL,
            };
            let verdict = match polarity {
                crate::verdicts::FixturePolarity::Positive => {
                    crate::verdicts::FixtureVerdict::PositiveFixtureFailed(diverged)
                }
                crate::verdicts::FixturePolarity::Negative => {
                    crate::verdicts::FixtureVerdict::NegativeFixtureFailed(diverged)
                }
            };
            (*name, verdict)
        })
        .collect();
    assert_eq!(expected.len(), super::TICKET_FIXTURES.len());
    assert_eq!(failures(&[wrapper::Hole::TicketOwnCounter])?, expected);
    Ok(())
}
