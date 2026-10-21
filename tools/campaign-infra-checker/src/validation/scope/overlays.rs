//! D78 negative overlays: build configuration, lockfile sources, compiled files, and scan gaps the
//! boundary check must read, each with its rule and exact denial.

/// A negative overlay, the rule it violates, and the exact denial message.
pub(crate) type Refusal = (&'static str, crate::rules::Rule, &'static str);

const REDIRECTED: &str = " is repository Cargo configuration, which may redirect dependency \
                          sources, compiler flags, or the compiler; none is admitted";

/// Every D78 overlay. Messages are asserted exactly by `validation::scope`.
pub(crate) const REFUSALS: &[Refusal] = &[
    (
        "cargo-config-paths",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ".cargo/config.toml",
    ),
    (
        "cargo-config-patch",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ".cargo/config",
    ),
    (
        "cargo-config-source",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ".cargo/config.toml",
    ),
    (
        "cargo-config-rustflags",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/.cargo/config.toml",
    ),
    (
        "lock-ambient-path-package",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "Cargo.lock resolves universal-weave from a path outside the workspace members; only \
         registry releases, workspace members, and the pinned Choregraph revision are admitted",
    ),
    (
        "lock-unpinned-git-package",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "Cargo.lock resolves weave-topology from source \
         git+https://example.invalid/universal-weave.git?rev=0809c9f10ba48b8ae0192ab3a29ff4c5b335e56e#0809c9f10ba48b8ae0192ab3a29ff4c5b335e56e; \
         only registry releases, workspace members, and the pinned Choregraph revision are admitted",
    ),
    (
        "core-library-outside-src",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/Cargo.toml compiles crates/campaign-core/lib.rs outside \
         crates/campaign-core/src, which the source scan does not read",
    ),
    (
        "core-build-script-clock",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/build.rs: ambient effect `std` facade \
         (std::fs/env/net/process/time/thread/sync/io)",
    ),
    (
        "core-path-module",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/src/lib.rs: a #[path] attribute is source indirection and is not \
         admitted",
    ),
    (
        "core-compile-time-read",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/src/policy/mod.rs: identifier `include_bytes` names a compile-time \
         file or build-environment read, which is not admitted",
    ),
    (
        "core-build-environment",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/src/policy/mod.rs: identifier `option_env` names a compile-time \
         file or build-environment read, which is not admitted",
    ),
    (
        "core-test-gated-sibling",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/src/policy/mod.rs: identifier `UniversalWeave` belongs to the \
         Choregraph history or Universal Weave identifier family",
    ),
    (
        "core-ungated-tests-module",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/src/tests.rs: identifier `UniversalWeaveTopology` belongs to the \
         Choregraph history or Universal Weave identifier family",
    ),
    (
        "core-twin-declared-tests-module",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/src/tests.rs: identifier `UniversalWeaveTopology` belongs to the \
         Choregraph history or Universal Weave identifier family",
    ),
    (
        "core-never-compiled-test-twin",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/src/topology.rs: identifier `UniversalWeaveTopology` belongs to the \
         Choregraph history or Universal Weave identifier family",
    ),
    (
        "conformance-universal-weave",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-conformance/src/lib.rs: identifier `UniversalWeaveTopology` belongs to \
         the Universal Weave identifier family",
    ),
    (
        "exempt-underscore-random-dependency",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "tools/campaign-infra-checker/Cargo.toml (campaign-infra-checker) declares dependency \
         rand_core refused by its dependency policy",
    ),
    (
        "core-underscore-build-dependency",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/Cargo.toml (campaign-core) declares dependency rand refused by its \
         dependency policy",
    ),
    (
        "core-hash-feature",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/Cargo.toml (campaign-core) enables non-default dependency features \
         through dependency blake3 with features = [\"mmap\"]; its dependencies keep their \
         default features",
    ),
    (
        "core-build-script-module",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/build.rs: the build script declares file module helper, whose \
         source lies outside src",
    ),
    (
        "core-build-path-outside-src",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/Cargo.toml compiles tools/build.rs outside crates/campaign-core/src, \
         which the source scan does not read",
    ),
    (
        "conformance-binary-outside-src",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-conformance/Cargo.toml compiles crates/campaign-conformance/tool.rs \
         outside crates/campaign-conformance/src, which the source scan does not read",
    ),
    (
        "core-library-without-no-std",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/src/entry.rs must declare #![no_std]",
    ),
    (
        "core-inherited-hash-feature",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/Cargo.toml (campaign-core) enables non-default dependency features \
         through dependency blake3 with features = [\"mmap\"]; its dependencies keep their \
         default features",
    ),
    (
        "core-feature-table-dependency-feature",
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        "crates/campaign-core/Cargo.toml (campaign-core) enables non-default dependency features \
         through [features] fast = [\"blake3/mmap\"]; its dependencies keep their default \
         features",
    ),
    (
        "core-underscore-target-build-table",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/Cargo.toml (campaign-core) declares dependency universal-weave \
         refused by its dependency policy",
    ),
    (
        "exempt-underscore-weave-dependency",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "tools/campaign-infra-checker/Cargo.toml (campaign-infra-checker) declares dependency \
         universal_weave refused by its dependency policy",
    ),
    (
        "cargo-config-rustdocflags",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        ".cargo/config.toml",
    ),
    (
        "core-test-gated-field",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "crates/campaign-core/src/policy/mod.rs: identifier `universal_weave` belongs to the \
         Choregraph history or Universal Weave identifier family",
    ),
    (
        "core-ungated-tests-directory",
        crate::rules::Rule::CONSUMER_PILOTS_LEAK,
        "crates/campaign-core/src/policy/tests/mod.rs: identifier `ChaosControlProbe` belongs to \
         the consumer product or VM identifier family",
    ),
    (
        "lock-alternate-registry",
        crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        "Cargo.lock resolves serde from source registry+https://example.invalid/index; only \
         registry releases, workspace members, and the pinned Choregraph revision are admitted",
    ),
];

/// The exact denial of an overlay: Cargo configuration refusals name the file and share the
/// redirect explanation.
pub(crate) fn expected_message((fixture, _, message): Refusal) -> String {
    if fixture.starts_with("cargo-config-") {
        format!("{message}{REDIRECTED}")
    } else {
        message.to_owned()
    }
}
