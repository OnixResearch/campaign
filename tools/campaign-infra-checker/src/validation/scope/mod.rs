//! D78 regressions: Cargo configuration, lockfile sources, compiled files, dependency spellings and
//! features, and identifier-scan gaps are each refused exactly, and admitted shapes still pass.

mod admitted;
pub(super) mod overlays;
pub(super) mod redirects;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn apply(root: &std::path::Path, edits: &[admitted::Edit]) -> TestResult {
    assert!(
        !edits.is_empty(),
        "an admitted variant edits the positive root"
    );
    for edit in edits {
        match edit {
            admitted::Edit::Write(path, text) => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(full, text)?;
            }
            admitted::Edit::Append(path, text) => {
                let full = root.join(path);
                let content = std::fs::read_to_string(&full)?;
                std::fs::write(full, format!("{content}{text}"))?;
            }
            admitted::Edit::Replace(path, old, new) => {
                let full = root.join(path);
                let content = std::fs::read_to_string(&full)?;
                if !content.contains(old) {
                    return Err(format!("{path} lacks {old}").into());
                }
                std::fs::write(full, content.replacen(old, new, 1))?;
            }
        }
    }
    assert!(root.is_dir(), "edits keep the materialized root");
    Ok(())
}

#[test]
// r[verify campaign.boundary.history_leak]
// r[verify campaign.boundary.lexical_guard]
// r[verify campaign.boundary.test_code]
// r[verify campaign.functional_core.effect]
fn refuses_each_overlay_with_its_exact_denial() -> TestResult {
    for overlay in overlays::REFUSALS.iter().chain(redirects::REFUSALS) {
        let (fixture, rule, _) = *overlay;
        let root = crate::validation::fixtures::materialize(Some(fixture))?;
        let error = crate::validation::fixtures::check_boundaries(root.path()).err();
        assert_eq!(
            error.as_ref().and_then(crate::failure::CheckError::rule),
            Some(rule),
            "{fixture}: {error:?}"
        );
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some(overlays::expected_message(*overlay).as_str()),
            "{fixture}"
        );
    }
    Ok(())
}

#[test]
// r[verify campaign.boundary.history_leak]
// r[verify campaign.functional_core.effect]
fn admits_inert_configuration_pinned_sources_scanned_targets_and_gated_tests() -> TestResult {
    for (name, edits) in admitted::VARIANTS {
        let root = crate::validation::fixtures::materialize(None)?;
        apply(root.path(), edits)?;
        let refusal = crate::validation::fixtures::check_boundaries(root.path())
            .err()
            .map(|error| error.to_string());
        assert_eq!(refusal, None, "{name}");
    }
    Ok(())
}

/// `include!` under any path, rename, macro body, or attribute is refused in every member, the
/// exempt checker included (Audit9 finding 3 and D85); these cases are edits rather than committed
/// overlays because the checker's own fixture sources are scanned as member files.
#[test]
// r[verify campaign.functional_core.effect]
fn refuses_include_under_any_path_rename_or_macro_body_exactly() -> TestResult {
    let refused = |path: &str| {
        format!(
            "{path}: identifier `include` names include! source indirection, which is not admitted"
        )
    };
    let core = "crates/campaign-core/src/lib.rs";
    let checker = "tools/campaign-infra-checker/src/main.rs";
    let cases: [(&str, admitted::Edit); 5] = [
        (
            core,
            admitted::Edit::Append(core, "\ncore::include!(\"clock.inc\");\n"),
        ),
        (
            core,
            admitted::Edit::Append(
                core,
                "\nmacro_rules! splice {\n    ($file:literal) => {\n        include!($file);\n    };\n}\nsplice!(\"clock.inc\");\n",
            ),
        ),
        (
            core,
            admitted::Edit::Append(
                core,
                "\nuse core::include as splice;\nsplice!(\"clock.inc\");\n",
            ),
        ),
        (
            checker,
            admitted::Edit::Append(
                checker,
                "\n#[doc = include!(\"clock.inc\")]\nstruct Probe;\n",
            ),
        ),
        (
            "crates/campaign-profile/src/lib.rs",
            admitted::Edit::Append(
                "crates/campaign-profile/src/lib.rs",
                "\nconst SPLICE: () = call!(include, \"clock.inc\");\n",
            ),
        ),
    ];
    for (path, edit) in cases {
        let root = crate::validation::fixtures::materialize(None)?;
        apply(root.path(), core::slice::from_ref(&edit))?;
        let error = crate::validation::fixtures::check_boundaries(root.path()).err();
        assert_eq!(
            error.as_ref().and_then(crate::failure::CheckError::rule),
            Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT),
            "{path}"
        );
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some(refused(path).as_str())
        );
    }
    Ok(())
}
