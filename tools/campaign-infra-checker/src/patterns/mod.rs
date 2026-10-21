//! Token-level source-pattern scans for shared Campaign crates.
//!
//! Compile-time reads and source indirection are refused over every token tree, attributes and
//! macro bodies included (`indirection`). The effect and family scans read identifiers and
//! punctuation only; comments, doc text, string literals, attributes, and lifetimes never match.
//! Ambient effects are rejected everywhere, including tests. Identifier families skip files
//! declared as `#[cfg(test)]` modules (`gating`) and `#[cfg(test)]` items. This is syntactic
//! evidence, not a proof.

mod effects;
pub(crate) mod gating;
pub(crate) mod indirection;
mod tokens;
mod words;

/// Scan one compiled source; `is_test_module` marks a file declared as a `#[cfg(test)]` module.
pub(crate) fn validate_file(
    source: &crate::model::SourceFile,
    profile: &crate::policy::scope::SourceProfile,
    is_test_module: bool,
) -> Result<(), crate::failure::CheckError> {
    assert!(
        !source.display_path.is_empty(),
        "a scanned source names its file"
    );
    if let Some(found) = indirection::find_in(source, indirection::SHARED)? {
        return Err(crate::failure::CheckError::violation(
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
            format!("{}: {}", source.display_path, found.describe()),
        ));
    }
    let scanned = tokens::scan(source, is_test_module)?;
    assert!(
        scanned
            .iter()
            .all(|token| token.is_test_only || !is_test_module),
        "every token of a declared test module is test-only"
    );
    if let Some(effect) = effects::find(&scanned) {
        return Err(crate::failure::CheckError::violation(
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
            format!("{}: ambient effect {effect}", source.display_path),
        ));
    }
    let Some((family, identifier)) = first_family_match(&scanned, profile) else {
        return Ok(());
    };
    Err(crate::failure::CheckError::violation(
        family.rule,
        format!(
            "{}: identifier `{identifier}` belongs to the {} family",
            source.display_path, family.name
        ),
    ))
}

/// The first production identifier that belongs to one of the profile's families.
fn first_family_match<'scan>(
    scanned: &'scan [tokens::Scanned],
    profile: &crate::policy::scope::SourceProfile,
) -> Option<(&'static crate::policy::scope::Family, &'scan str)> {
    let families: &'static [crate::policy::scope::Family] = profile.families;
    scanned
        .iter()
        .filter_map(tokens::Scanned::production_ident)
        .find_map(|identifier| {
            families
                .iter()
                .find(|family| is_family_match(&family.matcher, identifier))
                .map(|family| (family, identifier))
        })
}

/// The core library root must declare `#![no_std]` without a configuration predicate.
// r[impl campaign.functional_core]
pub(crate) fn require_no_std(
    library_root: Option<&crate::model::SourceFile>,
    expected_path: &str,
) -> Result<(), crate::failure::CheckError> {
    let missing = || {
        crate::failure::CheckError::violation(
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
            format!("{expected_path} must declare #![no_std]"),
        )
    };
    let source = library_root.ok_or_else(missing)?;
    let syntax = syn::parse_file(&source.content).map_err(|error| {
        crate::failure::CheckError::failed(format!("invalid Rust in {expected_path}: {error}"))
    })?;
    let is_freestanding = syntax.attrs.iter().any(|attribute| {
        matches!(attribute.style, syn::AttrStyle::Inner(_))
            && matches!(&attribute.meta, syn::Meta::Path(path) if path.is_ident("no_std"))
    });
    is_freestanding.then_some(()).ok_or_else(missing)
}

/// Whether one identifier belongs to a family.
pub(crate) fn is_family_match(matcher: &crate::policy::scope::Matcher, identifier: &str) -> bool {
    match matcher {
        crate::policy::scope::Matcher::Words(expected) => words::split(identifier)
            .iter()
            .any(|word| expected.contains(&word.as_str())),
        crate::policy::scope::Matcher::WordPrefixes(prefixes) => words::split(identifier)
            .iter()
            .any(|word| prefixes.iter().any(|prefix| word.starts_with(prefix))),
        crate::policy::scope::Matcher::Joined(fragments) => {
            let joined = words::joined(identifier);
            fragments.iter().any(|fragment| joined.contains(fragment))
        }
        crate::policy::scope::Matcher::WordPair(first, second) => {
            let found = words::split(identifier);
            let has_word = |list: &[&str]| found.iter().any(|word| list.contains(&word.as_str()));
            has_word(first) && has_word(second)
        }
    }
}

#[cfg(test)]
mod tests;
