//! An outer `#[cfg(test)]` exempts exactly one member or item from identifier families.

const CORE_PATH: &str = "crates/campaign-core/src/policy/mod.rs";

fn core_rule(content: &str) -> Result<Option<crate::rules::Rule>, crate::failure::CheckError> {
    let profile = crate::policy::scope::for_package(crate::policy::CORE_PACKAGE)
        .and_then(|package| package.sources.as_ref())
        .ok_or_else(|| crate::failure::CheckError::failed("core has no source profile"))?;
    let source = crate::model::SourceFile {
        display_path: CORE_PATH.to_owned(),
        content: content.to_owned(),
    };
    Ok(crate::patterns::validate_file(&source, profile, false)
        .err()
        .and_then(|error| error.rule()))
}

#[test]
// r[verify campaign.boundary.history_leak]
fn a_gate_ends_at_its_member_comma_or_its_item_body() -> Result<(), crate::failure::CheckError> {
    let history = Some(crate::rules::Rule::BOUNDARY_HISTORY_LEAK);
    let cases = [
        ("pub enum T { #[cfg(test)] Probe, UniversalWeave }", history),
        (
            "pub enum T { #[cfg(test)] Probe(u8), UniversalWeave }",
            history,
        ),
        (
            "pub struct P { #[cfg(test)] pub merge: u8, pub universal: u8 }",
            history,
        ),
        (
            "fn f(x: u8) { match x { #[cfg(test)] 0 => merge(), _ => weave() } }",
            history,
        ),
        (
            "#[cfg(test)] fn merge_fixture() {} pub fn merge() {}",
            Some(crate::rules::Rule::REPLAY_BOUNDARY_INVALID),
        ),
        ("pub enum K { #[cfg(test)] MergeProbe, Neutral }", None),
        (
            "pub enum K { #[cfg(test)] MergeProbe { at: u8 }, Neutral }",
            None,
        ),
        (
            "#[cfg(test)] impl<A, B> P<A, B> where A: Copy, B: Copy { fn merge_fixture() {} }",
            None,
        ),
        (
            "#[cfg(test)] pub(crate) fn merge_pair<A, B>(a: A, b: B) {}",
            None,
        ),
        (
            "#[cfg(test)] #[derive(Debug)] struct MergeProbe<A, B>(A, B);",
            None,
        ),
        (
            "fn f() { #[cfg(test)] let merged: Map<u8, MergeKey> = x; }",
            None,
        ),
    ];
    assert!(cases.iter().any(|(_, expected)| expected.is_none()));
    for (content, expected) in cases {
        assert_eq!(core_rule(content)?, expected, "{content}");
    }
    Ok(())
}
