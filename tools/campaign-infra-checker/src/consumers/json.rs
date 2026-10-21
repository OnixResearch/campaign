//! Strict admission of the exported consumer pilot record: closed records, exact types, and the
//! structural facts `provenance/contracts.ncl` enforces, each reported under its specification rule.

type Admitted<T> = super::fields::Admitted<T>;
type Object = super::fields::Object;

const RECORD_FIELDS: &[&str] = &["schema", "reordered_profile", "consumers"];
const PROFILE_FIELDS: &[&str] = &["exploration_order", "retention_rank_order"];
const FIXTURE_FIELDS: &[&str] = &[
    "id",
    "kind",
    "adapter",
    "tests",
    "conformance",
    "retained_authority",
    "non_claims",
];
const MIGRATED_FIELDS: &[&str] = &[
    "id",
    "kind",
    "repository",
    "adoption",
    "pins",
    "conformance",
    "parity",
    "kvm_smoke",
    "retained_authority",
    "non_claims",
];
const CONFORMANCE_FIELDS: &[&str] = &[
    "fixtures",
    "transcript_digest",
    "reordered_profile_digest",
    "command",
];
const IN_REPO_FIXTURE: &str = "in_repo_fixture";
const MIGRATED_CONSUMER: &str = "migrated_consumer";

pub(crate) fn parse(text: &crate::model::NamedText) -> Admitted<super::Pilots> {
    let value: serde_json::Value = serde_json::from_str(&text.content).map_err(|error| {
        super::fields::violation(
            super::VALID,
            format!("{} is not valid JSON: {error}", text.relative_path),
        )
    })?;
    let record = super::fields::closed(&value, RECORD_FIELDS, super::VALID)?;
    if record.get("schema").and_then(serde_json::Value::as_str)
        != Some(crate::policy::CONSUMER_RECORD_SCHEMA)
    {
        return Err(super::fields::violation(
            super::VALID,
            format!(
                "consumer record schema must be {}",
                crate::policy::CONSUMER_RECORD_SCHEMA
            ),
        ));
    }
    let reordered_profile = reordered_profile(record)?;
    let entries = super::fields::list(record, "consumers", super::VALID)?;
    let pilots = entries.iter().map(pilot).collect::<Admitted<Vec<_>>>()?;
    require_unique_ids(&pilots)?;
    require_both_kinds(&pilots)?;
    assert_eq!(pilots.len(), entries.len());
    assert!(!pilots.is_empty());
    Ok(super::Pilots {
        reordered_profile,
        pilots,
    })
}

fn reordered_profile(record: &Object) -> Admitted<super::ReorderedProfile> {
    let profile =
        super::fields::section(record, "reordered_profile", PROFILE_FIELDS, super::VALID)?;
    assert_eq!(
        profile.len(),
        PROFILE_FIELDS.len(),
        "a closed profile section"
    );
    let reordered = super::ReorderedProfile {
        exploration_order: rank_order(profile, "exploration_order")?,
        retention_rank_order: rank_order(profile, "retention_rank_order")?,
    };
    let orders = [
        ("exploration_order", &reordered.exploration_order),
        ("retention_rank_order", &reordered.retention_rank_order),
    ];
    assert_eq!(orders.len(), PROFILE_FIELDS.len());
    if let Some((name, _)) = orders
        .iter()
        .find(|(_, order)| super::fields::has_repeat(order))
    {
        return Err(super::fields::violation(
            super::VALID,
            format!("reordered_profile.{name} must not repeat an entry"),
        ));
    }
    Ok(reordered)
}

fn rank_order(profile: &Object, name: &str) -> Admitted<Vec<u64>> {
    super::fields::list(profile, name, super::VALID)?
        .iter()
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                super::fields::violation(
                    super::VALID,
                    format!("reordered_profile.{name} must hold non-negative integers"),
                )
            })
        })
        .collect()
}

fn pilot(value: &serde_json::Value) -> Admitted<super::Pilot> {
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| super::fields::violation(super::VALID, "consumer pilot lacks a kind"))?;
    match kind {
        IN_REPO_FIXTURE => in_repo_fixture(value),
        MIGRATED_CONSUMER => migrated_consumer(value),
        other => Err(super::fields::violation(
            super::VALID,
            format!("unknown consumer pilot kind: {other}"),
        )),
    }
}

fn in_repo_fixture(value: &serde_json::Value) -> Admitted<super::Pilot> {
    let object = super::fields::closed(value, FIXTURE_FIELDS, super::VALID)?;
    let adapter = super::fields::relative_path(object, "adapter", super::VALID)?;
    let tests = super::fields::relative_paths(object, "tests", super::VALID)?;
    super::sections::retained_authority(object, &[], super::VALID)?;
    super::fields::texts(object, "non_claims", super::VALID)?;
    let campaign_paths: Vec<String> = core::iter::once(adapter)
        .chain(tests)
        .map(str::to_owned)
        .collect();
    assert!(
        campaign_paths.len() >= 2,
        "an adapter and at least one test"
    );
    assert_eq!(object.len(), FIXTURE_FIELDS.len());
    Ok(super::Pilot {
        id: pilot_id(object)?,
        conformance: conformance(object)?,
        subject: super::Subject::InRepoFixture { campaign_paths },
    })
}

fn migrated_consumer(value: &serde_json::Value) -> Admitted<super::Pilot> {
    let object = super::fields::closed(value, MIGRATED_FIELDS, super::VALID)?;
    let repository = super::fields::text(object, "repository", super::MIGRATION)?.to_owned();
    super::sections::adoption(object)?;
    let choregraph_revision = super::sections::pins(object)?;
    super::sections::parity(object)?;
    super::sections::kvm_smoke(object)?;
    super::sections::retained_authority(
        object,
        super::sections::MIGRATION_AUTHORITIES,
        super::AUTHORITY,
    )?;
    super::fields::texts(object, "non_claims", super::VALID)?;
    assert!(crate::syntax::is_full_revision(&choregraph_revision));
    assert_eq!(object.len(), MIGRATED_FIELDS.len());
    Ok(super::Pilot {
        id: pilot_id(object)?,
        conformance: conformance(object)?,
        subject: super::Subject::MigratedConsumer {
            repository,
            choregraph_revision,
        },
    })
}

fn pilot_id(object: &Object) -> Admitted<String> {
    let id = super::fields::text(object, "id", super::VALID)?;
    if !crate::syntax::is_source_id(id) {
        return Err(super::fields::violation(
            super::VALID,
            format!("consumer pilot id is not kebab-case: {id}"),
        ));
    }
    Ok(id.to_owned())
}

fn conformance(object: &Object) -> Admitted<super::Conformance> {
    let conformance =
        super::fields::section(object, "conformance", CONFORMANCE_FIELDS, super::VALID)?;
    super::fields::text(conformance, "command", super::VALID)?;
    Ok(super::Conformance {
        fixtures: super::fields::positive(conformance, "fixtures", super::VALID)?,
        transcript_digest: super::fields::digest(conformance, "transcript_digest", super::VALID)?
            .to_owned(),
        reordered_profile_digest: super::fields::digest(
            conformance,
            "reordered_profile_digest",
            super::VALID,
        )?
        .to_owned(),
    })
}

fn require_unique_ids(pilots: &[super::Pilot]) -> Admitted<()> {
    for (index, pilot) in pilots.iter().enumerate() {
        let earlier = pilots.get(..index).unwrap_or_default();
        if earlier.iter().any(|previous| previous.id == pilot.id) {
            return Err(super::fields::violation(
                super::VALID,
                format!("duplicate consumer pilot id: {}", pilot.id),
            ));
        }
    }
    Ok(())
}

/// `campaign.consumer_pilots` needs an independent in-repo fixture and a migrated consumer.
fn require_both_kinds(pilots: &[super::Pilot]) -> Admitted<()> {
    let has_fixture = pilots
        .iter()
        .any(|pilot| matches!(pilot.subject, super::Subject::InRepoFixture { .. }));
    let has_migrated = pilots
        .iter()
        .any(|pilot| matches!(pilot.subject, super::Subject::MigratedConsumer { .. }));
    if has_fixture && has_migrated {
        return Ok(());
    }
    Err(super::fields::violation(
        super::VALID,
        "consumer pilots must include an in-repo fixture and a migrated consumer",
    ))
}
