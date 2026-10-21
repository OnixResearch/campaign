//! Controls separate code attributes from harmless strings and comments.

fn source(content: &str) -> crate::model::SourceFile {
    crate::model::SourceFile {
        display_path: "crates/example/src/lib.rs".to_owned(),
        content: content.to_owned(),
    }
}

#[test]
fn accepts_checked_code_and_narrow_item_allowance() -> Result<(), crate::failure::CheckError> {
    super::validate(&source(
        "#![no_std]\n#[allow(dead_code, reason = \"fixture owner tests unused items\")] fn helper() {}",
    ))?;
    super::validate(&source(
        "// #![allow(unused)]\nconst TEXT: &str = \"#![expect(unused)]\";",
    ))
}

/// The exact refusal message for one candidate source.
fn refusal(candidate: &crate::model::SourceFile) -> Option<String> {
    super::validate(candidate)
        .err()
        .map(|error| error.message().to_owned())
}

#[test]
fn rejects_file_module_and_conditional_suppressions() {
    let cases = [
        "#![expect(assertion_density)] fn example() {}",
        "#[allow(unused)] mod unchecked { fn example() {} }",
        "mod nested { #![allow(unused)] fn example() {} }",
        "#![cfg_attr(test, expect(assertion_density))] fn example() {}",
        "#[cfg_attr(test, cfg_attr(unix, allow(unused)))] mod nested {}",
    ];
    for case in cases {
        assert_eq!(
            refusal(&source(case)).as_deref(),
            Some(
                "crates/example/src/lib.rs: file-wide or module-wide lint suppression is not admitted"
            ),
            "{case}"
        );
    }
}

#[test]
fn rejects_excluded_source_paths() {
    let mut candidate = source("fn helper() {}");
    candidate.display_path = "crates/example/src/generated/implementation.rs".to_owned();
    assert_eq!(
        refusal(&candidate).as_deref(),
        Some("Rust source occupies an excluded lint path")
    );
}

/// `include!` is refused under any path, rename, or macro body, since `core::include!` and
/// `std::include!` are the same macro; the refusal carries its rule.
#[test]
// r[verify campaign.functional_core.effect]
fn rejects_source_include_under_any_path_and_malformed_rust() {
    for case in [
        "include!(\"hidden.rs\");",
        "core::include!(\"clock.inc\");",
        "fn f() { ::std::include!(\"clock.inc\"); }",
        "macro_rules! splice { ($file:literal) => { include!($file); }; }",
        "use core::include as splice;",
    ] {
        let error = super::validate(&source(case)).err();
        assert_eq!(
            error.as_ref().and_then(crate::failure::CheckError::rule),
            Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT),
            "{case}"
        );
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some(
                "crates/example/src/lib.rs: identifier `include` names include! source \
                 indirection, which is not admitted"
            ),
            "{case}"
        );
    }
    assert_eq!(
        refusal(&source("const TEXT: &str = include_str!(\"x\");")),
        None
    );
    assert_eq!(
        refusal(&source("fn incomplete(")).as_deref(),
        Some("invalid Rust in crates/example/src/lib.rs: cannot parse string into token stream")
    );
}
