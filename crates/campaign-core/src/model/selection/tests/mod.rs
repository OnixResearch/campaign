const FIXTURE_LABEL: &[u8] = b"selection-profile-fixture";
const THRESHOLD: u64 = 250_000;
const OTHER_THRESHOLD: u64 = 500_000;
const OVER_SCALE: u64 = 1_000_001;
const PENALTY: u64 = 3;
const OTHER_PENALTY: u64 = 4;
const FINDING_LIMIT: u64 = 2;
const FIRST_DIMENSION: u32 = 0;
const SECOND_DIMENSION: u32 = 1;

fn id(kind: crate::identity::IdKind, label: &[u8]) -> crate::identity::Id {
    crate::identity::derive_id(
        kind,
        FIXTURE_LABEL,
        &[crate::identity::derive_id(kind, label, &[])],
    )
}

fn exploratory() -> super::SelectionMode {
    super::SelectionMode::Exploratory {
        threshold_parts_per_million: THRESHOLD,
        entropy_source_id: id(crate::identity::IdKind::EntropyProfile, b"entropy"),
        exploration_order: alloc::vec![SECOND_DIMENSION, FIRST_DIMENSION],
    }
}

fn fixture() -> super::SelectionProfile {
    super::SelectionProfile {
        profile_id: crate::identity::Id::absent(),
        scheduling_policy_id: id(crate::identity::IdKind::SchedulingProfile, b"schedule"),
        admission_policy_id: id(crate::identity::IdKind::AdmissionPolicy, b"admit"),
        coalescing_policy_id: id(crate::identity::IdKind::CoalescingPolicy, b"coalesce"),
        mode: exploratory(),
        decay: super::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: PENALTY,
        },
        stopping: super::StoppingRule {
            finding_limit: Some(FINDING_LIMIT),
        },
        implementation: crate::discipline::PolicyImplementation::ProductionCompiled {
            policy_id: id(crate::identity::IdKind::Policy, b"policy"),
        },
    }
}

type ProfileChange = fn(&mut super::SelectionProfile);

const PROFILE_CHANGES: [ProfileChange; 12] = [
    |profile| profile.mode = super::SelectionMode::Ranked,
    |profile| {
        if let super::SelectionMode::Exploratory {
            threshold_parts_per_million,
            ..
        } = &mut profile.mode
        {
            *threshold_parts_per_million = OTHER_THRESHOLD;
        }
    },
    |profile| {
        if let super::SelectionMode::Exploratory {
            entropy_source_id, ..
        } = &mut profile.mode
        {
            *entropy_source_id = id(crate::identity::IdKind::EntropyProfile, b"other-entropy");
        }
    },
    |profile| {
        if let super::SelectionMode::Exploratory {
            exploration_order, ..
        } = &mut profile.mode
        {
            exploration_order.reverse();
        }
    },
    |profile| profile.decay = super::SelectionCountDecay::None,
    |profile| {
        profile.decay = super::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: OTHER_PENALTY,
        };
    },
    |profile| profile.stopping.finding_limit = None,
    |profile| {
        profile.implementation = crate::discipline::PolicyImplementation::ReferenceEvidence {
            reference_id: id(crate::identity::IdKind::Policy, b"policy"),
        };
    },
    |profile| {
        profile.implementation = crate::discipline::PolicyImplementation::ProductionCompiled {
            policy_id: id(crate::identity::IdKind::Policy, b"other-policy"),
        };
    },
    |profile| {
        profile.scheduling_policy_id = id(
            crate::identity::IdKind::SchedulingProfile,
            b"other-schedule",
        );
    },
    |profile| {
        profile.admission_policy_id = id(crate::identity::IdKind::AdmissionPolicy, b"other-admit");
    },
    |profile| {
        profile.coalescing_policy_id =
            id(crate::identity::IdKind::CoalescingPolicy, b"other-coalesce");
    },
];

#[test]
fn derived_profile_validates_and_is_deterministic() -> crate::error::CampaignResult<()> {
    let first = fixture().derive()?;
    let second = fixture().derive()?;
    first.validate()?;
    assert_eq!(first, second);
    assert_eq!(
        first.profile_id.kind,
        crate::identity::IdKind::SelectionProfile
    );
    Ok(())
}

#[test]
// r[verify campaign.entropy]
fn every_declared_fact_is_bound_into_the_profile_identity() -> crate::error::CampaignResult<()> {
    let base = fixture().derive()?;
    let mut seen = alloc::vec![base.profile_id];
    for change in PROFILE_CHANGES {
        let mut changed = fixture();
        change(&mut changed);
        let changed = changed.derive()?;
        assert!(!seen.contains(&changed.profile_id));
        seen.push(changed.profile_id);
    }
    Ok(())
}

#[test]
fn a_fact_changed_after_derivation_is_a_malformed_identity() -> crate::error::CampaignResult<()> {
    for change in PROFILE_CHANGES {
        let mut tampered = fixture().derive()?;
        change(&mut tampered);
        assert_eq!(
            tampered.validate(),
            Err(crate::error::CampaignError::MalformedIdentity)
        );
    }
    let mut absent = fixture().derive()?;
    absent.profile_id = crate::identity::Id::absent();
    assert_eq!(
        absent.validate(),
        Err(crate::error::CampaignError::MissingIdentity)
    );
    Ok(())
}

#[test]
fn zero_decay_penalty_is_invalid() {
    let mut profile = fixture();
    profile.decay = super::SelectionCountDecay::SubtractFromFirstRank {
        penalty_per_selection: 0,
    };
    assert_eq!(
        profile.derive(),
        Err(crate::error::CampaignError::InvalidConfig)
    );
}

#[test]
fn zero_finding_limit_is_invalid() {
    let mut profile = fixture();
    profile.stopping.finding_limit = Some(0);
    assert_eq!(
        profile.derive(),
        Err(crate::error::CampaignError::InvalidConfig)
    );
}

#[test]
fn exploration_order_must_be_nonempty_and_unique() {
    let mut empty = fixture();
    if let super::SelectionMode::Exploratory {
        exploration_order, ..
    } = &mut empty.mode
    {
        exploration_order.clear();
    }
    assert_eq!(
        empty.derive(),
        Err(crate::error::CampaignError::InvalidConfig)
    );
    let mut duplicate = fixture();
    if let super::SelectionMode::Exploratory {
        exploration_order, ..
    } = &mut duplicate.mode
    {
        *exploration_order = alloc::vec![FIRST_DIMENSION, FIRST_DIMENSION];
    }
    assert_eq!(
        duplicate.derive(),
        Err(crate::error::CampaignError::InvalidConfig)
    );
}

#[test]
fn threshold_above_scale_and_crossed_entropy_source_are_rejected() {
    let mut over = fixture();
    if let super::SelectionMode::Exploratory {
        threshold_parts_per_million,
        ..
    } = &mut over.mode
    {
        *threshold_parts_per_million = OVER_SCALE;
    }
    assert_eq!(
        over.derive(),
        Err(crate::error::CampaignError::InvalidConfig)
    );
    let mut crossed = fixture();
    if let super::SelectionMode::Exploratory {
        entropy_source_id, ..
    } = &mut crossed.mode
    {
        *entropy_source_id = id(crate::identity::IdKind::Policy, b"not-entropy");
    }
    assert_eq!(
        crossed.derive(),
        Err(crate::error::CampaignError::CrossedIdentity)
    );
}
