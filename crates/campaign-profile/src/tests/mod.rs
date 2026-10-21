//! Profile loader tests: the embedded default, every rejected value class, and runtime consumption.

mod consumption;
mod defaults;
mod rejection;
mod relations;
mod representability;
mod shape;

const FIXTURE_LABEL: &[u8] = b"campaign-profile-fixture";

type TestResult = Result<(), String>;

/// Deterministic identity of `kind` for a fixture `label`.
fn id(kind: campaign_core::IdKind, label: &[u8]) -> campaign_core::Id {
    campaign_core::derive_id(
        kind,
        FIXTURE_LABEL,
        &[campaign_core::derive_id(kind, label, &[])],
    )
}

/// The embedded default profile as a JSON value to edit.
fn default_document() -> Result<serde_json::Value, String> {
    serde_json::from_str(crate::load::DEFAULT_PROFILE_JSON).map_err(|error| error.to_string())
}

/// The default document with the value at `pointer` replaced.
fn with(pointer: &str, value: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut document = default_document()?;
    let slot = document
        .pointer_mut(pointer)
        .ok_or_else(|| format!("fixture pointer {pointer} is absent"))?;
    *slot = value;
    Ok(document)
}

/// The default document with the key at `path` (`<object pointer>/<key>`) inserted, replaced, or removed.
fn with_key(path: &str, value: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let (pointer, key) = path
        .rsplit_once('/')
        .ok_or_else(|| format!("fixture path {path} has no key"))?;
    let mut document = default_document()?;
    let object = document
        .pointer_mut(pointer)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| format!("fixture object {pointer} is absent"))?;
    let is_inserted = value.is_some();
    match value {
        Some(value) => {
            object.insert(key.to_owned(), value);
        }
        None => {
            object
                .remove(key)
                .ok_or_else(|| format!("fixture key {path} is absent"))?;
        }
    }
    assert_eq!(object.contains_key(key), is_inserted, "{path}");
    assert_ne!(
        document,
        default_document()?,
        "{path} left the default unchanged"
    );
    Ok(document)
}

/// Loads a JSON value through the public text entry point.
fn load(document: &serde_json::Value) -> crate::failure::ProfileResult<crate::CampaignProfile> {
    crate::CampaignProfile::from_json(&document.to_string())
}

/// Loads a JSON value that must be rejected and returns the error.
fn rejected(document: &serde_json::Value) -> Result<crate::ProfileError, String> {
    match load(document) {
        Ok(profile) => Err(format!("profile unexpectedly loaded: {profile:?}")),
        Err(error) => Ok(error),
    }
}

/// Splits a decode rejection into its category and decoder message.
fn decoded(error: crate::ProfileError) -> Result<(crate::DecodeCategory, String), String> {
    match error {
        crate::ProfileError::Decode { category, message } => Ok((category, message)),
        other => Err(format!("expected a decode error, got {other:?}")),
    }
}
