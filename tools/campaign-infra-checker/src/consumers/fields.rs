//! Closed-record and exact-type primitives for consumer record admission; each failure carries
//! the specification rule of the section being admitted.

pub(super) type Object = serde_json::Map<String, serde_json::Value>;
pub(super) type Admitted<T> = Result<T, crate::failure::CheckError>;

pub(super) fn violation(
    rule: crate::rules::Rule,
    message: impl AsRef<str>,
) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(rule, message)
}

/// The object with exactly `fields`, no more and no fewer.
pub(super) fn closed<'value>(
    value: &'value serde_json::Value,
    fields: &[&str],
    rule: crate::rules::Rule,
) -> Admitted<&'value Object> {
    let object = value
        .as_object()
        .ok_or_else(|| violation(rule, "consumer record value is not an object"))?;
    if let Some(missing) = fields.iter().find(|name| !object.contains_key(**name)) {
        return Err(violation(
            rule,
            format!("consumer record object lacks field {missing}"),
        ));
    }
    if let Some(unknown) = object.keys().find(|key| !fields.contains(&key.as_str())) {
        return Err(violation(
            rule,
            format!("consumer record object has unknown field {unknown}"),
        ));
    }
    assert_eq!(object.len(), fields.len());
    Ok(object)
}

pub(super) fn field<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<&'value serde_json::Value> {
    object
        .get(name)
        .ok_or_else(|| violation(rule, format!("consumer record lacks field {name}")))
}

/// The closed object held by `name`.
pub(super) fn section<'value>(
    object: &'value Object,
    name: &str,
    fields: &[&str],
    rule: crate::rules::Rule,
) -> Admitted<&'value Object> {
    closed(field(object, name, rule)?, fields, rule)
}

pub(super) fn text<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<&'value str> {
    object
        .get(name)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| violation(rule, format!("field {name} must be a non-empty string")))
}

/// A non-empty, bounded array.
pub(super) fn list<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<&'value [serde_json::Value]> {
    let values = field(object, name, rule)?
        .as_array()
        .ok_or_else(|| violation(rule, format!("field {name} must be an array")))?;
    if values.is_empty() {
        return Err(violation(rule, format!("field {name} must not be empty")));
    }
    if values.len() > crate::policy::MAX_CONSUMER_LIST_ITEMS {
        return Err(violation(
            rule,
            format!(
                "field {name} exceeds {} items",
                crate::policy::MAX_CONSUMER_LIST_ITEMS
            ),
        ));
    }
    Ok(values)
}

pub(super) fn texts<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<Vec<&'value str>> {
    list(object, name, rule)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| {
                    violation(rule, format!("field {name} must contain non-empty strings"))
                })
        })
        .collect()
}

pub(super) fn relative_paths<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<Vec<&'value str>> {
    let paths = texts(object, name, rule)?;
    if let Some(path) = paths
        .iter()
        .find(|path| !crate::syntax::is_relative_path(path))
    {
        return Err(violation(
            rule,
            format!("field {name} contains a non-relative path: {path}"),
        ));
    }
    Ok(paths)
}

pub(super) fn relative_path<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<&'value str> {
    let path = text(object, name, rule)?;
    if !crate::syntax::is_relative_path(path) {
        return Err(violation(
            rule,
            format!("field {name} is not a relative path: {path}"),
        ));
    }
    Ok(path)
}

pub(super) fn revision<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<&'value str> {
    let revision = text(object, name, rule)?;
    if !crate::syntax::is_full_revision(revision) {
        return Err(violation(
            rule,
            format!("{name} is not a full lowercase 40-hex revision: {revision}"),
        ));
    }
    Ok(revision)
}

pub(super) fn digest<'value>(
    object: &'value Object,
    name: &str,
    rule: crate::rules::Rule,
) -> Admitted<&'value str> {
    let digest = text(object, name, rule)?;
    if !crate::syntax::is_blake3_digest(digest) {
        return Err(violation(
            rule,
            format!("{name} is not a lowercase 64-hex BLAKE3 digest: {digest}"),
        ));
    }
    Ok(digest)
}

pub(super) fn positive(object: &Object, name: &str, rule: crate::rules::Rule) -> Admitted<u64> {
    let count = natural(object, name, rule)?;
    if count == 0 {
        return Err(violation(
            rule,
            format!("field {name} must be a positive integer"),
        ));
    }
    Ok(count)
}

pub(super) fn natural(object: &Object, name: &str, rule: crate::rules::Rule) -> Admitted<u64> {
    object
        .get(name)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| violation(rule, format!("field {name} must be a non-negative integer")))
}

pub(super) fn has_repeat<T: PartialEq>(values: &[T]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values.get(..index).unwrap_or_default().contains(value))
}
