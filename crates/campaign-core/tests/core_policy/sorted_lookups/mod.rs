//! D71: settlement, intake, and replay find accepted findings, children, and payloads by binary
//! search over the canonical state. Every accepted entry must still be found, wherever it sorts,
//! when a later result re-reports it.

const ACCEPTED: u64 = 64;
const REPORTED: u64 = 128;
const FINDING_BOUND: u64 = 256;

fn finding(index: u64, facts: &'static [u8]) -> Result<campaign_core::FindingRef, String> {
    Ok(campaign_core::FindingRef {
        finding_id: campaign_core::identity::derive_id_with(
            campaign_core::IdKind::Finding,
            b"lookup-finding",
            |writer| Ok(writer.u64(index)),
        )
        .map_err(crate::suite::s3_fixture::err)?,
        priority_facts_id: crate::suite::admission::xid(campaign_core::IdKind::SourceFacts, facts),
    })
}

/// Findings `indices`, submitted in descending order (not the canonical order).
fn findings(
    indices: core::ops::Range<u64>,
    facts: &'static [u8],
) -> Result<Vec<campaign_core::FindingRef>, String> {
    indices.rev().map(|index| finding(index, facts)).collect()
}

/// A state whose first selection settled with `ACCEPTED` findings and whose second selection is
/// outstanding.
fn settled_and_outstanding() -> Result<
    (
        crate::suite::s3_fixture::History,
        campaign_core::FrontierState,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_findings = FINDING_BOUND;
    })?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"lookup-a")?;
    let (events, state, second) = crate::suite::s3_fixture::selected(events, &state, b"lookup-b")?;
    let mut observation = crate::suite::s3_fixture::observation(&state, &first, Vec::new());
    observation.findings = findings(0..ACCEPTED, b"lookup-facts")?;
    let result =
        campaign_core::admit_result(&state, observation).map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, result)?;
    assert_eq!(
        u64::try_from(state.findings.len()).map_err(crate::suite::s3_fixture::err)?,
        ACCEPTED
    );
    Ok((events, state, second))
}

/// A result re-reporting every accepted finding (plus new ones) records each accepted finding
/// once; re-reporting any single accepted finding with other source facts, at the first, middle,
/// or last canonical position, is `ConflictingResult` at admission and at replay.
#[test]
fn every_accepted_finding_is_found_when_re_reported() -> Result<(), String> {
    let (events, state, second) = settled_and_outstanding()?;
    let mut overlapping = crate::suite::s3_fixture::observation(&state, &second, Vec::new());
    overlapping.findings = findings(0..REPORTED, b"lookup-facts")?;
    let result =
        campaign_core::admit_result(&state, overlapping).map_err(crate::suite::s3_fixture::err)?;
    let (_events, after) = crate::suite::s3_fixture::append(events.clone(), result)?;
    assert_eq!(
        u64::try_from(after.findings.len()).map_err(crate::suite::s3_fixture::err)?,
        REPORTED
    );
    let mut canonical = findings(0..ACCEPTED, b"lookup-facts")?;
    canonical.sort_by_key(|finding| finding.finding_id);
    let mut new_only = crate::suite::s3_fixture::observation(&state, &second, Vec::new());
    new_only.findings = findings(ACCEPTED..REPORTED, b"lookup-facts")?;
    for position in [0, canonical.len() / 2, canonical.len().saturating_sub(1)] {
        let changed = campaign_core::FindingRef {
            finding_id: canonical[position].finding_id,
            priority_facts_id: crate::suite::admission::xid(
                campaign_core::IdKind::SourceFacts,
                b"lookup-other-facts",
            ),
        };
        let mut conflicting = new_only.clone();
        conflicting.findings.push(changed);
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::admit_result(
                &state,
                conflicting.clone()
            ))?,
            campaign_core::CampaignError::ConflictingResult
        );
        assert_eq!(
            crate::suite::s3_fixture::replay_denial(
                &events,
                crate::suite::s3_fixture::forged_result(&state, conflicting)?,
            )?,
            campaign_core::CampaignError::ConflictingResult
        );
    }
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}
