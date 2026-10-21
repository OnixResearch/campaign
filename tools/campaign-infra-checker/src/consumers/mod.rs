//! Typed consumer pilot record exported from `provenance/consumers.ncl`.
//!
//! Admission (`json`) enforces the record's structure; `validate` then checks it against the
//! repository: in-repo pilot paths exist, every migrated consumer is an inventoried source that
//! runs the Choregraph revision Campaign pins, every pilot claims exactly the suite evidence
//! campaign-conformance pins (`evidence`), and a registered, armed test reproduces those pins
//! (`registration`).

mod calls;
mod evidence;
mod fields;
pub(crate) mod json;
mod registration;
mod sections;

const VALID: crate::rules::Rule = crate::rules::Rule::CONSUMER_PILOTS_VALID;
const MIGRATION: crate::rules::Rule = crate::rules::Rule::CHAOSCONTROL_MIGRATION;
const PARITY: crate::rules::Rule = crate::rules::Rule::CHAOSCONTROL_MIGRATION_PARITY;
const AUTHORITY: crate::rules::Rule = crate::rules::Rule::CHAOSCONTROL_MIGRATION_AUTHORITY;

/// Conformance evidence one pilot recorded for the shared suite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Conformance {
    pub(crate) fixtures: u64,
    pub(crate) transcript_digest: String,
    pub(crate) reordered_profile_digest: String,
}

/// What distinguishes the two pilot kinds after admission.
#[derive(Debug)]
pub(crate) enum Subject {
    /// An in-repo fixture adapter: its adapter path and the test paths that run it.
    InRepoFixture { campaign_paths: Vec<String> },
    /// A consumer migrated onto a pinned Campaign revision.
    MigratedConsumer {
        repository: String,
        choregraph_revision: String,
    },
}

/// One admitted consumer pilot.
#[derive(Debug)]
pub(crate) struct Pilot {
    pub(crate) id: String,
    pub(crate) conformance: Conformance,
    pub(crate) subject: Subject,
}

/// The rank orders of the record's `reordered_profile`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ReorderedProfile {
    pub(crate) exploration_order: Vec<u64>,
    pub(crate) retention_rank_order: Vec<u64>,
}

/// Every recorded pilot, with unique ids and both kinds present, and the reordered profile.
#[derive(Debug)]
pub(crate) struct Pilots {
    pub(crate) reordered_profile: ReorderedProfile,
    pub(crate) pilots: Vec<Pilot>,
}

impl Pilots {
    /// Campaign paths named by in-repo fixture pilots.
    pub(crate) fn campaign_paths(&self) -> impl Iterator<Item = &String> {
        self.pilots.iter().flat_map(|pilot| match &pilot.subject {
            Subject::InRepoFixture { campaign_paths } => campaign_paths.as_slice(),
            Subject::MigratedConsumer { .. } => &[],
        })
    }
}

/// The admitted record and the repository facts it is checked against.
#[derive(Clone, Copy)]
pub(crate) struct Repository<'facts> {
    pub(crate) facts: &'facts crate::model::ConsumerFacts,
    pub(crate) sources: &'facts crate::inventory::Sources,
    pub(crate) members: &'facts [crate::model::Member],
}

/// Check the admitted pilots against the repository, the source inventory, and the pinned suite.
pub(crate) fn validate(repository: Repository<'_>) -> Result<(), crate::failure::CheckError> {
    let Repository {
        facts,
        sources,
        members,
    } = repository;
    assert!(
        facts.pilots.pilots.len() >= 2,
        "admission requires an in-repo fixture and a migrated consumer"
    );
    assert!(facts.absent_campaign_paths.len() <= facts.pilots.campaign_paths().count());
    if let Some(path) = facts.absent_campaign_paths.first() {
        return Err(fields::violation(
            VALID,
            format!("consumer record names absent Campaign path: {path}"),
        ));
    }
    let campaign_choregraph = sources
        .entry(crate::policy::CHOREGRAPH_SOURCE_ID)
        .map(|entry| entry.revision.as_str());
    facts
        .pilots
        .pilots
        .iter()
        .try_for_each(|pilot| match &pilot.subject {
            Subject::InRepoFixture { .. } => Ok(()),
            Subject::MigratedConsumer {
                repository,
                choregraph_revision,
            } => validate_migration(Migration {
                id: &pilot.id,
                repository,
                choregraph_revision,
                sources,
                campaign_choregraph,
            }),
        })?;
    evidence::validate(&facts.pilots, members)?;
    registration::validate(members)
}

/// One migrated consumer's pinned facts and the Campaign facts they must match.
#[derive(Clone, Copy)]
struct Migration<'facts> {
    id: &'facts str,
    repository: &'facts str,
    choregraph_revision: &'facts str,
    sources: &'facts crate::inventory::Sources,
    campaign_choregraph: Option<&'facts str>,
}

/// The consumer is an inventoried source and runs the Choregraph revision Campaign pins.
// r[impl campaign.chaoscontrol_migration]
fn validate_migration(migration: Migration<'_>) -> Result<(), crate::failure::CheckError> {
    let Migration { id, repository, .. } = migration;
    assert!(crate::syntax::is_full_revision(
        migration.choregraph_revision
    ));
    assert!(!repository.trim().is_empty());
    let is_inventoried = migration
        .sources
        .entries
        .iter()
        .any(|entry| entry.repository == repository);
    if !is_inventoried {
        return Err(fields::violation(
            MIGRATION,
            format!("{id}.repository {repository} is not an inventoried source repository"),
        ));
    }
    if migration.campaign_choregraph != Some(migration.choregraph_revision) {
        return Err(fields::violation(
            MIGRATION,
            format!(
                "{id}.pins.choregraph_revision {} differs from Campaign's pinned {} revision",
                migration.choregraph_revision,
                crate::policy::CHOREGRAPH_SOURCE_ID
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
