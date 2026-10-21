//! Framing and rejection cases for the application-envelope boundary.

const FIXTURE_DIGEST_BYTE: u8 = 37;
const ALTERED_DIGEST_BYTE: u8 = 41;
const RETIREMENT_KIND_CODE: u32 = 7;
const PAYLOAD_LABEL: &[u8] = b"campaign-choregraph-control-payload";

const fn fixture_id(kind: campaign_core::IdKind) -> campaign_core::Id {
    campaign_core::Id {
        kind,
        digest: [FIXTURE_DIGEST_BYTE; campaign_core::identity::DIGEST_LENGTH],
    }
}

const fn retirement() -> campaign_core::RetirementEvent {
    campaign_core::RetirementEvent {
        event_id: fixture_id(campaign_core::IdKind::Event),
        campaign_id: fixture_id(campaign_core::IdKind::Campaign),
        state_before_id: fixture_id(campaign_core::IdKind::State),
        candidate_id: fixture_id(campaign_core::IdKind::Candidate),
        reason_id: fixture_id(campaign_core::IdKind::Reason),
        policy_id: fixture_id(campaign_core::IdKind::Policy),
        adapter_id: fixture_id(campaign_core::IdKind::Adapter),
        origin: campaign_core::RetirementOrigin::Consumer,
        invalidation_id: fixture_id(campaign_core::IdKind::Invalidation),
    }
}

#[test]
fn retirement_payload_retains_the_published_transcript_order() -> crate::AdapterResult<()> {
    let record = retirement();
    let expected = campaign_core::identity::derive_id_with(
        campaign_core::IdKind::EventPayload,
        PAYLOAD_LABEL,
        |writer| {
            Ok(writer
                .u32(RETIREMENT_KIND_CODE)
                .id(record.event_id)
                .option_id(Some(record.state_before_id))
                .id(record.campaign_id)
                .id(record.candidate_id)
                .id(record.reason_id)
                .id(record.policy_id)
                .id(record.adapter_id)
                .u32(record.origin.code())
                .id(record.invalidation_id))
        },
    )?;
    let actual = super::derive_control_payload_id(
        &campaign_core::CampaignControlEvent::CandidateRetired(record),
    )?;
    assert_eq!(actual, expected);
    assert_eq!(actual.kind, campaign_core::IdKind::EventPayload);
    Ok(())
}

#[test]
fn a_changed_reason_cannot_reuse_the_payload_identity() -> crate::AdapterResult<()> {
    let original = campaign_core::CampaignControlEvent::CandidateRetired(retirement());
    let mut changed_record = retirement();
    changed_record.reason_id.digest = [ALTERED_DIGEST_BYTE; campaign_core::identity::DIGEST_LENGTH];
    let changed = campaign_core::CampaignControlEvent::CandidateRetired(changed_record);
    assert_ne!(
        super::derive_control_payload_id(&original)?,
        super::derive_control_payload_id(&changed)?
    );
    Ok(())
}

#[test]
fn a_root_draft_has_exact_metadata_and_no_parent() -> crate::AdapterResult<()> {
    let event = campaign_core::CampaignControlEvent::CandidateRetired(retirement());
    let draft = super::control_event_draft(
        &event,
        fixture_id(campaign_core::IdKind::Projection),
        fixture_id(campaign_core::IdKind::Adapter),
        fixture_id(campaign_core::IdKind::ControlDomain),
        None,
    )?;
    assert_eq!(draft.metadata.len(), super::EXACT_METADATA_COUNT);
    assert!(draft.parents.is_empty());
    assert!(draft.metadata.is_sorted());
    Ok(())
}

#[test]
fn a_wrong_schema_kind_returns_the_existing_typed_error() {
    let event = campaign_core::CampaignControlEvent::CandidateRetired(retirement());
    let result = super::control_event_draft(
        &event,
        fixture_id(campaign_core::IdKind::Adapter),
        fixture_id(campaign_core::IdKind::Adapter),
        fixture_id(campaign_core::IdKind::ControlDomain),
        None,
    );
    assert_eq!(
        result,
        Err(crate::AdapterError::Campaign(
            campaign_core::CampaignError::CrossedIdentity
        ))
    );
}

#[test]
fn a_linear_parent_is_accepted_but_another_event_is_rejected() -> crate::AdapterResult<()> {
    let expected = choregraph_history::EventId::new(crate::digest_from_id(fixture_id(
        campaign_core::IdKind::Event,
    )));
    let other = choregraph_history::EventId::new(choregraph_history::Digest::from_bytes(
        [ALTERED_DIGEST_BYTE; campaign_core::identity::DIGEST_LENGTH],
    ));
    let links = [choregraph_history::ParentLink {
        role: alloc::string::String::from(super::CONTROL_PARENT_ROLE),
        event: expected,
    }];
    super::validate_linear_parent(&links, expected)?;
    assert_eq!(
        super::validate_linear_parent(&links, other),
        Err(crate::AdapterError::ParentLineageMismatch)
    );
    assert_eq!(
        super::validate_root_parent(&links),
        Err(crate::AdapterError::ParentLineageMismatch)
    );
    Ok(())
}
