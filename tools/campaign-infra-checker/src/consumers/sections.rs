//! Admission of a migrated consumer's pins, parity, KVM smoke, and retained authority.

type Admitted<T> = super::fields::Admitted<T>;
type Object = super::fields::Object;

const ADOPTION_FIELDS: &[&str] = &["change", "branch", "commit"];
const PIN_FIELDS: &[&str] = &[
    "cargo_campaign_revision",
    "nix_campaign_revision",
    "choregraph_revision",
];
const PARITY_FIELDS: &[&str] = &[
    "corpus_tests",
    "receipt",
    "cutover_ready",
    "agreeing_decision_classes",
];
const AGREEMENT_FIELDS: &[&str] = &["class", "compared_fields"];
const ARTIFACT_FIELDS: &[&str] = &["path", "blake3"];
const KVM_SMOKE_FIELDS: &[&str] = &["record", "receipt", "bounds"];
const BOUND_FIELDS: &[&str] = &[
    "vms",
    "rounds",
    "branches",
    "ticks",
    "seed",
    "mode",
    "memory_mb",
];
const POSITIVE_BOUNDS: &[&str] = &["vms", "rounds", "branches", "ticks", "memory_mb"];
/// Every authority a pilot may record as retained outside Campaign.
const AUTHORITIES: &[&str] = &[
    "execution",
    "snapshot",
    "coverage",
    "fault",
    "assertion",
    "finding",
    "replay",
    "minimization",
    "evidence",
    "promotion",
    "acceptance",
];
/// Authority a migrated consumer must retain (`campaign.chaoscontrol_migration.authority`).
pub(super) const MIGRATION_AUTHORITIES: &[&str] = &[
    "execution",
    "snapshot",
    "coverage",
    "fault",
    "assertion",
    "finding",
    "replay",
    "minimization",
    "evidence",
];
/// Decision classes the parity corpus must compare (`campaign.chaoscontrol_migration.parity`).
const DECISION_CLASSES: &[&str] = &[
    "candidate_order",
    "selected_parents",
    "priority_results",
    "pruning",
    "budget_use",
    "stop_classes",
];

pub(super) fn adoption(object: &Object) -> Admitted<()> {
    let adoption = super::fields::section(object, "adoption", ADOPTION_FIELDS, super::MIGRATION)?;
    let change = super::fields::text(adoption, "change", super::MIGRATION)?;
    if !crate::syntax::is_source_id(change) {
        return Err(super::fields::violation(
            super::MIGRATION,
            format!("adoption.change is not a kebab-case change id: {change}"),
        ));
    }
    super::fields::text(adoption, "branch", super::MIGRATION)?;
    super::fields::revision(adoption, "commit", super::MIGRATION)?;
    Ok(())
}

/// The consumer's Choregraph revision, once its Cargo and Nix Campaign pins agree.
// r[impl campaign.chaoscontrol_migration]
pub(super) fn pins(object: &Object) -> Admitted<String> {
    let pins = super::fields::section(object, "pins", PIN_FIELDS, super::MIGRATION)?;
    let cargo = super::fields::revision(pins, "cargo_campaign_revision", super::MIGRATION)?;
    let nix = super::fields::revision(pins, "nix_campaign_revision", super::MIGRATION)?;
    if cargo != nix {
        return Err(super::fields::violation(
            super::MIGRATION,
            format!("cargo Campaign pin {cargo} differs from nix Campaign pin {nix}"),
        ));
    }
    Ok(super::fields::revision(pins, "choregraph_revision", super::MIGRATION)?.to_owned())
}

// r[impl campaign.chaoscontrol_migration.parity]
pub(super) fn parity(object: &Object) -> Admitted<()> {
    let parity = super::fields::section(object, "parity", PARITY_FIELDS, super::PARITY)?;
    super::fields::relative_path(parity, "corpus_tests", super::PARITY)?;
    artifact(parity, "receipt", super::PARITY)?;
    require_cutover(parity)?;
    require_tags(
        Tags {
            field: "agreeing_decision_classes",
            names: &decision_classes(parity)?,
            known: DECISION_CLASSES,
            required: DECISION_CLASSES,
        },
        super::PARITY,
    )
}

/// `cutover_ready` is the boolean `true`, never a look-alike string or number.
fn require_cutover(parity: &Object) -> Admitted<()> {
    let ready = super::fields::field(parity, "cutover_ready", super::PARITY)?;
    if ready.as_bool() == Some(true) {
        return Ok(());
    }
    Err(super::fields::violation(
        super::PARITY,
        "parity must be cutover_ready",
    ))
}

fn decision_classes(parity: &Object) -> Admitted<Vec<&str>> {
    super::fields::list(parity, "agreeing_decision_classes", super::PARITY)?
        .iter()
        .map(|agreement| {
            let agreement = super::fields::closed(agreement, AGREEMENT_FIELDS, super::PARITY)?;
            super::fields::texts(agreement, "compared_fields", super::PARITY)?;
            super::fields::text(agreement, "class", super::PARITY)
        })
        .collect()
}

pub(super) fn kvm_smoke(object: &Object) -> Admitted<()> {
    let smoke = super::fields::section(object, "kvm_smoke", KVM_SMOKE_FIELDS, super::PARITY)?;
    artifact(smoke, "record", super::PARITY)?;
    artifact(smoke, "receipt", super::PARITY)?;
    let bounds = super::fields::section(smoke, "bounds", BOUND_FIELDS, super::PARITY)?;
    for name in POSITIVE_BOUNDS {
        super::fields::positive(bounds, name, super::PARITY)?;
    }
    super::fields::natural(bounds, "seed", super::PARITY)?;
    super::fields::text(bounds, "mode", super::PARITY)?;
    Ok(())
}

/// Admit `retained_authority`: known, unrepeated authorities that include every `required` one.
// r[impl campaign.chaoscontrol_migration.authority]
pub(super) fn retained_authority(
    object: &Object,
    required: &[&str],
    rule: crate::rules::Rule,
) -> Admitted<()> {
    require_tags(
        Tags {
            field: "retained_authority",
            names: &super::fields::texts(object, "retained_authority", rule)?,
            known: AUTHORITIES,
            required,
        },
        rule,
    )
}

/// A list of vocabulary names and the vocabulary it must draw from and cover.
#[derive(Clone, Copy)]
struct Tags<'names> {
    field: &'static str,
    names: &'names [&'names str],
    known: &'static [&'static str],
    required: &'names [&'names str],
}

fn require_tags(tags: Tags<'_>, rule: crate::rules::Rule) -> Admitted<()> {
    assert!(tags.required.iter().all(|name| tags.known.contains(name)));
    let field = tags.field;
    if let Some(unknown) = tags.names.iter().find(|name| !tags.known.contains(name)) {
        return Err(super::fields::violation(
            rule,
            format!("{field} names an unknown entry: {unknown}"),
        ));
    }
    if super::fields::has_repeat(tags.names) {
        return Err(super::fields::violation(
            rule,
            format!("{field} must not repeat an entry"),
        ));
    }
    let missing: Vec<&str> = tags
        .required
        .iter()
        .copied()
        .filter(|required| !tags.names.contains(required))
        .collect();
    assert!(missing.len() <= tags.required.len());
    if missing.is_empty() {
        return Ok(());
    }
    Err(super::fields::violation(
        rule,
        format!("{field} lacks {}", missing.join(", ")),
    ))
}

fn artifact(object: &Object, name: &str, rule: crate::rules::Rule) -> Admitted<()> {
    let artifact = super::fields::section(object, name, ARTIFACT_FIELDS, rule)?;
    super::fields::relative_path(artifact, "path", rule)?;
    super::fields::digest(artifact, "blake3", rule)?;
    Ok(())
}
