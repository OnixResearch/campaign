//! D79: intake finds admitted operation schemas, and retention finds candidates with outstanding
//! work, by binary search; every entry must still be found wherever it sorts.

const SCHEMA_COUNT: usize = 5;
const SCHEMA_LABELS: [&[u8]; SCHEMA_COUNT] = [
    b"scan-schema-e",
    b"scan-schema-d",
    b"scan-schema-c",
    b"scan-schema-b",
    b"scan-schema-a",
];
/// Each of the first three candidates outranks every earlier one, so each publication selects it;
/// the last two rank below all three.
const RANKS: [i64; 5] = [10, 20, 30, 5, 1];
const CANDIDATE_LABELS: [&[u8]; 5] = [
    b"scan-candidate-a",
    b"scan-candidate-b",
    b"scan-candidate-c",
    b"scan-candidate-d",
    b"scan-candidate-e",
];
const OUTSTANDING_SELECTIONS: usize = 3;
const RETAINED: u64 = 1;
const PRUNE_BATCH: u64 = 16;
/// Room for five admissions, five guidance records, three selections, their settlements, and a
/// stop.
const CONTROL_EVENTS: u64 = 32;

fn schema(label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::OperationSchema, label)
}

fn admit_with_schema(
    state: &campaign_core::FrontierState,
    schema_id: campaign_core::Id,
) -> campaign_core::CampaignResult<campaign_core::CampaignControlEvent> {
    campaign_core::admit_candidates(
        state,
        vec![campaign_core::CandidateRecord {
            operation_schema_id: schema_id,
            ..crate::suite::candidate::record(
                state,
                b"scan-schema-candidate",
                crate::suite::constants::COST_ONE,
            )
        }],
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
    )
}

/// An admission declaring five schemas (listed in descending order) admits a candidate of each
/// one, and replays it, whatever its canonical position; an undeclared schema is
/// `UnsupportedSchema`.
#[test]
// r[verify campaign.candidate_admission.valid]
// r[verify campaign.candidate_admission.invalid]
fn every_admitted_operation_schema_is_found_wherever_it_sorts() -> Result<(), String> {
    let mut input = crate::suite::admission::default_input()?;
    input.operation_schema_ids = SCHEMA_LABELS.iter().map(|label| schema(label)).collect();
    let (events, state) = crate::suite::admission::admitted(input)?;
    let mut canonical: Vec<campaign_core::Id> =
        SCHEMA_LABELS.iter().map(|label| schema(label)).collect();
    canonical.sort();
    assert_eq!(state.admission.operation_schema_ids, canonical);
    for schema_id in &canonical {
        assert!(state.admission.supports_operation_schema(*schema_id));
        let event = admit_with_schema(&state, *schema_id).map_err(crate::suite::s3_fixture::err)?;
        let (_events, admitted) = crate::suite::s3_fixture::append(events.clone(), event)?;
        assert_eq!(admitted.candidates.len(), 1);
        assert_eq!(
            admitted.candidates[0].candidate.operation_schema_id,
            *schema_id
        );
    }
    let foreign = schema(b"scan-schema-foreign");
    assert!(!state.admission.supports_operation_schema(foreign));
    assert_eq!(
        admit_with_schema(&state, foreign).map(|_| ()),
        Err(campaign_core::CampaignError::UnsupportedSchema)
    );
    Ok(())
}

/// With three of five guided candidates outstanding over a retained bound of one, the retention
/// plan keeps every outstanding candidate and prunes exactly the other two; naming any outstanding
/// candidate in an explicit prune is `InvalidPruning`, naming another is admitted.
#[test]
// r[verify campaign.pruning.valid]
// r[verify campaign.pruning.invalid]
fn every_outstanding_candidate_is_kept_wherever_it_sorts() -> Result<(), String> {
    let (mut events, mut state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_retained_candidates = RETAINED;
        bounds.max_prune_batch = PRUNE_BATCH;
        bounds.max_control_events = CONTROL_EVENTS;
    })?;
    let mut candidates = Vec::with_capacity(CANDIDATE_LABELS.len());
    for (index, (label, rank)) in CANDIDATE_LABELS.iter().zip(RANKS).enumerate() {
        let candidate_id;
        (events, state, candidate_id) =
            crate::suite::s3_fixture::guided(events, &state, label, rank)?;
        candidates.push(candidate_id);
        if index < OUTSTANDING_SELECTIONS {
            let selection;
            (events, state, selection) = crate::suite::s3_fixture::published(events, &state)?;
            assert_eq!(selection.candidate_id, candidate_id);
        }
    }
    let mut outstanding: Vec<campaign_core::Id> = state
        .outstanding
        .iter()
        .map(|selection| selection.candidate_id)
        .collect();
    outstanding.sort();
    outstanding.dedup();
    assert_eq!(outstanding.len(), OUTSTANDING_SELECTIONS);
    let mut idle: Vec<campaign_core::Id> = candidates
        .iter()
        .copied()
        .filter(|candidate_id| outstanding.binary_search(candidate_id).is_err())
        .collect();
    idle.sort();
    let plan = campaign_core::plan_pruning(
        &state,
        crate::suite::s3_pruning::plan_request(
            &state,
            vec![crate::suite::s3_pruning::FIRST_DIMENSION],
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or_else(|| String::from("expected a prune plan"))?;
    assert_eq!(plan.candidates, idle);
    let event =
        campaign_core::prune_frontier(&state, plan).map_err(crate::suite::s3_fixture::err)?;
    crate::suite::s3_fixture::append(events.clone(), event)?;
    for candidate_id in &outstanding {
        assert!(state.has_outstanding_for_candidate(*candidate_id));
        assert_eq!(
            campaign_core::prune_frontier(
                &state,
                crate::suite::s3_pruning::explicit(&state, vec![*candidate_id]),
            )
            .map(|_| ()),
            Err(campaign_core::CampaignError::InvalidPruning)
        );
    }
    for candidate_id in &idle {
        assert!(!state.has_outstanding_for_candidate(*candidate_id));
        let event = campaign_core::prune_frontier(
            &state,
            crate::suite::s3_pruning::explicit(&state, vec![*candidate_id]),
        )
        .map_err(crate::suite::s3_fixture::err)?;
        crate::suite::s3_fixture::append(events.clone(), event)?;
    }
    Ok(())
}
