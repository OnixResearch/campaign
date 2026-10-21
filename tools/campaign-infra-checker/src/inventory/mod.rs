//! Typed Campaign source inventory exported from `provenance/sources.ncl`.

pub(crate) mod coverage;
pub(crate) mod declarations;
pub(crate) mod json;
mod license;

/// How Campaign uses an inventoried source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Adoption {
    Dependency,
    VendoredCopy,
    ReferenceOnly,
}

/// The inventory facts other checks consume; every other field is validated at admission.
#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) repository: String,
    pub(crate) revision: String,
    pub(crate) adoption: Adoption,
    pub(crate) campaign_paths: Vec<String>,
}

/// Every adopted source with unique identifiers.
#[derive(Debug)]
pub(crate) struct Sources {
    pub(crate) entries: Vec<Entry>,
}

impl Sources {
    pub(crate) fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub(crate) fn records_revision(&self, revision: &str) -> bool {
        self.entries.iter().any(|entry| entry.revision == revision)
    }
}

fn violation(message: impl AsRef<str>) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(crate::rules::Rule::SOURCE_INVENTORY_COMPLETE, message)
}

#[cfg(test)]
mod tests;
