const NATIVE_LENGTH: usize = 3;
const PROTOCOL_LIMIT: u64 = 3;
const SMALLER_LIMIT: u64 = 2;

#[test]
fn accepts_zero_and_exact_limit() {
    assert_eq!(super::check_len(0, 0), Ok(()));
    assert_eq!(super::check_len(NATIVE_LENGTH, PROTOCOL_LIMIT), Ok(()));
}

#[test]
fn rejects_length_above_limit() {
    assert_eq!(
        super::check_len(NATIVE_LENGTH, SMALLER_LIMIT),
        Err(crate::error::CampaignError::BoundExceeded)
    );
    assert_eq!(
        super::check_len(1, 0),
        Err(crate::error::CampaignError::BoundExceeded)
    );
}

#[test]
fn accepts_a_nonempty_exact_limit() {
    assert_eq!(
        super::check_len_at_least_one(
            NATIVE_LENGTH,
            PROTOCOL_LIMIT,
            crate::error::CampaignError::EmptyRoots
        ),
        Ok(())
    );
}

#[test]
fn preserves_the_supplied_empty_error() {
    assert_eq!(
        super::check_len_at_least_one(0, PROTOCOL_LIMIT, crate::error::CampaignError::EmptyRoots),
        Err(crate::error::CampaignError::EmptyRoots)
    );
    assert_eq!(
        super::check_len_at_least_one(0, 0, crate::error::CampaignError::InvalidGuidanceShape),
        Err(crate::error::CampaignError::InvalidGuidanceShape)
    );
}

#[test]
fn rejects_a_nonempty_over_limit_value() {
    assert_eq!(
        super::check_len_at_least_one(
            NATIVE_LENGTH,
            SMALLER_LIMIT,
            crate::error::CampaignError::EmptyRoots
        ),
        Err(crate::error::CampaignError::BoundExceeded)
    );
}

const FIXTURE_BOUND: u64 = 2;
const CHANGED_BOUND: u64 = 3;
const ZERO_BOUND: u64 = 0;

type BoundField = fn(&mut super::ExplorationBounds) -> &mut u64;

const BOUND_FIELDS: [BoundField; super::BOUND_COUNT] = [
    |bounds| &mut bounds.max_roots,
    |bounds| &mut bounds.max_candidates,
    |bounds| &mut bounds.max_candidate_batch,
    |bounds| &mut bounds.max_retained_candidates,
    |bounds| &mut bounds.max_selections,
    |bounds| &mut bounds.max_guidance_records,
    |bounds| &mut bounds.max_outstanding,
    |bounds| &mut bounds.max_depth,
    |bounds| &mut bounds.max_child_moments,
    |bounds| &mut bounds.max_derived_records,
    |bounds| &mut bounds.max_rank_dimensions,
    |bounds| &mut bounds.max_rank_magnitude,
    |bounds| &mut bounds.max_findings,
    |bounds| &mut bounds.max_control_events,
    |bounds| &mut bounds.max_resource_dimensions,
    |bounds| &mut bounds.max_operation_schemas,
    |bounds| &mut bounds.max_prune_batch,
];

fn fixture() -> super::ExplorationBounds {
    super::ExplorationBounds {
        bounds_id: crate::identity::Id::absent(),
        max_roots: FIXTURE_BOUND,
        max_candidates: FIXTURE_BOUND,
        max_candidate_batch: FIXTURE_BOUND,
        max_retained_candidates: FIXTURE_BOUND,
        max_selections: FIXTURE_BOUND,
        max_guidance_records: FIXTURE_BOUND,
        max_outstanding: FIXTURE_BOUND,
        max_depth: FIXTURE_BOUND,
        max_child_moments: FIXTURE_BOUND,
        max_derived_records: FIXTURE_BOUND,
        max_rank_dimensions: FIXTURE_BOUND,
        max_rank_magnitude: FIXTURE_BOUND,
        max_findings: FIXTURE_BOUND,
        max_control_events: FIXTURE_BOUND,
        max_resource_dimensions: FIXTURE_BOUND,
        max_operation_schemas: FIXTURE_BOUND,
        max_prune_batch: FIXTURE_BOUND,
    }
}

#[test]
fn every_bound_is_framed_into_the_identity() -> crate::error::CampaignResult<()> {
    let base = fixture().derive()?;
    base.validate()?;
    let mut seen = alloc::vec![base.bounds_id];
    for field in BOUND_FIELDS {
        let mut changed = fixture();
        *field(&mut changed) = CHANGED_BOUND;
        let changed = changed.derive()?;
        assert!(!seen.contains(&changed.bounds_id));
        seen.push(changed.bounds_id);
    }
    assert_eq!(Some(seen.len()), super::BOUND_COUNT.checked_add(1));
    Ok(())
}

#[test]
fn rejects_a_zero_value_in_every_bound() {
    for field in BOUND_FIELDS {
        let mut zero = fixture();
        *field(&mut zero) = ZERO_BOUND;
        assert_eq!(
            zero.derive(),
            Err(crate::error::CampaignError::InvalidConfig)
        );
    }
}

#[test]
fn rejects_a_value_changed_after_derivation() -> crate::error::CampaignResult<()> {
    for field in BOUND_FIELDS {
        let mut tampered = fixture().derive()?;
        *field(&mut tampered) = CHANGED_BOUND;
        assert_eq!(
            tampered.validate(),
            Err(crate::error::CampaignError::InvalidConfig)
        );
    }
    Ok(())
}

#[test]
fn the_rank_magnitude_is_bounded_by_i64_max() -> crate::error::CampaignResult<()> {
    let mut at_limit = fixture();
    at_limit.max_rank_magnitude = super::MAX_RANK_MAGNITUDE;
    let at_limit = at_limit.derive()?;
    at_limit.validate()?;
    assert_eq!(
        i64::try_from(at_limit.max_rank_magnitude),
        Ok(i64::MAX),
        "the limit is exactly i64::MAX"
    );
    // `|i64::MIN|` is one past the limit: an admitted `i64::MIN` rank would have no negation.
    for over in [i64::MIN.unsigned_abs(), u64::MAX] {
        let mut over_limit = fixture();
        over_limit.max_rank_magnitude = over;
        assert_eq!(
            over_limit.derive(),
            Err(crate::error::CampaignError::InvalidConfig)
        );
    }
    Ok(())
}
