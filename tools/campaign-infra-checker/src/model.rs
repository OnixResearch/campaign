//! Explicit inputs for repository validation.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CheckCommand {
    All,
    Boundaries,
    CairnPolicySource,
}

impl CheckCommand {
    pub(crate) fn includes_boundaries(self) -> bool {
        self == Self::All || self == Self::Boundaries
    }

    pub(crate) fn includes_cairn_policy_source(self) -> bool {
        self == Self::All || self == Self::CairnPolicySource
    }
}

#[derive(Debug)]
pub(crate) struct CheckRequest {
    pub(crate) command: CheckCommand,
    pub(crate) root: std::path::PathBuf,
    pub(crate) cairn_source_root: Option<std::path::PathBuf>,
    pub(crate) choregraph_history_source: Option<std::path::PathBuf>,
}

#[derive(Debug)]
pub(crate) struct ProcessInputs {
    pub(crate) args: Vec<std::ffi::OsString>,
    pub(crate) cairn_source_root: Option<std::ffi::OsString>,
    pub(crate) choregraph_history_source: Option<std::ffi::OsString>,
}

#[derive(Debug)]
pub(crate) struct BoundaryInputs {
    pub(crate) root_manifest: NamedText,
    pub(crate) cargo_lock: NamedText,
    pub(crate) flake_nix: NamedText,
    pub(crate) flake_lock: NamedText,
    pub(crate) repository_ncl: NamedText,
    pub(crate) generated_repository_json: NamedText,
    pub(crate) profile_copy: ProjectionCopy,
    pub(crate) core_manifest: NamedText,
    pub(crate) adapter_manifest: NamedText,
    pub(crate) pre_commit_config: NamedText,
    pub(crate) dylint_config: NamedText,
    pub(crate) publication_record: NamedText,
    pub(crate) cited_documents: Vec<NamedText>,
    pub(crate) choregraph_history_manifest: Option<NamedText>,
    /// Cargo configuration and toolchain files anywhere below the root.
    pub(crate) build_configuration: BuildConfiguration,
    pub(crate) inventory: InventoryFacts,
    pub(crate) consumers: ConsumerFacts,
    pub(crate) members: Vec<Member>,
}

/// The parsed source inventory plus the Campaign paths it names that are absent from the root.
#[derive(Debug)]
pub(crate) struct InventoryFacts {
    pub(crate) sources: crate::inventory::Sources,
    pub(crate) absent_campaign_paths: Vec<String>,
}

/// The admitted consumer pilot record plus the in-repo pilot paths it names that are absent from the root.
#[derive(Debug)]
pub(crate) struct ConsumerFacts {
    pub(crate) pilots: crate::consumers::Pilots,
    pub(crate) absent_campaign_paths: Vec<String>,
}

/// One declared workspace member with its manifest and every Rust file below it.
#[derive(Debug)]
pub(crate) struct Member {
    pub(crate) path: String,
    pub(crate) manifest: String,
    pub(crate) files: Vec<SourceFile>,
}

#[derive(Debug)]
pub(crate) struct CairnPolicyInputs {
    pub(crate) source_manifest: NamedText,
    pub(crate) compared_files: Vec<VendoredFileComparison>,
}

#[derive(Debug)]
pub(crate) struct NamedText {
    pub(crate) relative_path: &'static str,
    pub(crate) content: String,
}

/// Borrowed manifest text with the path used in diagnostics.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ManifestText<'text> {
    pub(crate) relative_path: &'text str,
    pub(crate) content: &'text str,
}

/// Build configuration found in every directory below the root outside the checker's own fixture
/// roots, each path relative to the root.
#[derive(Debug)]
pub(crate) struct BuildConfiguration {
    /// Every `.cargo/config.toml` and `.cargo/config`.
    pub(crate) cargo_configurations: Vec<String>,
    /// Every `rust-toolchain.toml` and `rust-toolchain`, with its text.
    pub(crate) toolchain_files: Vec<ConfigurationFile>,
}

/// A configuration file whose path is relative to the checked repository root.
#[derive(Debug)]
pub(crate) struct ConfigurationFile {
    pub(crate) relative_path: String,
    pub(crate) content: String,
}

/// A Rust file whose display path is relative to the checked repository root.
#[derive(Debug)]
pub(crate) struct SourceFile {
    pub(crate) display_path: String,
    pub(crate) content: String,
}

#[derive(Debug)]
pub(crate) struct VendoredFileComparison {
    pub(crate) relative_path: &'static str,
    pub(crate) source_relative_path: std::path::PathBuf,
    pub(crate) local_bytes: Vec<u8>,
    pub(crate) source_bytes: Vec<u8>,
}

/// A committed copy of a generated configuration projection and the projection it must equal byte for byte.
#[derive(Debug)]
pub(crate) struct ProjectionCopy {
    pub(crate) copy_path: &'static str,
    pub(crate) projection_path: &'static str,
    pub(crate) copy_bytes: Vec<u8>,
    pub(crate) projection_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct VendoredFile {
    pub(crate) path: &'static str,
    pub(crate) digest_blake3: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TextAbsence<'text> {
    pub(crate) relative_path: &'static str,
    pub(crate) forbidden: &'text str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TextRequirement<'text> {
    pub(crate) content: &'text str,
    pub(crate) relative_path: &'static str,
    pub(crate) expected: &'text str,
}
