//! Campaign infrastructure policy facts checked by this tool.

pub(crate) mod families;
pub(crate) mod scope;

pub(crate) const SUCCESS_EXIT: i32 = 0;
pub(crate) const FAILURE_EXIT: i32 = 1;
pub(crate) const USAGE_EXIT: i32 = 2;
pub(crate) const EXPECTED_ARG_COUNT: usize = 4;
pub(crate) const COMMAND_ARG_INDEX: usize = 1;
pub(crate) const ROOT_FLAG_ARG_INDEX: usize = 2;
pub(crate) const ROOT_VALUE_ARG_INDEX: usize = 3;
pub(crate) const MAX_SCAN_ENTRIES: usize = 4_096;
pub(crate) const MAX_SOURCE_BYTES: u64 = 1_048_576;
pub(crate) const OVERSIZE_PROBE_BYTES: u64 = 1;
pub(crate) const SOURCE_READ_LIMIT: u64 = MAX_SOURCE_BYTES.saturating_add(OVERSIZE_PROBE_BYTES);
const _: () = assert!(SOURCE_READ_LIMIT > MAX_SOURCE_BYTES);
pub(crate) const MAX_DIRECTORY_ENTRIES: usize = 512;
pub(crate) const VISITED_ENTRY_STEP: usize = 1;
pub(crate) const MAX_TOKEN_NESTING: usize = 256;

pub(crate) const COMMAND_ALL: &str = "all";
pub(crate) const COMMAND_BOUNDARIES: &str = "boundaries";
pub(crate) const COMMAND_CAIRN_POLICY_SOURCE: &str = "cairn-policy-source";
pub(crate) const ROOT_FLAG: &str = "--root";
pub(crate) const USAGE_TEXT: &str =
    "usage: campaign-infra-checker <all|boundaries|cairn-policy-source> --root <path>";
pub(crate) const RS_EXTENSION: &str = "rs";

pub(crate) const CANONICAL_CAMPAIGN_URL: &str =
    "https://git.onix.computer/z2scC9MCm3pxk9mX4FEidRKabQ5LN.git";
pub(crate) const OCTET_REVISION: &str = "890f424cd7b1b2015f39c0eb5747f1bd1e576028";
pub(crate) const CAIRN_REVISION: &str = "5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b";
pub(crate) const CAIRN_SOURCE_ROOT_ENV: &str = "CAIRN_POLICY_SOURCE_ROOT";
pub(crate) const CHOREGRAPH_HISTORY_SOURCE_ENV: &str = "CHOREGRAPH_HISTORY_SOURCE";
pub(crate) const CHOREGRAPH_HISTORY_MANIFEST: &str = "crates/choregraph-history/Cargo.toml";

pub(crate) const ROOT_MANIFEST: &str = "Cargo.toml";
pub(crate) const FLAKE_NIX: &str = "flake.nix";
pub(crate) const REPOSITORY_NCL: &str = "config/repository.ncl";
pub(crate) const GENERATED_REPOSITORY_JSON: &str = "config/generated/repository.json";
pub(crate) const GENERATED_DEFAULT_PROFILE_JSON: &str = "config/generated/default-profile.json";
/// Crate-local copy of the default profile that `campaign-profile` embeds; it must equal the checked projection.
pub(crate) const PROFILE_CRATE_DEFAULT_JSON: &str = "crates/campaign-profile/profiles/default.json";
pub(crate) const CORE_MANIFEST: &str = "crates/campaign-core/Cargo.toml";
pub(crate) const ADAPTER_MANIFEST: &str = "crates/campaign-choregraph/Cargo.toml";
pub(crate) const PRE_COMMIT_CONFIG: &str = ".pre-commit-config.yaml";
pub(crate) const DYLINT_CONFIG: &str = "dylint.toml";
pub(crate) const CAIRN_POLICY_MANIFEST: &str = "cairn-policy/source-manifest.ncl";
pub(crate) const CARGO_LOCK: &str = "Cargo.lock";
/// Cargo configuration files Cargo reads from a directory it runs in or below.
pub(crate) const CARGO_CONFIGURATION_FILES: &[&str] = &[".cargo/config.toml", ".cargo/config"];
/// Toolchain files rustup reads from a directory it runs in or below.
pub(crate) const TOOLCHAIN_FILES: &[&str] = &["rust-toolchain.toml", "rust-toolchain"];
/// The checker's own fixture roots, whose negative overlays hold refused configuration on purpose.
pub(crate) const CHECKER_FIXTURE_ROOT: &str = "tools/campaign-infra-checker/fixtures";
/// Directories the repository-wide build-configuration walk may visit.
pub(crate) const MAX_REPOSITORY_DIRECTORIES: usize = 131_072;
pub(crate) const FLAKE_LOCK: &str = "flake.lock";
pub(crate) const CHOREGRAPH_PUBLICATION_RECORD: &str = "provenance/CHOREGRAPH-PUBLICATION.md";
pub(crate) const SOURCE_INVENTORY_JSON: &str = "provenance/generated/sources.json";
pub(crate) const CONSUMER_RECORD_JSON: &str = "provenance/generated/consumers.json";
/// The conformance member, whose unit tests must run the pinned consumer suite test.
pub(crate) const CONFORMANCE_MEMBER: &str = "crates/campaign-conformance";
/// The campaign-conformance test module whose top-level constants pin the suite evidence every
/// recorded consumer pilot must claim.
pub(crate) const CONSUMER_PINNED_SUITE: &str =
    "crates/campaign-conformance/src/tests/consumers/mod.rs";
pub(crate) const PINNED_FIXTURE_COUNT: &str = "PINNED_FIXTURE_COUNT";
pub(crate) const PINNED_TRANSCRIPT_DIGEST: &str = "PINNED_TRANSCRIPT_DIGEST";
pub(crate) const PINNED_REORDERED_DIGEST: &str = "PINNED_REORDERED_DIGEST";
/// Both rank orders of the profile `PINNED_REORDERED_DIGEST` is computed under; the record's
/// `reordered_profile` must name exactly this profile.
pub(crate) const PINNED_REORDERED_RANK_ORDER: &[u64] = &[1];
/// Human-authored records whose cited full revisions must all be inventoried sources.
pub(crate) const CITED_SOURCE_DOCUMENTS: &[&str] = &[
    "README.md",
    "provenance/SOURCE.md",
    "provenance/CHAOSCONTROL-BASELINE.md",
];

pub(crate) const SOURCE_INVENTORY_SCHEMA: &str = "campaign.provenance.v1";
pub(crate) const MAX_INVENTORY_SOURCES: usize = 64;
pub(crate) const MAX_INVENTORY_LIST_ITEMS: usize = 64;
pub(crate) const FULL_REVISION_LENGTH: usize = 40;
pub(crate) const BLAKE3_DIGEST_LENGTH: usize = 64;
pub(crate) const CONSUMER_RECORD_SCHEMA: &str = "campaign.consumers.v1";
pub(crate) const MAX_CONSUMER_LIST_ITEMS: usize = 64;
pub(crate) const NO_ASSERTION_LICENSE: &str = "NOASSERTION";
pub(crate) const CHOREGRAPH_SOURCE_ID: &str = "choregraph-history";
pub(crate) const CHOREGRAPH_PACKAGE_FAMILY: &str = "choregraph";
pub(crate) const SOURCE_DECLARATION_MARKER: &str = "campaign-source:";
pub(crate) const SOURCE_DECLARATION_PREFIX: &str = "// campaign-source: ";
pub(crate) const CORE_PACKAGE: &str = "campaign-core";
pub(crate) const MEMBER_SOURCE_DIRECTORY: &str = "src";
pub(crate) const LIBRARY_ROOT_FILE: &str = "src/lib.rs";

pub(crate) const CAIRN_SOURCE_REFERENCE: &str =
    "github:OnixResearch/cairn/5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b";
pub(crate) const CAIRN_SOURCE_NAR_HASH: &str =
    "sha256-xYCzFUurU87B0BPKH9ns34nydx6eJVjTUVGb8GLi+to=";

pub(crate) const FORBIDDEN_DYLINT_TOKENS: &[&str] = &[
    "disabled_lints",
    "warning_budget",
    "warning_budgets",
    "finding_baseline",
    "finding_baselines",
    "baseline_path",
];

pub(crate) const VENDORED_CAIRN_POLICY_FILES: &[crate::model::VendoredFile] = &[
    crate::model::VendoredFile {
        path: "cairn-policy/contracts.ncl",
        digest_blake3: "bc537dc32ff01fc9004dc032b78ad6fb5855c5c6f0158ecda219d5912b46619c",
    },
    crate::model::VendoredFile {
        path: "cairn-policy/default.ncl",
        digest_blake3: "c3b5981d71789b04ff3a7567a84a7b1df29b5703dcb3a65e664143713e77a97e",
    },
    crate::model::VendoredFile {
        path: "cairn-policy/generated/cairn-policy.json",
        digest_blake3: "7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9",
    },
    crate::model::VendoredFile {
        path: "cairn-policy/lifecycle-store-contracts.ncl",
        digest_blake3: "cfa0842a93d34d87db473c5f3a26b2e6b8285bf4d0fffc6bc9ffc54c77f32b99",
    },
    crate::model::VendoredFile {
        path: "cairn-policy/nominal-evidence-identities.ncl",
        digest_blake3: "6e4798683072695e0c06feb2ceac31d9aa9fa74f6821c048c875dfe5b4c92b6c",
    },
];
