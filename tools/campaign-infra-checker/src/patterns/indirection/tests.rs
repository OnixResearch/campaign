//! Compile-time reads and source indirection are refused wherever their tokens appear; neutral
//! identifiers, casts, precise-capturing `use<..>` bounds, `as _` imports, strings, and comments
//! stay admitted.

type Found = Result<Option<super::Finding>, crate::failure::CheckError>;

fn walk(content: &str, scope: super::Scope) -> Found {
    super::find_in(
        &crate::model::SourceFile {
            display_path: "crates/campaign-core/src/lib.rs".to_owned(),
            content: content.to_owned(),
        },
        scope,
    )
}

fn identifier(name: &str) -> super::Finding {
    super::Finding::Identifier(name.to_owned())
}

#[test]
// r[verify campaign.functional_core.effect]
fn admits_neutral_tokens_casts_and_unnamed_imports() -> Result<(), crate::failure::CheckError> {
    let content = r#"
        //! Mentions include!, env!, #[path], and macro_rules! only in documentation.
        #![no_std]
        #![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]
        use core::fmt::Write as _;
        use crate::{model::Bounds, policy::{plan, stop}};
        /// Environment-free identity.
        #[derive(Debug)]
        #[cfg_attr(test, derive(Clone))]
        pub struct Environment { path: u64, included: bool }
        pub fn widen(value: u32, path: &str) -> u64 {
            let environment = "include_bytes!(\"/proc\") option_env!(\"X\")";
            let _ = (environment, path);
            u64::from(value) + value as u64
        }
        pub fn captured<'a>(text: &'a str) -> impl Sized + use<'a> { let n = text.len() as u64; n }
    "#;
    assert_eq!(walk(content, super::SHARED)?, None);
    assert_eq!(walk(content, super::ANY_MEMBER)?, None);
    Ok(())
}

/// Round-9 reproducers (Audit9 finding 3) and the attribute and argument shapes of the same class:
/// each is refused at its first refused token.
#[test]
// r[verify campaign.functional_core.effect]
fn refuses_reads_renames_definitions_and_path_attributes_in_every_token_tree()
-> Result<(), crate::failure::CheckError> {
    let definition = Some(super::Finding::MacroDefinition);
    let path = Some(super::Finding::PathAttribute);
    let rename = |alias: &str| Some(super::Finding::Rename(alias.to_owned()));
    let cases = [
        (
            "macro_rules! call { ($m:ident) => { $m!() }; }",
            definition.clone(),
        ),
        (
            "macro_rules! splice { () => { #[path = \"clock.inc\"] mod clock; }; }",
            definition,
        ),
        (
            "const SEED: &[u8] = call!(include_bytes, \"/proc/sys/kernel/random/uuid\");",
            Some(identifier("include_bytes")),
        ),
        (
            "const HOST: Option<&str> = call!(option_env, \"CAMPAIGN_AMBIENT\");",
            Some(identifier("option_env")),
        ),
        ("use core::include as splice;", Some(identifier("include"))),
        (
            "use core::include_bytes as grab;",
            Some(identifier("include_bytes")),
        ),
        (
            "use core::option_env as ambient;",
            Some(identifier("option_env")),
        ),
        ("use core::{assert as check};", rename("check")),
        ("pub use crate::plan as schedule;", rename("schedule")),
        (
            "#[doc = include_str!(\"/proc/sys/kernel/random/uuid\")]\npub struct Seeded;",
            Some(identifier("include_str")),
        ),
        (
            "#[cfg_attr(test, doc = core::include_str!(\"x\"))]\npub struct Seeded;",
            Some(identifier("include_str")),
        ),
        (
            "const SEED: &str = r#env!(\"CAMPAIGN_SEED\");",
            Some(identifier("env")),
        ),
        (
            "fn f() -> u8 { matches!(0, _ if env!(\"X\").is_empty()) as u8 }",
            Some(identifier("env")),
        ),
        ("#[path = \"clock.inc\"]\nmod clock;", path.clone()),
        ("mod outer { #![path = \"clock.inc\"] }", path.clone()),
        (
            "#[cfg_attr(unix, cfg_attr(all(), path = \"clock.inc\"))]\nmod clock {}",
            path.clone(),
        ),
        (
            "#[cfg_attr(unix, allow(dead_code), r#path = \"clock.inc\")]\nmod clock {}",
            path.clone(),
        ),
        ("m! { #[path = \"clock.inc\"] mod clock; }", path),
    ];
    assert!(!cases.is_empty());
    for (content, expected) in cases {
        assert_eq!(walk(content, super::SHARED)?, expected, "{content}");
    }
    Ok(())
}

/// Every member refuses only the `include` identifier: other compile-time reads, definitions, and
/// renames are shared-crate refusals.
#[test]
// r[verify campaign.functional_core.effect]
fn refuses_only_the_include_identifier_in_any_member() -> Result<(), crate::failure::CheckError> {
    for content in [
        "include!(\"hidden.rs\");",
        "fn f() { ::std::include!(\"clock.inc\"); }",
        "macro_rules! splice { ($f:literal) => { include!($f); }; }",
        "use core::include as splice;",
        "#[doc = include!(\"x\")]\npub struct S;",
    ] {
        assert_eq!(
            walk(content, super::ANY_MEMBER)?,
            Some(identifier("include")),
            "{content}"
        );
    }
    for content in [
        "const TEXT: &str = include_str!(\"x\");",
        "fn f() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }",
        "macro_rules! call { ($m:ident) => { $m!() }; }",
        "use core::assert as check;",
        "#[path = \"clock.inc\"]\nmod clock;",
    ] {
        assert_eq!(walk(content, super::ANY_MEMBER)?, None, "{content}");
    }
    Ok(())
}
