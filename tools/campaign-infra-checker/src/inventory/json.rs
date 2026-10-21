//! Strict admission of the exported inventory JSON: closed records, exact types, immutable sources.

const INVENTORY_FIELDS: &[&str] = &["schema", "sources"];
const ENTRY_FIELDS: &[&str] = &[
    "id",
    "repository",
    "revision",
    "license",
    "owner",
    "adoption",
    "reviewed_files",
    "adopted_mechanisms",
    "campaign_paths",
    "retained_authority",
    "non_adoption",
    "notes",
];
const MOVING_URL_SEGMENTS: &[&str] = &["/tree/", "/blob/", "/commit/", "/commits/", "/releases/"];
const RADICLE_PREFIX: &str = "rad:z";
const HTTPS_PREFIX: &str = "https://";
const ADOPTIONS: &[(&str, super::Adoption)] = &[
    ("dependency", super::Adoption::Dependency),
    ("vendored_copy", super::Adoption::VendoredCopy),
    ("reference_only", super::Adoption::ReferenceOnly),
];

type Object = serde_json::Map<String, serde_json::Value>;

/// The identity fields of one admitted entry.
struct Identity<'value> {
    id: &'value str,
    repository: &'value str,
    revision: &'value str,
    adoption: super::Adoption,
}

// r[impl campaign.source_inventory]
pub(crate) fn parse(
    text: &crate::model::NamedText,
) -> Result<super::Sources, crate::failure::CheckError> {
    let value: serde_json::Value = serde_json::from_str(&text.content).map_err(|error| {
        super::violation(format!("{} is not valid JSON: {error}", text.relative_path))
    })?;
    let object = closed_object(&value, INVENTORY_FIELDS)?;
    if object.get("schema").and_then(serde_json::Value::as_str)
        != Some(crate::policy::SOURCE_INVENTORY_SCHEMA)
    {
        return Err(super::violation(format!(
            "inventory schema must be {}",
            crate::policy::SOURCE_INVENTORY_SCHEMA
        )));
    }
    let sources = bounded_array(object, "sources", crate::policy::MAX_INVENTORY_SOURCES)?;
    if sources.is_empty() {
        return Err(super::violation("inventory records no sources"));
    }
    let entries = sources
        .iter()
        .map(parse_entry)
        .collect::<Result<Vec<_>, _>>()?;
    reject_duplicate_ids(&entries)?;
    assert_eq!(entries.len(), sources.len());
    assert!(!entries.is_empty());
    Ok(super::Sources { entries })
}

// r[impl campaign.source_inventory]
fn parse_entry(value: &serde_json::Value) -> Result<super::Entry, crate::failure::CheckError> {
    let object = closed_object(value, ENTRY_FIELDS)?;
    let identity = entry_identity(object)?;
    text_field(object, "owner")?;
    text_field(object, "retained_authority")?;
    relative_paths(object, "reviewed_files")?;
    text_list(object, "adopted_mechanisms", false)?;
    text_list(object, "non_adoption", false)?;
    text_list(object, "notes", true)?;
    let campaign_paths = relative_paths(object, "campaign_paths")?;
    assert!(!campaign_paths.is_empty());
    assert_eq!(object.len(), ENTRY_FIELDS.len());
    Ok(super::Entry {
        id: identity.id.to_owned(),
        repository: identity.repository.to_owned(),
        revision: identity.revision.to_owned(),
        adoption: identity.adoption,
        campaign_paths,
    })
}

fn entry_identity(object: &Object) -> Result<Identity<'_>, crate::failure::CheckError> {
    let id = text_field(object, "id")?;
    if !crate::syntax::is_source_id(id) {
        return Err(super::violation(format!(
            "source id is not kebab-case: {id}"
        )));
    }
    let repository = text_field(object, "repository")?;
    if !is_immutable_repository(repository) {
        return Err(super::violation(format!(
            "{id}.repository is not an immutable repository location: {repository}"
        )));
    }
    let revision = text_field(object, "revision")?;
    if !crate::syntax::is_full_revision(revision) {
        return Err(super::violation(format!(
            "{id}.revision is not a full lowercase 40-hex revision: {revision}"
        )));
    }
    let adoption = parse_adoption(object)?;
    super::license::validate(super::license::Declared {
        expression: text_field(object, "license")?,
        adoption,
    })
    .map_err(|reason| super::violation(format!("{id}.license: {reason}")))?;
    assert!(crate::syntax::is_source_id(id));
    assert!(crate::syntax::is_full_revision(revision));
    Ok(Identity {
        id,
        repository,
        revision,
        adoption,
    })
}

fn closed_object<'value>(
    value: &'value serde_json::Value,
    fields: &[&str],
) -> Result<&'value Object, crate::failure::CheckError> {
    let object = value
        .as_object()
        .ok_or_else(|| super::violation("inventory record is not an object"))?;
    if let Some(missing) = fields.iter().find(|field| !object.contains_key(**field)) {
        return Err(super::violation(format!(
            "inventory record lacks field {missing}"
        )));
    }
    if let Some(unknown) = object.keys().find(|key| !fields.contains(&key.as_str())) {
        return Err(super::violation(format!(
            "inventory record has unknown field {unknown}"
        )));
    }
    assert_eq!(object.len(), fields.len());
    Ok(object)
}

fn text_field<'value>(
    object: &'value Object,
    field: &str,
) -> Result<&'value str, crate::failure::CheckError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| super::violation(format!("field {field} must be a non-empty string")))
}

fn bounded_array<'value>(
    object: &'value Object,
    field: &str,
    maximum: usize,
) -> Result<&'value Vec<serde_json::Value>, crate::failure::CheckError> {
    let values = object
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| super::violation(format!("field {field} must be an array")))?;
    if values.len() > maximum {
        return Err(super::violation(format!(
            "field {field} exceeds {maximum} items"
        )));
    }
    Ok(values)
}

fn text_list(
    object: &Object,
    field: &str,
    is_empty_allowed: bool,
) -> Result<Vec<String>, crate::failure::CheckError> {
    let values = bounded_array(object, field, crate::policy::MAX_INVENTORY_LIST_ITEMS)?;
    if values.is_empty() && !is_empty_allowed {
        return Err(super::violation(format!("field {field} must not be empty")));
    }
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    super::violation(format!("field {field} must contain non-empty strings"))
                })
        })
        .collect()
}

fn relative_paths(object: &Object, field: &str) -> Result<Vec<String>, crate::failure::CheckError> {
    let paths = text_list(object, field, false)?;
    if let Some(path) = paths
        .iter()
        .find(|path| !crate::syntax::is_relative_path(path))
    {
        return Err(super::violation(format!(
            "field {field} contains a non-relative path: {path}"
        )));
    }
    Ok(paths)
}

fn parse_adoption(object: &Object) -> Result<super::Adoption, crate::failure::CheckError> {
    let text = text_field(object, "adoption")?;
    ADOPTIONS
        .iter()
        .find(|(name, _)| *name == text)
        .map(|(_, adoption)| *adoption)
        .ok_or_else(|| super::violation(format!("unknown adoption kind: {text}")))
}

fn is_immutable_repository(location: &str) -> bool {
    if let Some(identity) = location.strip_prefix(RADICLE_PREFIX) {
        return !identity.is_empty() && identity.bytes().all(|byte| byte.is_ascii_alphanumeric());
    }
    location.strip_prefix(HTTPS_PREFIX).is_some_and(|rest| {
        !rest.is_empty()
            && !rest.contains(|character: char| {
                character.is_whitespace() || character == '?' || character == '#'
            })
            && !MOVING_URL_SEGMENTS
                .iter()
                .any(|segment| location.contains(segment))
    })
}

fn reject_duplicate_ids(entries: &[super::Entry]) -> Result<(), crate::failure::CheckError> {
    for (index, entry) in entries.iter().enumerate() {
        let earlier = entries.get(..index).unwrap_or_default();
        if earlier.iter().any(|previous| previous.id == entry.id) {
            return Err(super::violation(format!(
                "duplicate source id: {}",
                entry.id
            )));
        }
    }
    Ok(())
}
