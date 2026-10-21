//! `campaign.configuration.invalid`: well-shaped JSON with values the Nickel contracts reject fails with a
//! typed `ProfileError`.

pub(super) const BOUND_KEYS: [&str; 17] = [
    "maximum_roots",
    "maximum_candidates",
    "maximum_candidate_batch",
    "maximum_retained_candidates",
    "maximum_selections",
    "maximum_guidance_records",
    "maximum_outstanding_selections",
    "maximum_depth",
    "maximum_children_per_result",
    "maximum_derived_records",
    "maximum_rank_dimensions",
    "maximum_rank_magnitude",
    "maximum_finding_records",
    "maximum_control_events",
    "maximum_budget_dimensions",
    "maximum_operation_schemas",
    "maximum_prune_batch",
];

pub(super) const OWNER_GROUPS: [&str; 8] = [
    "bounds",
    "selection",
    "stopping",
    "retention",
    "admission",
    "implementation",
    "versions",
    "receipt_non_claims",
];

pub(super) fn loaded(document: &serde_json::Value) -> Result<crate::CampaignProfile, String> {
    super::load(document).map_err(|error| error.to_string())
}

#[test]
fn versions_schema_and_implementation_must_be_supported() -> super::TestResult {
    assert_eq!(
        super::rejected(&super::with(
            "/schema",
            serde_json::json!("campaign.profile.v1")
        )?)?,
        crate::ProfileError::UnsupportedProfileSchema
    );
    assert_eq!(
        super::rejected(&super::with(
            "/schema",
            serde_json::json!("campaign.profile.v2 ")
        )?)?,
        crate::ProfileError::UnsupportedProfileSchema
    );
    let next_campaign_schema = u64::from(campaign_core::CAMPAIGN_SCHEMA_VERSION)
        .checked_add(1)
        .ok_or("campaign schema version overflow")?;
    assert_eq!(
        super::rejected(&super::with(
            "/versions/campaign_schema",
            serde_json::json!(next_campaign_schema),
        )?)?,
        crate::ProfileError::UnsupportedCampaignSchema
    );
    assert_eq!(
        super::rejected(&super::with(
            "/versions/identity_version",
            serde_json::json!(2)
        )?)?,
        crate::ProfileError::UnsupportedIdentityVersion
    );
    assert_eq!(
        super::rejected(&super::with(
            "/implementation",
            serde_json::json!("reference_evidence"),
        )?)?,
        crate::ProfileError::ReferenceImplementation
    );
    Ok(())
}

#[test]
fn owners_must_be_non_empty() -> super::TestResult {
    for group in OWNER_GROUPS {
        assert_eq!(
            super::rejected(&super::with(
                &format!("/owners/{group}"),
                serde_json::json!("")
            )?)?,
            crate::ProfileError::EmptyOwner { group }
        );
    }
    let short = loaded(&super::with("/owners/bounds", serde_json::json!("x"))?)?;
    assert_eq!(short.owners.bounds, "x");
    Ok(())
}

#[test]
// r[verify campaign.configuration.invalid]
fn every_bound_must_be_at_least_one() -> super::TestResult {
    for bound in BOUND_KEYS {
        assert_eq!(
            super::rejected(&super::with(
                &format!("/bounds/{bound}"),
                serde_json::json!(0)
            )?)?,
            crate::ProfileError::ZeroBound { bound }
        );
    }
    let mut minimal = super::default_document()?;
    let bounds = minimal
        .pointer_mut("/bounds")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("default bounds are absent")?;
    for value in bounds.values_mut() {
        *value = serde_json::json!(1);
    }
    bounds.insert(
        String::from("maximum_control_events"),
        serde_json::json!(campaign_core::MIN_CONTROL_EVENTS),
    );
    let profile = loaded(&minimal)?;
    assert_eq!(profile.bounds.max_roots, 1);
    assert_eq!(profile.bounds.max_prune_batch, 1);
    assert_eq!(
        profile.bounds.max_control_events,
        campaign_core::MIN_CONTROL_EVENTS
    );
    Ok(())
}

#[test]
// r[verify campaign.configuration.invalid]
fn contained_bounds_must_not_exceed_their_limit() -> super::TestResult {
    const LIMIT: u64 = 65_536;
    const OVER_LIMIT: u64 = 65_537;
    let default_bounds_id = crate::default_profile()
        .map_err(|error| error.to_string())?
        .bounds
        .bounds_id;
    for (bound, limit) in [
        ("maximum_candidate_batch", "maximum_candidates"),
        ("maximum_retained_candidates", "maximum_candidates"),
        ("maximum_outstanding_selections", "maximum_selections"),
        ("maximum_prune_batch", "maximum_candidates"),
    ] {
        let pointer = format!("/bounds/{bound}");
        assert_eq!(
            super::rejected(&super::with(&pointer, serde_json::json!(OVER_LIMIT))?)?,
            crate::ProfileError::BoundExceedsLimit { bound, limit }
        );
        let at_limit = loaded(&super::with(&pointer, serde_json::json!(LIMIT))?)?;
        assert_ne!(
            at_limit.bounds.bounds_id, default_bounds_id,
            "{bound} at its limit"
        );
    }
    Ok(())
}

#[test]
fn exploratory_threshold_must_be_within_scale_and_match_the_mode() -> super::TestResult {
    let over_scale = campaign_core::ENTROPY_SCALE
        .checked_add(1)
        .ok_or("entropy scale overflow")?;
    assert_eq!(
        super::rejected(&super::with(
            "/selection/exploratory_threshold_parts_per_million",
            serde_json::json!(over_scale),
        )?)?,
        crate::ProfileError::ThresholdOverScale
    );
    for path in [
        "/selection/exploratory_threshold_parts_per_million",
        "/selection/exploration_order",
    ] {
        assert_eq!(
            super::rejected(&super::with_key(path, None)?)?,
            crate::ProfileError::ExploratoryFieldsMismatch
        );
    }
    assert_eq!(
        super::rejected(&super::with(
            "/selection/mode",
            serde_json::json!("ranked")
        )?)?,
        crate::ProfileError::ExploratoryFieldsMismatch
    );
    Ok(())
}

#[test]
fn rank_orders_must_be_non_empty_unique_and_in_range() -> super::TestResult {
    for (pointer, order) in [
        ("/selection/exploration_order", "exploration_order"),
        ("/retention/rank_order", "rank_order"),
    ] {
        assert_eq!(
            super::rejected(&super::with(pointer, serde_json::json!([]))?)?,
            crate::ProfileError::EmptyOrder { order }
        );
        assert_eq!(
            super::rejected(&super::with(pointer, serde_json::json!([3, 1, 3]))?)?,
            crate::ProfileError::DuplicateOrderIndex { order, index: 3 }
        );
        assert_eq!(
            super::rejected(&super::with(pointer, serde_json::json!([0, 16]))?)?,
            crate::ProfileError::OrderIndexOutOfRange { order, index: 16 }
        );
        loaded(&super::with(pointer, serde_json::json!([15, 0]))?)?;
    }
    Ok(())
}

#[test]
fn decay_penalty_and_finding_limit_must_be_positive_and_declared() -> super::TestResult {
    assert_eq!(
        super::rejected(&super::with(
            "/selection/decay/rule",
            serde_json::json!("subtract_from_first_rank"),
        )?)?,
        crate::ProfileError::DecayPenaltyMismatch
    );
    assert_eq!(
        super::rejected(&super::with_key(
            "/selection/decay/penalty_per_selection",
            Some(serde_json::json!(1)),
        )?)?,
        crate::ProfileError::DecayPenaltyMismatch
    );
    assert_eq!(
        super::rejected(&super::with(
            "/selection/decay",
            serde_json::json!({ "rule": "subtract_from_first_rank", "penalty_per_selection": 0 }),
        )?)?,
        crate::ProfileError::ZeroDecayPenalty
    );
    assert_eq!(
        super::rejected(&super::with_key(
            "/stopping/finding_limit",
            Some(serde_json::json!(0)),
        )?)?,
        crate::ProfileError::ZeroFindingLimit
    );
    Ok(())
}
