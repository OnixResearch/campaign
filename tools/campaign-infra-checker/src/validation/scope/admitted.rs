//! D78 and Audit9 admitted shapes beside each refusal: build configuration only inside the
//! checker's fixture roots and channel-only toolchain files, crates.io (Git and sparse index) and
//! pinned-revision lock sources, explicit targets under `src/` and test-only targets under the
//! member's test directories, an effect-free build script, gated variants, fields and generic
//! items, declared nested test modules, the approved feature set of the allowlisted hash in either
//! spelling, and underscore names outside every family.

const CORE_MANIFEST: &str = "crates/campaign-core/Cargo.toml";
const CORE_POLICY: &str = "crates/campaign-core/src/policy/mod.rs";
const CONFORMANCE_MANIFEST: &str = "crates/campaign-conformance/Cargo.toml";
const CHECKER_MANIFEST: &str = "tools/campaign-infra-checker/Cargo.toml";

/// One overlay edit of a materialized positive root.
pub(super) enum Edit {
    Write(&'static str, &'static str),
    Append(&'static str, &'static str),
    Replace(&'static str, &'static str, &'static str),
}

/// Named edits of the positive root that boundary validation must still admit.
pub(super) const VARIANTS: &[(&str, &[Edit])] = &[
    (
        "configuration in the checker's fixture roots and channel-only toolchain files",
        &[
            Edit::Write(
                "tools/campaign-infra-checker/fixtures/negative/probe/.cargo/config.toml",
                "[build]\nrustc-wrapper = \"../ambient.sh\"\n",
            ),
            Edit::Write(
                "tools/campaign-infra-checker/fixtures/negative/probe/rust-toolchain.toml",
                "[toolchain]\npath = \"/opt/ambient\"\n",
            ),
            Edit::Write(
                "rust-toolchain.toml",
                "[toolchain]\nchannel = \"1.96.0\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
            ),
            Edit::Write("crates/campaign-core/rust-toolchain", "1.96.0\n"),
        ],
    ),
    (
        "registry and pinned lock sources",
        &[Edit::Append(
            "Cargo.lock",
            "\n[[package]]\nname = \"serde\"\nversion = \"1.0.229\"\nsource = \"sparse+https://index.crates.io/\"\n\n[[package]]\nname = \"history-codec\"\nversion = \"0.1.0\"\nsource = \"git+https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git?rev=b3e08e19750f53bdbcae970cdf58a47a791ed20b#b3e08e19750f53bdbcae970cdf58a47a791ed20b\"\n",
        )],
    ),
    (
        "explicit targets under src",
        &[
            Edit::Replace(
                CORE_MANIFEST,
                "name = \"campaign-core\"\n",
                "name = \"campaign-core\"\nbuild = false\n",
            ),
            Edit::Append(CORE_MANIFEST, "\n[lib]\npath = \"src/lib.rs\"\n"),
            Edit::Append(
                CONFORMANCE_MANIFEST,
                "\n[[bin]]\nname = \"conformance-tool\"\npath = \"src/bin/tool.rs\"\n",
            ),
            Edit::Write(
                "crates/campaign-conformance/src/bin/tool.rs",
                "//! Tool.\nfn main() {}\n",
            ),
            Edit::Append(
                CORE_MANIFEST,
                "\n[[example]]\nname = \"topology\"\npath = \"examples/topology.rs\"\n\n[[test]]\nname = \"ambient\"\npath = \"tests/ambient.rs\"\n\n[[bench]]\nname = \"speed\"\npath = \"benches/speed.rs\"\n\n[[test]]\nname = \"unit\"\npath = \"src/unit.rs\"\n",
            ),
            Edit::Append(
                CHECKER_MANIFEST,
                "\n[[bin]]\nname = \"campaign-infra-checker\"\npath = \"src/main.rs\"\n\n[[test]]\nname = \"cli\"\npath = \"tests/cli.rs\"\n",
            ),
        ],
    ),
    (
        "effect-free build script",
        &[Edit::Write(
            "crates/campaign-core/build.rs",
            "//! Build script without effects.\nfn main() {}\n",
        )],
    ),
    (
        "gated variants, fields, and generic items",
        &[Edit::Append(
            CORE_POLICY,
            "\n/// Probe kinds.\npub(crate) enum Probe {\n    #[cfg(test)]\n    MergeProbe(u8),\n    Neutral,\n    #[cfg(test)]\n    ReplayCursor { cursor: u8 },\n}\n/// A pair.\npub(crate) struct Pair<A, B> {\n    #[cfg(test)]\n    pub(crate) merge_left: A,\n    pub(crate) right: B,\n}\n#[cfg(test)]\nimpl<A, B> Pair<A, B>\nwhere\n    A: Copy,\n    B: Copy,\n{\n    fn merge_fixture(self) -> Self {\n        self\n    }\n}\n",
        )],
    ),
    (
        "declared nested test modules",
        &[
            Edit::Append("crates/campaign-core/src/tests.rs", "\nmod fixtures;\n"),
            Edit::Write(
                "crates/campaign-core/src/tests/fixtures.rs",
                "pub(crate) struct UniversalWeaveTopology;\n",
            ),
            Edit::Append(
                CORE_POLICY,
                "\n#[cfg(test)]\nmod probes {\n    mod weave;\n}\n",
            ),
            Edit::Write(
                "crates/campaign-core/src/policy/probes/weave.rs",
                "pub(crate) struct UniversalWeave;\n",
            ),
        ],
    ),
    (
        "default features and names outside every family",
        &[
            Edit::Replace(
                "Cargo.toml",
                "blake3 = { version = \"1.8.2\", default-features = false }",
                "blake3 = { version = \"1.8.2\", default_features = false, features = [] }",
            ),
            Edit::Replace(
                CORE_MANIFEST,
                "blake3.workspace = true",
                "blake3 = { workspace = true }",
            ),
            Edit::Replace(
                CORE_MANIFEST,
                "proptest = \"1\"",
                "proptest = { version = \"1\", features = [\"std\"] }",
            ),
            Edit::Append(
                CHECKER_MANIFEST,
                "randomizer = \"0.1\"\nweaver_tools = \"0.1\"\n",
            ),
        ],
    ),
];
