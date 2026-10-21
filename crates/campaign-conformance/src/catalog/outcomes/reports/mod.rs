//! Finding fixtures: reported findings enter the state exactly, at every presented count, and the
//! findings of two results in one campaign stay distinct.

/// Intermediate finding count: more than three, so a mapping that keeps the top three findings of
/// a result cannot conform.
const INTERMEDIATE_FINDINGS: u32 = 4;

/// Most findings the suite presents in one result: the default profile's
/// `maximum_finding_records`; a profile with a smaller `max_findings` caps it.
const FINDINGS_ROOM: u32 = 4096;

/// Finding counts presented after the result without findings: one finding, the intermediate
/// count, and the most one result may carry (`None`), each in its own campaign and at most the
/// most.
const PRESENTED: [Option<u32>; 3] = [Some(1), Some(INTERMEDIATE_FINDINGS), None];

/// Finding counts of the two finding-bearing results one campaign settles in turn, together at
/// most the most findings one campaign records (the first leaves room for the second).
const SAME_CAMPAIGN: [u32; 2] = [1, INTERMEDIATE_FINDINGS];

/// Settles one execution with `count` findings and requires the result to report exactly that
/// many findings under distinct identities that no earlier result of the campaign reported, each
/// recorded by the state.
///
/// # Errors
///
/// Returns the error of the scenario and `Diverged` for a missing, merged, or unrecorded finding.
fn settle_with_findings(
    session: &mut crate::harness::Session<'_, '_>,
    count: u32,
) -> crate::verdicts::ConformanceResult<()> {
    let recorded_before: alloc::vec::Vec<campaign_core::Id> = session
        .state
        .findings
        .iter()
        .map(|finding| finding.finding_id)
        .collect();
    let found = session.execute(None)?;
    let reported = crate::vocabulary::ScenarioOutcome {
        children: alloc::vec::Vec::new(),
        findings: count,
    };
    let accepted = session.settle(&found, &reported)?;
    let expected =
        usize::try_from(count).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let mut distinct: alloc::vec::Vec<campaign_core::Id> = accepted
        .findings
        .iter()
        .map(|finding| finding.finding_id)
        .collect();
    distinct.sort_unstable();
    distinct.dedup();
    super::super::common::require(
        accepted.findings.len() == expected
            && distinct.len() == expected
            && !distinct
                .iter()
                .any(|finding_id| recorded_before.contains(finding_id)),
        "the result reports every observed finding under its own identity",
    )?;
    let mut recorded: alloc::vec::Vec<campaign_core::Id> = session
        .state
        .findings
        .iter()
        .map(|finding| finding.finding_id)
        .collect();
    recorded.sort_unstable();
    let is_recorded = |finding_id: &campaign_core::Id| recorded.binary_search(finding_id).is_ok();
    super::super::common::require(
        session.state.findings.len() == recorded_before.len().saturating_add(expected)
            && distinct.iter().all(is_recorded),
        "every distinct reported finding enters the state",
    )?;
    debug_assert_eq!(accepted.findings.len(), expected);
    Ok(())
}

/// Settles one execution without findings, then, each in its own campaign, one execution with
/// one finding, with more than three findings, and with the most findings one result may carry,
/// and finally two executions of one campaign with one and with more than three findings: the
/// adapter must report exactly the observed findings under identities distinct within and across
/// results, and each distinct finding identity must enter `state.findings`.
// r[impl campaign.conformance.bounds]
// r[impl campaign.findings.valid]
pub(super) fn findings(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let quiet = session.execute(None)?;
    let accepted = session.settle(&quiet, &super::super::common::outcome(&[]))?;
    super::super::common::require(
        accepted.findings.is_empty() && session.state.findings.is_empty(),
        "a result without findings reports none",
    )?;
    debug_assert!(accepted.findings.is_empty());
    let most = u32::try_from(session.state.admission.bounds.max_findings)
        .map_or(FINDINGS_ROOM, |bound| bound.min(FINDINGS_ROOM));
    for requested in PRESENTED {
        let mut session = super::super::common::ample(harness)?;
        super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
        settle_with_findings(
            &mut session,
            requested.map_or(most, |count| count.min(most)),
        )?;
    }
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let [first, second] = SAME_CAMPAIGN;
    let first = first.min(most.saturating_sub(1));
    let second = second.min(most.saturating_sub(first));
    super::super::common::require(
        first > 0 && second > 0,
        "the campaign's finding bound admits two finding-bearing results",
    )?;
    settle_with_findings(&mut session, first)?;
    settle_with_findings(&mut session, second)
}
