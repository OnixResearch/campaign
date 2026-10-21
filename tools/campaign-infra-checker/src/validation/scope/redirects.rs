//! Audit9 negative overlays (D85-D88): compile-time reads and indirection reached through macro
//! bodies, macro arguments, attributes, and renames; repository Cargo configuration anywhere and
//! toolchain paths; features any member unifies into a core-allowlisted crate; compiled targets
//! outside their member; and crate roots a `#[cfg(test)]` declaration would exempt from the
//! identifier scan (previously an assertion panic). Each carries its rule and exact denial.

const EFFECT: crate::rules::Rule = crate::rules::Rule::FUNCTIONAL_CORE_EFFECT;
const HISTORY: crate::rules::Rule = crate::rules::Rule::BOUNDARY_HISTORY_LEAK;

/// Every Audit9 overlay; Cargo configuration fixtures carry only the refused file's path
/// (`super::overlays::expected_message`).
pub(crate) const REFUSALS: &[super::overlays::Refusal] = &[
    (
        "core-macro-definition",
        EFFECT,
        "crates/campaign-core/src/policy/mod.rs: macro_rules! definitions hide tokens from the \
         scan and are not admitted",
    ),
    (
        "core-read-as-macro-argument",
        EFFECT,
        "crates/campaign-core/src/policy/mod.rs: identifier `include_bytes` names a compile-time \
         file or build-environment read, which is not admitted",
    ),
    (
        "core-renamed-import",
        EFFECT,
        "crates/campaign-core/src/policy/mod.rs: `use .. as check` renames an import, which is \
         not admitted",
    ),
    (
        "core-doc-include-str",
        EFFECT,
        "crates/campaign-core/src/policy/mod.rs: identifier `include_str` names a compile-time \
         file or build-environment read, which is not admitted",
    ),
    (
        "core-path-in-macro-body",
        EFFECT,
        "crates/campaign-core/src/policy/mod.rs: a #[path] attribute is source indirection and is \
         not admitted",
    ),
    (
        "cargo-config-compiler-wrapper",
        HISTORY,
        ".cargo/config.toml",
    ),
    (
        "cargo-config-nested-directory",
        HISTORY,
        "crates/.cargo/config.toml",
    ),
    (
        "toolchain-path",
        HISTORY,
        "rust-toolchain.toml sets [toolchain] path; a repository toolchain file may not select a \
         compiler from a local directory",
    ),
    (
        "toolchain-nested-legacy-path",
        HISTORY,
        "crates/campaign-core/rust-toolchain sets [toolchain] path; a repository toolchain file \
         may not select a compiler from a local directory",
    ),
    (
        "exempt-hash-feature",
        EFFECT,
        "tools/campaign-infra-checker/Cargo.toml (campaign-infra-checker) declares blake3 without \
         inheriting its [workspace.dependencies] entry; a core-allowlisted dependency is declared \
         once, in [workspace.dependencies] with its approved feature set",
    ),
    (
        "exempt-inherited-hash-feature",
        EFFECT,
        "tools/campaign-infra-checker/Cargo.toml (campaign-infra-checker) sets features on blake3; \
         a core-allowlisted dependency is declared once, in [workspace.dependencies] with its \
         approved feature set",
    ),
    (
        "core-hash-default-features",
        EFFECT,
        "crates/campaign-core/Cargo.toml (campaign-core) sets default-features on blake3; a \
         core-allowlisted dependency is declared once, in [workspace.dependencies] with its \
         approved feature set",
    ),
    (
        "workspace-hash-default-features",
        EFFECT,
        "Cargo.toml [workspace.dependencies] blake3 declares core-allowlisted blake3 with \
         \"1.8.2\"; its approved feature set is default-features = false and no features",
    ),
    (
        "core-example-outside-member",
        EFFECT,
        "crates/campaign-core/Cargo.toml compiles test-only target \
         ../../../universal-weave/examples/topology.rs outside crates/campaign-core/src, \
         crates/campaign-core/tests, crates/campaign-core/examples, and \
         crates/campaign-core/benches",
    ),
    (
        "core-test-target-outside-member",
        EFFECT,
        "crates/campaign-core/Cargo.toml compiles test-only target \
         crates/campaign-choregraph/tests/ambient.rs outside crates/campaign-core/src, \
         crates/campaign-core/tests, crates/campaign-core/examples, and \
         crates/campaign-core/benches",
    ),
    (
        "core-bench-in-member-root",
        EFFECT,
        "crates/campaign-core/Cargo.toml compiles test-only target crates/campaign-core/speed.rs \
         outside crates/campaign-core/src, crates/campaign-core/tests, \
         crates/campaign-core/examples, and crates/campaign-core/benches",
    ),
    (
        "exempt-example-outside-member",
        EFFECT,
        "tools/campaign-infra-checker/Cargo.toml compiles test-only target \
         ../../../universal-weave/examples/topology.rs outside tools/campaign-infra-checker/src, \
         tools/campaign-infra-checker/tests, tools/campaign-infra-checker/examples, and \
         tools/campaign-infra-checker/benches",
    ),
    (
        "core-gated-library-root",
        EFFECT,
        "crates/campaign-core/src/lib.rs is a crate root that a #[cfg(test)] module declaration \
         names; the identifier scan never exempts a crate root",
    ),
    (
        "conformance-gated-binary-directory",
        EFFECT,
        "crates/campaign-conformance/src/bin/tool.rs is a crate root that a #[cfg(test)] module \
         declaration names; the identifier scan never exempts a crate root",
    ),
    (
        "exempt-binary-outside-src",
        EFFECT,
        "tools/campaign-infra-checker/Cargo.toml compiles tools/ambient.rs outside \
         tools/campaign-infra-checker/src, which the source scan does not read",
    ),
];
