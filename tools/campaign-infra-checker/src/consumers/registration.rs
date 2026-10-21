//! The pinned suite constants back a consumer record only while a test that reproduces them runs.
//!
//! The chain is read from the conformance member itself: its library target keeps the test harness,
//! its crate root declares `#[cfg(test)] mod tests;`, the `tests` module declares `mod consumers;`,
//! none of the library root, the tests module, and the pinned module is configured away by an
//! inner `#![cfg]`, and the pinned module holds a top-level `#[test]` that is neither ignored,
//! expected to panic, nor configured away, names all three pinned constants in its body, and calls
//! `run_suite` in its body or through a same-module function its body calls (D93). Otherwise
//! constants no run verifies could back a stale record (`campaign.consumer_pilots.valid`).

const LIBRARY_TABLE: &str = "lib";
const HARNESS_KEYS: &[&str] = &["test", "harness"];
const TESTS_MODULE: &str = "tests";
const CONSUMERS_MODULE: &str = "consumers";
const TESTS_MODULE_FILES: &[&str] = &["tests/mod.rs", "tests.rs"];
const CONFIGURATION_ATTRIBUTES: &[&str] = &["cfg"];
const DOC_ATTRIBUTE: &str = "doc";

type Admitted<T> = super::fields::Admitted<T>;

/// Refuse a pinned suite whose test is unregistered, disarmed, or neither names the constants nor
/// calls the suite run.
// r[impl campaign.consumer_pilots.valid]
pub(super) fn validate(members: &[crate::model::Member]) -> Admitted<()> {
    let suite = crate::policy::CONSUMER_PINNED_SUITE;
    let member = members
        .iter()
        .find(|member| member.path.eq(crate::policy::CONFORMANCE_MEMBER))
        .ok_or_else(|| {
            never_runs(&format!(
                "{} is not a member",
                crate::policy::CONFORMANCE_MEMBER
            ))
        })?;
    let manifest_path = format!("{}/{}", member.path, crate::policy::ROOT_MANIFEST);
    let manifest = crate::dependencies::parse_manifest(crate::model::ManifestText {
        relative_path: &manifest_path,
        content: &member.manifest,
    })?;
    if let Some(key) = disabled_harness(&manifest) {
        return Err(never_runs(&format!(
            "{manifest_path} sets [{LIBRARY_TABLE}] {key} = false"
        )));
    }
    let library = crate::rustscope::targets::derive(crate::rustscope::targets::MemberManifest {
        member_path: &member.path,
        manifest_path: &manifest_path,
        manifest: &manifest,
    })?
    .library;
    assert!(
        library.starts_with(&member.path),
        "the library root lies in its member"
    );
    let library_syntax = syntax(member, &library)?;
    let (tests, tests_syntax) = tests_module(member, &library, &library_syntax)?;
    let pinned = syntax(member, suite)?;
    for (path, module) in [
        (library.as_str(), &library_syntax),
        (tests.as_str(), &tests_syntax),
        (suite, &pinned),
    ] {
        if module
            .attrs
            .iter()
            .any(|attribute| applies(attribute, CONFIGURATION_ATTRIBUTES))
        {
            return Err(never_runs(&format!(
                "{path} is configured by an inner #![cfg]"
            )));
        }
    }
    assert!(
        pinned
            .attrs
            .iter()
            .all(|attribute| !applies(attribute, CONFIGURATION_ATTRIBUTES)),
        "the pinned module is compiled whenever the tests module is"
    );
    require_running_test(&pinned)
}

/// The `tests` module the library root declares under `#[cfg(test)]`, which must own the pinned
/// suite's directory and declare `mod consumers;`.
fn tests_module(
    member: &crate::model::Member,
    library: &str,
    library_syntax: &syn::File,
) -> Admitted<(String, syn::File)> {
    let suite = crate::policy::CONSUMER_PINNED_SUITE;
    let directory = library.rsplit_once('/').map_or("", |(parent, _)| parent);
    if !declares(library_syntax, TESTS_MODULE, true) {
        return Err(never_runs(&format!(
            "{library} does not declare `#[cfg(test)] mod {TESTS_MODULE};`"
        )));
    }
    let tests_directory = format!("{directory}/{TESTS_MODULE}");
    if !crate::syntax::is_contained(crate::syntax::Containment {
        path: suite,
        ancestor: &tests_directory,
    }) {
        return Err(never_runs(&format!(
            "{library} is the library root, so `mod {TESTS_MODULE}` resolves under {tests_directory}"
        )));
    }
    let tests = TESTS_MODULE_FILES
        .iter()
        .map(|file| format!("{directory}/{file}"))
        .find(|path| file(member, path).is_some())
        .ok_or_else(|| never_runs(&format!("{tests_directory}/mod.rs is absent")))?;
    let tests_syntax = syntax(member, &tests)?;
    if !declares(&tests_syntax, CONSUMERS_MODULE, false) {
        return Err(never_runs(&format!(
            "{tests} does not declare `mod {CONSUMERS_MODULE};`"
        )));
    }
    assert!(
        tests.starts_with(directory),
        "the tests module lies beside its crate root"
    );
    assert!(
        suite.starts_with(&tests_directory),
        "the tests module owns the pinned suite's directory"
    );
    Ok((tests, tests_syntax))
}

/// `[lib] test = false` or `harness = false`, either of which stops `#[test]` functions running.
fn disabled_harness(manifest: &toml::Table) -> Option<&'static str> {
    let library = manifest.get(LIBRARY_TABLE)?;
    HARNESS_KEYS
        .iter()
        .find(|key| library.get(**key).and_then(toml::Value::as_bool) == Some(false))
        .copied()
}

fn file<'member>(
    member: &'member crate::model::Member,
    path: &str,
) -> Option<&'member crate::model::SourceFile> {
    member.files.iter().find(|file| file.display_path.eq(path))
}

fn syntax(member: &crate::model::Member, path: &str) -> Admitted<syn::File> {
    let source = file(member, path).ok_or_else(|| never_runs(&format!("{path} is absent")))?;
    syn::parse_file(&source.content).map_err(|error| {
        super::fields::violation(super::VALID, format!("{path} is not valid Rust: {error}"))
    })
}

/// A top-level file module `name` whose attributes are doc comments and, exactly when
/// `is_test_gated`, one `#[cfg(test)]`.
fn declares(file: &syn::File, name: &str, is_test_gated: bool) -> bool {
    file.items.iter().any(|item| {
        let syn::Item::Mod(module) = item else {
            return false;
        };
        let gates = module
            .attrs
            .iter()
            .filter(|attribute| is_cfg_test(attribute))
            .count();
        let is_plain = module.attrs.iter().all(|attribute| {
            attribute.path().is_ident(DOC_ATTRIBUTE) || (is_test_gated && is_cfg_test(attribute))
        });
        module.ident.eq(name)
            && module.content.is_none()
            && is_plain
            && gates.eq(&usize::from(is_test_gated))
    })
}

fn is_cfg_test(attribute: &syn::Attribute) -> bool {
    matches!(attribute.style, syn::AttrStyle::Outer)
        && matches!(
            &attribute.meta,
            syn::Meta::List(list) if list.path.is_ident("cfg") && list.tokens.to_string().as_str().eq("test")
        )
}

fn applies(attribute: &syn::Attribute, names: &[&str]) -> bool {
    crate::rustscope::applies_attribute(&attribute.meta, names)
}

/// One top-level armed `#[test]` names every pinned constant in its body and calls `run_suite` in
/// its body or through a same-module function its body calls.
fn require_running_test(pinned: &syn::File) -> Admitted<()> {
    let suite = crate::policy::CONSUMER_PINNED_SUITE;
    let armed = super::calls::armed_tests(pinned);
    if armed.is_empty() {
        return Err(never_runs(&format!(
            "{suite} holds no top-level #[test] without ignore, should_panic, or cfg attributes \
             that names {}",
            super::calls::PINNED_CONSTANTS.join(", ")
        )));
    }
    assert!(
        armed.len() <= pinned.items.len(),
        "every armed test is a top-level item"
    );
    if !armed
        .iter()
        .any(|test| super::calls::reaches_suite(pinned, test))
    {
        return Err(never_runs(&format!(
            "{suite} holds no top-level armed #[test] naming the pinned constants that calls \
             {} in its body or through a same-module function its body calls, so no suite run \
             reproduces its constants",
            super::calls::SUITE_ENTRY
        )));
    }
    assert!(
        !super::calls::PINNED_CONSTANTS.is_empty(),
        "a running test names the pinned constants"
    );
    Ok(())
}

fn never_runs(reason: &str) -> crate::failure::CheckError {
    super::fields::violation(
        super::VALID,
        format!(
            "{reason}; the pinned consumer suite test in {} never runs",
            crate::policy::CONSUMER_PINNED_SUITE
        ),
    )
}
