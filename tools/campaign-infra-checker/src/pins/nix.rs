//! `flake.nix` inputs and `flake.lock` nodes must select the inventoried Choregraph revision.

const INPUT_FAMILY: &str = "choregraph";
const GIT_URL_PREFIX: &str = "git+";
const MOVING_PARAMETERS: &[&str] = &["ref", "branch", "tag"];
const MOVING_URL_SCHEMES: &[&str] = &["path:", "file:", "git+file:"];
const URL_KEY: &str = "url";
const MAX_INPUT_ATTRIBUTE_GAP_BYTES: usize = 256;
const LOCK_SECTIONS: &[&str] = &["locked", "original"];

pub(super) fn validate_flake(
    text: &crate::model::NamedText,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let urls = input_urls(&text.content);
    if urls.is_empty() {
        return Err(super::mismatch(
            "flake.nix declares no Choregraph input URL",
        ));
    }
    for url in urls {
        validate_flake_url(url, pin)?;
    }
    Ok(())
}

pub(super) fn validate_flake_lock(
    text: &crate::model::NamedText,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let lock: serde_json::Value = serde_json::from_str(&text.content).map_err(|error| {
        crate::failure::CheckError::failed(format!("invalid {}: {error}", text.relative_path))
    })?;
    let nodes: Vec<(&String, &serde_json::Value)> = lock
        .get("nodes")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| crate::failure::CheckError::failed("flake.lock has no nodes object"))?
        .iter()
        .filter(|(name, _)| name.starts_with(INPUT_FAMILY))
        .collect();
    if nodes.is_empty() {
        return Err(super::mismatch("flake.lock locks no Choregraph input"));
    }
    assert!(!nodes.is_empty());
    assert!(nodes.iter().all(|(name, _)| name.starts_with(INPUT_FAMILY)));
    for (name, node) in nodes {
        validate_lock_node(name, node, pin)?;
    }
    Ok(())
}

fn validate_lock_node(
    name: &str,
    node: &serde_json::Value,
    pin: super::Pin<'_>,
) -> Result<(), crate::failure::CheckError> {
    let field = |section: &str, key: &str| {
        node.get(section)
            .and_then(|table| table.get(key))
            .and_then(serde_json::Value::as_str)
    };
    let has_original_ref = node
        .get("original")
        .is_some_and(|table| table.get("ref").is_some());
    if field("locked", "type") != Some("git")
        || has_original_ref
        || field("original", "rev").is_none()
    {
        return Err(super::moving(format!(
            "flake.lock input {name} is not an explicit Git revision selection"
        )));
    }
    for section in LOCK_SECTIONS {
        if field(section, "url") != Some(pin.repository)
            || field(section, "rev") != Some(pin.revision)
        {
            return Err(super::mismatch(format!(
                "flake.lock input {name}.{section} does not select {} at {}",
                pin.repository, pin.revision
            )));
        }
    }
    assert_eq!(field("original", "rev"), Some(pin.revision));
    assert!(!has_original_ref);
    Ok(())
}

// r[impl campaign.choregraph_dependency.moving]
fn validate_flake_url(url: &str, pin: super::Pin<'_>) -> Result<(), crate::failure::CheckError> {
    if MOVING_URL_SCHEMES
        .iter()
        .any(|scheme| url.starts_with(scheme))
    {
        return Err(super::moving(format!(
            "flake.nix selects Choregraph by path: {url}"
        )));
    }
    let Some(git_url) = url.strip_prefix(GIT_URL_PREFIX) else {
        return Err(super::mismatch(format!(
            "flake.nix Choregraph input is not a git+ URL: {url}"
        )));
    };
    let (repository, query) = git_url.split_once('?').unwrap_or((git_url, ""));
    let revision = query_revision(query).map_err(|reason| {
        super::moving(format!("flake.nix selects Choregraph by {reason}: {url}"))
    })?;
    if repository != pin.repository || revision != pin.revision {
        return Err(super::mismatch(format!(
            "flake.nix selects {url}; the inventory pins {} at {}",
            pin.repository, pin.revision
        )));
    }
    assert_eq!(revision, pin.revision);
    assert_eq!(repository, pin.repository);
    Ok(())
}

/// The `rev` parameter of a flake URL query, rejecting moving parameters.
fn query_revision(query: &str) -> Result<&str, &'static str> {
    let mut revision = None;
    for parameter in query.split('&').filter(|parameter| !parameter.is_empty()) {
        let (key, value) = parameter.split_once('=').unwrap_or((parameter, ""));
        if let Some(moving) = MOVING_PARAMETERS.iter().find(|moving| **moving == key) {
            return Err(moving);
        }
        if key == "rev" {
            revision = Some(value);
        }
    }
    revision.ok_or("a missing rev")
}

/// URL strings of `choregraph*` inputs written as `name = { url = "..."; }` or `name.url = "...";`.
fn input_urls(flake: &str) -> Vec<&str> {
    flake
        .match_indices(INPUT_FAMILY)
        .filter_map(|(start, _)| flake.get(start..).and_then(input_url))
        .collect()
}

/// The URL of the input whose name starts `input`, when the name opens an input attribute.
fn input_url(input: &str) -> Option<&str> {
    let name_end = input
        .find(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        .unwrap_or(input.len());
    let attribute = input.get(name_end..)?.trim_start();
    let is_input_open = attribute.starts_with(".url")
        || attribute
            .strip_prefix('=')
            .is_some_and(|value| value.trim_start().starts_with('{'));
    if is_input_open {
        next_url_value(attribute)
    } else {
        None
    }
}

fn next_url_value(text: &str) -> Option<&str> {
    let window = text.get(..MAX_INPUT_ATTRIBUTE_GAP_BYTES.min(text.len()))?;
    let after_key = text.get(window.find(URL_KEY)?.checked_add(URL_KEY.len())?..)?;
    let value = after_key
        .trim_start()
        .strip_prefix('=')?
        .trim_start()
        .strip_prefix('"')?;
    value.split_once('"').map(|(url, _)| url)
}
