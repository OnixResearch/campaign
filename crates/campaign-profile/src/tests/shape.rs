//! `campaign.configuration.invalid`: JSON outside the closed profile shape is rejected while decoding.

/// Rejects `document` and returns its decode category and message.
fn decode(document: &serde_json::Value) -> Result<(crate::DecodeCategory, String), String> {
    super::decoded(super::rejected(document)?)
}

/// Rejects exported-profile `text` and returns its decode category and message.
fn decode_text(text: &str) -> Result<(crate::DecodeCategory, String), String> {
    match crate::CampaignProfile::from_json(text) {
        Ok(profile) => Err(format!("profile unexpectedly loaded: {profile:?}")),
        Err(error) => super::decoded(error),
    }
}

#[test]
fn unknown_fields_are_rejected_in_every_record() -> super::TestResult {
    for pointer in [
        "",
        "/owners",
        "/versions",
        "/bounds",
        "/selection",
        "/selection/decay",
        "/stopping",
        "/retention",
        "/admission",
    ] {
        let (category, message) = decode(&super::with_key(
            &format!("{pointer}/maximum_entropy_value"),
            Some(serde_json::json!(1)),
        )?)?;
        assert_eq!(category, crate::DecodeCategory::Data, "{pointer}");
        assert!(
            message.starts_with("unknown field `maximum_entropy_value`"),
            "{pointer}: {message}"
        );
    }
    Ok(())
}

#[test]
fn missing_and_null_fields_are_rejected() -> super::TestResult {
    for (path, field) in [
        ("/bounds/maximum_depth", "maximum_depth"),
        ("/owners/stopping", "stopping"),
        ("/selection/tie_break", "tie_break"),
        ("/receipt_non_claims", "receipt_non_claims"),
    ] {
        let (category, message) = decode(&super::with_key(path, None)?)?;
        assert_eq!(category, crate::DecodeCategory::Data, "{path}");
        assert!(
            message.starts_with(&format!("missing field `{field}`")),
            "{path}: {message}"
        );
    }
    for path in [
        "/selection/exploratory_threshold_parts_per_million",
        "/selection/exploration_order",
        "/selection/decay/penalty_per_selection",
        "/stopping/finding_limit",
    ] {
        let (category, message) = decode(&super::with_key(path, Some(serde_json::Value::Null))?)?;
        assert_eq!(category, crate::DecodeCategory::Data, "{path}");
        assert!(
            message.starts_with("invalid type: null"),
            "{path}: {message}"
        );
    }
    Ok(())
}

#[test]
fn non_integer_numbers_are_rejected() -> super::TestResult {
    for (pointer, value) in [
        ("/bounds/maximum_roots", serde_json::json!(256.5)),
        ("/bounds/maximum_roots", serde_json::json!(256.0)),
        (
            "/selection/exploratory_threshold_parts_per_million",
            serde_json::json!(0.5),
        ),
        ("/retention/rank_order", serde_json::json!([0.0])),
        ("/selection/exploration_order", serde_json::json!([1.5])),
        ("/versions/campaign_schema", serde_json::json!(1.0)),
    ] {
        let (category, message) = decode(&super::with(pointer, value)?)?;
        assert_eq!(category, crate::DecodeCategory::Data, "{pointer}");
        assert!(
            message.starts_with("invalid type: floating point"),
            "{pointer}: {message}"
        );
    }
    let (_, negative) = decode(&super::with(
        "/bounds/maximum_depth",
        serde_json::json!(-1),
    )?)?;
    assert!(
        negative.starts_with("invalid value: integer `-1`"),
        "{negative}"
    );
    let (_, wide_index) = decode(&super::with(
        "/retention/rank_order",
        serde_json::json!([4_294_967_296_u64]),
    )?)?;
    assert!(
        wide_index.starts_with("invalid value: integer `4294967296`"),
        "{wide_index}"
    );
    // A bound above 2^64-1 cannot be represented as a JSON value here, so edit the exported text.
    let text = crate::load::DEFAULT_PROFILE_JSON.replace(
        "\"maximum_roots\": 256,",
        "\"maximum_roots\": 18446744073709551616,",
    );
    assert_ne!(text, crate::load::DEFAULT_PROFILE_JSON);
    let (category, over_u64) = decode_text(&text)?;
    assert_eq!(category, crate::DecodeCategory::Data);
    assert!(
        over_u64.starts_with("invalid type: floating point"),
        "{over_u64}"
    );
    Ok(())
}

#[test]
fn unknown_enum_tags_are_rejected() -> super::TestResult {
    for (pointer, tag) in [
        ("/implementation", "reference"),
        ("/selection/mode", "greedy"),
        ("/selection/decay/rule", "subtract-from-first-rank"),
        ("/selection/tie_break", "random"),
        ("/admission/duplicate_candidate", "replace"),
        ("/receipt_non_claims/0", "exhaustive"),
    ] {
        let (category, message) = decode(&super::with(pointer, serde_json::json!(tag))?)?;
        assert_eq!(category, crate::DecodeCategory::Data, "{pointer}");
        assert!(
            message.starts_with(&format!("unknown variant `{tag}`")),
            "{pointer}: {message}"
        );
    }
    Ok(())
}

/// The default document with the record at `pointer` (`""` for the document) rewritten as the positional array
/// of its `fields` in declaration order: the spelling a derived serde struct impl would decode as the same record.
fn positional(pointer: &str, fields: &[&str]) -> Result<serde_json::Value, String> {
    let document = super::default_document()?;
    let record = document
        .pointer(pointer)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| format!("fixture record {pointer} is absent"))?;
    let values = fields
        .iter()
        .filter_map(|field| record.get(*field).cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        values.len(),
        record.len(),
        "{pointer} lists every present field"
    );
    assert!(
        record.keys().all(|key| fields.contains(&key.as_str())),
        "{pointer} fields name every record key"
    );
    if pointer.is_empty() {
        return Ok(serde_json::Value::Array(values));
    }
    super::with(pointer, serde_json::Value::Array(values))
}

#[test]
fn records_decode_only_from_json_objects() -> super::TestResult {
    for (pointer, fields) in [
        (
            "",
            &[
                "schema",
                "implementation",
                "owners",
                "versions",
                "bounds",
                "selection",
                "stopping",
                "retention",
                "admission",
                "receipt_non_claims",
            ][..],
        ),
        ("/owners", &super::rejection::OWNER_GROUPS[..]),
        ("/versions", &["campaign_schema", "identity_version"][..]),
        ("/bounds", &super::rejection::BOUND_KEYS[..]),
        (
            "/selection",
            &[
                "mode",
                "exploratory_threshold_parts_per_million",
                "exploration_order",
                "decay",
                "tie_break",
            ][..],
        ),
        ("/selection/decay", &["rule", "penalty_per_selection"][..]),
        ("/stopping", &["finding_limit"][..]),
        ("/retention", &["rank_order"][..]),
        ("/admission", &["duplicate_candidate"][..]),
    ] {
        let (category, message) = decode(&positional(pointer, fields)?)?;
        assert_eq!(category, crate::DecodeCategory::Data, "{pointer}");
        assert!(
            message.starts_with("invalid type: sequence, expected a JSON object record"),
            "{pointer}: {message}"
        );
    }
    Ok(())
}

#[test]
fn enum_tags_decode_only_from_json_strings() -> super::TestResult {
    for (pointer, tag) in [
        ("/implementation", "production"),
        ("/selection/mode", "exploratory"),
        ("/selection/decay/rule", "none"),
        ("/selection/tie_break", "candidate_identity"),
        ("/admission/duplicate_candidate", "reject"),
        ("/receipt_non_claims/0", "exhaustive_search"),
    ] {
        let mut unit_variant = serde_json::Map::new();
        unit_variant.insert(tag.to_owned(), serde_json::Value::Null);
        for (value, unexpected) in [
            (serde_json::Value::Object(unit_variant), "map"),
            (serde_json::Value::Null, "null"),
            (serde_json::json!([tag]), "sequence"),
            (serde_json::json!(0), "integer `0`"),
        ] {
            let (category, message) = decode(&super::with(pointer, value)?)?;
            assert_eq!(category, crate::DecodeCategory::Data, "{pointer}");
            assert!(
                message.starts_with(&format!(
                    "invalid type: {unexpected}, expected an enum tag string"
                )),
                "{pointer}: {message}"
            );
        }
    }
    let (category, message) = decode(&super::with(
        "/receipt_non_claims",
        serde_json::json!({ "exhaustive_search": null }),
    )?)?;
    assert_eq!(category, crate::DecodeCategory::Data);
    assert!(
        message.starts_with("invalid type: map, expected an array of at most 8 enum tag strings"),
        "{message}"
    );
    let mut over_long = super::default_document()?;
    over_long
        .pointer_mut("/receipt_non_claims")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("default non-claims are absent")?
        .push(serde_json::json!("authorization"));
    let (category, message) = decode(&over_long)?;
    assert_eq!(category, crate::DecodeCategory::Data);
    assert!(
        message.starts_with("invalid length 9, expected an array of at most 8 enum tag strings"),
        "{message}"
    );
    Ok(())
}

#[test]
fn duplicate_keys_and_malformed_text_are_rejected() -> super::TestResult {
    let duplicate = crate::load::DEFAULT_PROFILE_JSON.replacen(
        "\"schema\": \"campaign.profile.v2\",",
        "\"schema\": \"campaign.profile.v2\", \"schema\": \"campaign.profile.v2\",",
        1,
    );
    assert_ne!(duplicate, crate::load::DEFAULT_PROFILE_JSON);
    let (category, message) = decode_text(&duplicate)?;
    assert_eq!(category, crate::DecodeCategory::Data);
    assert!(message.starts_with("duplicate field `schema`"), "{message}");
    let (truncated, _) = decode_text("{\"schema\": ")?;
    assert_eq!(truncated, crate::DecodeCategory::Eof);
    let (malformed, _) = decode_text("{\"schema\" \"campaign.profile.v2\"}")?;
    assert_eq!(malformed, crate::DecodeCategory::Syntax);
    Ok(())
}
