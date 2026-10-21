//! `campaign.configuration.valid`: the embedded default and other contract-valid exports load exactly.

const DEFAULT_THRESHOLD: u64 = 100_000;
const RANKED_PENALTY: u64 = 3;
const RANKED_FINDING_LIMIT: u64 = 5;

#[test]
// r[verify campaign.configuration.valid]
fn default_profile_loads_the_documented_values() -> super::TestResult {
    let profile = crate::default_profile().map_err(|error| error.to_string())?;
    let expected_bounds = campaign_core::ExplorationBounds {
        bounds_id: campaign_core::Id::absent(),
        max_roots: 256,
        max_candidates: 65_536,
        max_candidate_batch: 256,
        max_retained_candidates: 4096,
        max_selections: 65_536,
        max_guidance_records: 65_536,
        max_outstanding: 256,
        max_depth: 4096,
        max_child_moments: 256,
        max_derived_records: 256,
        max_rank_dimensions: 16,
        max_rank_magnitude: 4_294_967_295,
        max_findings: 4096,
        max_control_events: 1_048_576,
        max_resource_dimensions: 32,
        max_operation_schemas: 64,
        max_prune_batch: 256,
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(profile.bounds, expected_bounds);
    assert_eq!(profile.bounds.validate(), Ok(()));
    assert_eq!(
        profile.selection_mode,
        crate::DeclaredSelectionMode::Exploratory {
            threshold_parts_per_million: DEFAULT_THRESHOLD,
            exploration_order: vec![0],
        }
    );
    assert_eq!(profile.decay, campaign_core::SelectionCountDecay::None);
    assert_eq!(
        profile.stopping,
        campaign_core::StoppingRule {
            finding_limit: None
        }
    );
    assert_eq!(profile.retention_rank_order, vec![0]);
    assert_eq!(
        profile.admission,
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        }
    );
    assert_eq!(profile.receipt_non_claims, campaign_core::NonClaim::ALL);
    let core = String::from("campaign-core");
    assert_eq!(
        profile.owners,
        crate::ProfileOwners {
            bounds: core.clone(),
            selection: core.clone(),
            stopping: String::from("consumer-shell"),
            retention: String::from("consumer-adapter"),
            admission: core.clone(),
            implementation: core.clone(),
            versions: core.clone(),
            receipt_non_claims: core,
        }
    );
    Ok(())
}

#[test]
// r[verify campaign.configuration.valid]
fn contract_valid_variants_load_their_declared_values() -> super::TestResult {
    // Mirrors `config/fixtures/positive/ranked-profile.ncl`: ranked mode, decay penalty, finding limit, coalescing.
    let mut ranked = super::default_document()?;
    let selection = ranked
        .pointer_mut("/selection")
        .ok_or("default selection is absent")?;
    *selection = serde_json::json!({
        "mode": "ranked",
        "decay": { "rule": "subtract_from_first_rank", "penalty_per_selection": RANKED_PENALTY },
        "tie_break": "candidate_identity",
    });
    *ranked
        .pointer_mut("/stopping")
        .ok_or("default stopping is absent")? =
        serde_json::json!({ "finding_limit": RANKED_FINDING_LIMIT });
    *ranked
        .pointer_mut("/admission/duplicate_candidate")
        .ok_or("default admission is absent")? = serde_json::json!("coalesce");
    *ranked
        .pointer_mut("/retention/rank_order")
        .ok_or("default rank order is absent")? = serde_json::json!([15, 0, 7]);
    let profile = super::load(&ranked).map_err(|error| error.to_string())?;
    assert_eq!(profile.selection_mode, crate::DeclaredSelectionMode::Ranked);
    assert_eq!(
        profile.decay,
        campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: RANKED_PENALTY
        }
    );
    assert_eq!(profile.stopping.finding_limit, Some(RANKED_FINDING_LIMIT));
    assert_eq!(
        profile.admission.duplicate_candidate,
        campaign_core::DuplicateCandidateRule::Coalesce
    );
    assert_eq!(profile.retention_rank_order, vec![15, 0, 7]);

    // Threshold scale boundaries and a permuted non-claim list stay valid.
    for threshold in [0, campaign_core::ENTROPY_SCALE] {
        let document = super::with(
            "/selection/exploratory_threshold_parts_per_million",
            serde_json::json!(threshold),
        )?;
        let profile = super::load(&document).map_err(|error| error.to_string())?;
        assert_eq!(
            profile.selection_mode,
            crate::DeclaredSelectionMode::Exploratory {
                threshold_parts_per_million: threshold,
                exploration_order: vec![0],
            }
        );
    }
    let mut permuted = super::default_document()?;
    permuted
        .pointer_mut("/receipt_non_claims")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("default non-claims are absent")?
        .reverse();
    let profile = super::load(&permuted).map_err(|error| error.to_string())?;
    assert_eq!(profile.receipt_non_claims, campaign_core::NonClaim::ALL);
    Ok(())
}
