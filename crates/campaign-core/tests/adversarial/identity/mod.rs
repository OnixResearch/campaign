#[test]
fn rejects_forged_closure() -> Result<(), String> {
    let (_events, state, selection) =
        crate::suite::selection::selected_state(crate::suite::constants::RESOURCE_BUDGET_TOTAL)?;
    let reason_id =
        crate::suite::admission::tid(campaign_core::IdKind::Reason, b"unobserved-reason");
    let mut valid_closure = campaign_core::UnobservedClosure {
        closure_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        reason_id,
        uncertainty_acknowledged: true,
    };
    valid_closure.closure_id =
        (campaign_core::derive_closure_id(&valid_closure)).map_err(|error| format!("{error:?}"))?;
    assert!(
        campaign_core::close_unobserved(&state, valid_closure).is_ok(),
        "valid prior settlement must remain admissible"
    );
    let forged_closure = campaign_core::UnobservedClosure {
        closure_id: crate::suite::admission::tid(
            campaign_core::IdKind::Event,
            b"forged-unobserved-closure",
        ),
        campaign_id: state.admission.campaign_id,
        selection_id: selection.selection_id,
        reason_id,
        uncertainty_acknowledged: true,
    };
    assert!(
        campaign_core::close_unobserved(&state, forged_closure).is_err(),
        "r[campaign.unobserved.valid]: closure admission must bind the exact closure identity"
    );
    Ok(())
}
