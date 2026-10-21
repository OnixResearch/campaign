const GOLDEN_CODES: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
];
const GOLDEN_KINDS: &[super::IdKind] = &[
    super::IdKind::Absent,
    super::IdKind::Campaign,
    super::IdKind::Projection,
    super::IdKind::Graph,
    super::IdKind::ControlBranch,
    super::IdKind::Head,
    super::IdKind::Policy,
    super::IdKind::GuidanceProfile,
    super::IdKind::EntropyProfile,
    super::IdKind::BudgetProfile,
    super::IdKind::Adapter,
    super::IdKind::ControlDomain,
    super::IdKind::RootMoment,
    super::IdKind::OperationProfile,
    super::IdKind::OperationSchema,
    super::IdKind::CostProfile,
    super::IdKind::Candidate,
    super::IdKind::GuidanceRecord,
    super::IdKind::State,
    super::IdKind::Selection,
    super::IdKind::Operation,
    super::IdKind::Event,
    super::IdKind::Result,
    super::IdKind::ChildReservation,
    super::IdKind::ChildMoment,
    super::IdKind::Finding,
    super::IdKind::Reason,
    super::IdKind::EntropyTicket,
    super::IdKind::Receipt,
    super::IdKind::Cache,
    super::IdKind::SchedulingProfile,
    super::IdKind::AdmissionPolicy,
    super::IdKind::CoalescingPolicy,
    super::IdKind::SourceFacts,
    super::IdKind::EventPayload,
    super::IdKind::PrunePlan,
    super::IdKind::Stop,
    super::IdKind::Discovery,
    super::IdKind::ResourceDimension,
    super::IdKind::SelectionProfile,
    super::IdKind::Request,
    super::IdKind::DerivedRecord,
    super::IdKind::Relaxation,
    super::IdKind::Invalidation,
    super::IdKind::RetentionProfile,
    super::IdKind::AdmissionOutcomes,
    super::IdKind::RoleProjection,
    super::IdKind::CostReservation,
];
const LOW_DIGEST: [u8; super::DIGEST_LENGTH] = [0; super::DIGEST_LENGTH];
const HIGH_DIGEST: [u8; super::DIGEST_LENGTH] = [u8::MAX; super::DIGEST_LENGTH];

#[test]
fn preserves_every_wire_code() {
    let codes: alloc::vec::Vec<u8> = GOLDEN_KINDS
        .iter()
        .copied()
        .map(super::IdKind::code)
        .collect();
    assert_eq!(codes, GOLDEN_CODES);
}

#[test]
fn preserves_tag_then_digest_order() {
    let campaign = super::Id {
        kind: super::IdKind::Campaign,
        digest: HIGH_DIGEST,
    };
    let candidate_low = super::Id {
        kind: super::IdKind::Candidate,
        digest: LOW_DIGEST,
    };
    let candidate_high = super::Id {
        kind: super::IdKind::Candidate,
        digest: HIGH_DIGEST,
    };
    let mut values = [candidate_high, campaign, candidate_low];
    values.sort();
    assert_eq!(values, [campaign, candidate_low, candidate_high]);
    assert_eq!(
        campaign.partial_cmp(&candidate_low),
        Some(core::cmp::Ordering::Less)
    );
}

#[test]
fn accepts_the_required_tag() {
    let value = super::Id {
        kind: super::IdKind::Campaign,
        digest: LOW_DIGEST,
    };
    assert_eq!(value.require_kind(super::IdKind::Campaign), Ok(value));
}

#[test]
fn rejects_missing_and_crossed_tags() {
    let absent = super::Id::absent();
    let candidate = super::Id {
        kind: super::IdKind::Candidate,
        digest: LOW_DIGEST,
    };
    assert_eq!(
        absent.require_kind(super::IdKind::Campaign),
        Err(crate::error::CampaignError::MissingIdentity)
    );
    assert_eq!(
        candidate.require_kind(super::IdKind::Campaign),
        Err(crate::error::CampaignError::CrossedIdentity)
    );
}
