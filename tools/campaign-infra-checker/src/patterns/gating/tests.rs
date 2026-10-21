//! Only files declared under `#[cfg(test)]` are test modules; build-script file modules are
//! refused.

const LIB: &str = "crates/c/src/lib.rs";
const BUILD: &str = "crates/c/build.rs";

/// A source file from `(display path, content)`.
fn source((display_path, content): (&str, &str)) -> crate::model::SourceFile {
    crate::model::SourceFile {
        display_path: display_path.to_owned(),
        content: content.to_owned(),
    }
}

fn modules(
    files: &[crate::model::SourceFile],
) -> Result<super::TestModules, crate::failure::CheckError> {
    let roots = [LIB.to_owned()];
    let borrowed: Vec<&crate::model::SourceFile> = files.iter().collect();
    super::collect(
        &borrowed,
        super::Roots {
            files: &roots,
            binary_directory: "crates/c/src/bin",
            build_script: Some(BUILD),
        },
    )
}

#[test]
// r[verify campaign.functional_core.effect]
fn only_files_declared_under_cfg_test_are_test_modules() -> Result<(), crate::failure::CheckError> {
    let files = [
        source((
            LIB,
            "#[cfg(test)] mod tests; mod policy; mod store; pub mod helpers;",
        )),
        source((
            "crates/c/src/policy/mod.rs",
            "#[cfg(test)] mod tests; #[cfg(test)] mod inline { mod deep; } mod open { #[cfg(test)] mod checks; } mod r#async;",
        )),
        source((
            "crates/c/src/store.rs",
            "#[cfg(test)] mod tests; pub mod tests_support;",
        )),
        source(("crates/c/src/bin/tool.rs", "#[cfg(test)] mod tests;")),
    ];
    let found = modules(&files)?;
    for gated in [
        "crates/c/src/tests.rs",
        "crates/c/src/tests/fixtures.rs",
        "crates/c/src/policy/tests/mod.rs",
        "crates/c/src/policy/inline/deep.rs",
        "crates/c/src/policy/open/checks.rs",
        "crates/c/src/store/tests.rs",
        "crates/c/src/bin/tests.rs",
    ] {
        assert!(found.contains(gated), "{gated} is not a test module");
    }
    for production in [
        LIB,
        "crates/c/src/helpers.rs",
        "crates/c/src/testsuite.rs",
        "crates/c/src/policy/mod.rs",
        "crates/c/src/policy/async.rs",
        "crates/c/src/store/tests_support.rs",
        "crates/c/src/policy/tests_support.rs",
    ] {
        assert!(!found.contains(production), "{production} is a test module");
    }
    Ok(())
}

#[test]
// r[verify campaign.functional_core.effect]
fn refuses_build_script_file_modules() {
    let cases = [(
        source((BUILD, "mod helper;\nfn main() {}")),
        "crates/c/build.rs: the build script declares file module helper, whose source lies outside src",
    )];
    for (file, expected) in cases {
        let error = modules(core::slice::from_ref(&file)).err();
        assert_eq!(
            error.as_ref().and_then(crate::failure::CheckError::rule),
            Some(crate::rules::Rule::FUNCTIONAL_CORE_EFFECT),
            "{expected}"
        );
        assert_eq!(
            error.as_ref().map(crate::failure::CheckError::message),
            Some(expected)
        );
    }
}

/// Audit10 (D93): a declaration exempts its module only when it carries exactly `#[cfg(test)]` and
/// no other `cfg`, and a module any other declaration names without that gate (a twin declaration,
/// file or inline) stays production code, with every file below it.
#[test]
// r[verify campaign.boundary.test_code]
fn a_module_is_test_only_when_every_declaration_naming_it_is_exactly_cfg_test()
-> Result<(), crate::failure::CheckError> {
    let files = [
        source((
            LIB,
            "#[cfg(test)] mod tests; #[cfg(not(test))] pub mod tests;\n\
             pub mod topology; #[cfg(test)] #[cfg(not(test))] pub mod topology;\n\
             #[cfg(test)] #[cfg(feature = \"probe\")] mod probe;\n\
             #[cfg(test)] #[cfg_attr(test, cfg(any()))] mod shadow;\n\
             #[cfg(test)] mod twin; #[cfg(not(test))] mod twin { mod leaf; }\n\
             #[cfg(test)] #[cfg(any())] mod dead { mod inner; }\n\
             /// Checks.\n#[cfg(test)] mod checks;",
        )),
        source(("crates/c/src/checks.rs", "mod fixtures;")),
    ];
    let found = modules(&files)?;
    for gated in ["crates/c/src/checks.rs", "crates/c/src/checks/fixtures.rs"] {
        assert!(found.contains(gated), "{gated} is not a test module");
    }
    for production in [
        "crates/c/src/tests.rs",
        "crates/c/src/tests/fixtures.rs",
        "crates/c/src/topology.rs",
        "crates/c/src/probe.rs",
        "crates/c/src/shadow/mod.rs",
        "crates/c/src/twin.rs",
        "crates/c/src/twin/leaf.rs",
        "crates/c/src/dead/inner.rs",
    ] {
        assert!(!found.contains(production), "{production} is a test module");
    }
    Ok(())
}
