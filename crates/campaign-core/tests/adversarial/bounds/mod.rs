#[test]
fn result_entrypoint_must_enforce_the_finding_bound_before_event_publication() -> Result<(), String>
{
    let (_events, state, selection) =
        crate::suite::selection::selected_state(crate::suite::constants::RESOURCE_BUDGET_TOTAL)?;
    let one_finding = vec![campaign_core::FindingRef {
        finding_id: crate::suite::admission::tid(campaign_core::IdKind::Finding, b"single-finding"),
        priority_facts_id: crate::suite::admission::tid(
            campaign_core::IdKind::SourceFacts,
            b"single-priority",
        ),
    }];
    let valid = campaign_core::admit_result(
        &state,
        crate::suite::observation::result(&state, &selection, one_finding),
    )
    .map_err(|error| format!("{error:?}"))?;
    assert!(matches!(
        valid,
        campaign_core::CampaignControlEvent::ResultObserved(_)
    ));
    let two_findings = vec![
        campaign_core::FindingRef {
            finding_id: crate::suite::admission::tid(
                campaign_core::IdKind::Finding,
                b"first-over-bound-finding",
            ),
            priority_facts_id: crate::suite::admission::tid(
                campaign_core::IdKind::SourceFacts,
                b"first-over-bound-priority",
            ),
        },
        campaign_core::FindingRef {
            finding_id: crate::suite::admission::tid(
                campaign_core::IdKind::Finding,
                b"second-over-bound-finding",
            ),
            priority_facts_id: crate::suite::admission::tid(
                campaign_core::IdKind::SourceFacts,
                b"second-over-bound-priority",
            ),
        },
    ];
    let denied = campaign_core::admit_result(
        &state,
        crate::suite::observation::result(&state, &selection, two_findings),
    );
    assert_eq!(
        denied.map(|_| ()),
        Err(campaign_core::CampaignError::BoundExceeded),
        "r[campaign.findings.valid]: result admission must enforce max_findings before publication"
    );
    Ok(())
}
