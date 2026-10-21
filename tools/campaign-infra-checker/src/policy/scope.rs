//! Per-member dependency allowlists and source-scan profiles.
//!
//! Every workspace member must have exactly one package row; the workspace root may not be a
//! package, and every member takes dependencies only from the registry, a workspace member, or the
//! pinned Choregraph revision (`crate::dependencies::origin`).

/// Dependency allowlist and source profile for one workspace package.
#[derive(Debug)]
pub(crate) struct Package {
    pub(crate) name: &'static str,
    pub(crate) dependencies: Allowlist,
    pub(crate) features: Features,
    pub(crate) sources: Option<SourceProfile>,
}

/// Which dependency features a package may enable beyond each dependency's defaults.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Features {
    /// Any feature of an admitted dependency.
    Any,
    /// Normal and build dependencies keep their default feature set: no dependency entry (nor the
    /// workspace entry it inherits) lists `features`, and no `[features]` entry enables a
    /// dependency feature (`dependency/feature`). Dev-dependencies are test-only and unrestricted.
    DefaultOnly,
}

/// Allowed direct dependencies for one package.
#[derive(Debug)]
pub(crate) enum Allowlist {
    /// Tooling outside the shared contract: no allowlist or consumer family applies, but the effect
    /// and history dependency families still do.
    Exempt,
    /// `normal` covers normal, build, and target tables; `dev_only` extends dev-dependency tables.
    Allowed {
        normal: &'static [&'static str],
        dev_only: &'static [&'static str],
    },
}

/// Source-pattern scan applied to a member's compiled sources: every Rust file under
/// `<member>/src` and the member's build script.
#[derive(Debug)]
pub(crate) struct SourceProfile {
    pub(crate) needs_no_std: bool,
    pub(crate) families: &'static [Family],
}

/// A named identifier family that violates one specification rule.
#[derive(Debug)]
pub(crate) struct Family {
    pub(crate) rule: crate::rules::Rule,
    pub(crate) name: &'static str,
    pub(crate) matcher: Matcher,
}

/// How a family matches one identifier.
#[derive(Debug)]
pub(crate) enum Matcher {
    /// Any lowercase word equals one entry.
    Words(&'static [&'static str]),
    /// Any lowercase word starts with one entry.
    WordPrefixes(&'static [&'static str]),
    /// The lowercase identifier without underscores contains one entry.
    Joined(&'static [&'static str]),
    /// One word from each list occurs in the same identifier.
    WordPair(&'static [&'static str], &'static [&'static str]),
}

/// The core's normal dependencies. Cargo unifies their features across the workspace, so every
/// member inherits them from `[workspace.dependencies]` unchanged
/// (`crate::dependencies::features::allowlisted`).
pub(crate) const CORE_NORMAL_DEPENDENCIES: &[&str] = &["blake3"];

/// Direct dependency families that grant ambient randomness.
// r[impl campaign.entropy.ambient]
pub(crate) const EFFECT_DEPENDENCY_FAMILIES: &[&str] = &["rand", "getrandom", "fastrand"];

/// Choregraph-owned Universal Weave topology dependency families rejected for every Campaign
/// crate, alongside the `choregraph` package family outside a package's allowlist.
pub(crate) const HISTORY_DEPENDENCY_FAMILIES: &[&str] = &["weave", "universal"];

/// Consumer, runtime, VM, database, and framework dependency families rejected for every Campaign crate.
// r[impl campaign.boundary.consumer_leak]
pub(crate) const CONSUMER_DEPENDENCY_FAMILIES: &[&str] = &[
    "chaoscontrol",
    "animus",
    "lattice",
    "tokio",
    "async-std",
    "smol",
    "kvm",
    "kvm-ioctls",
    "kvm-bindings",
    "vmm",
    "vm-cohort",
    "firecracker",
    "rusqlite",
    "sqlx",
    "diesel",
    "postgres",
    "tokio-postgres",
    "redis",
    "sled",
    "rocksdb",
    "redb",
    "axum",
    "actix",
    "actix-web",
    "hyper",
    "reqwest",
    "tonic",
    "warp",
    "rocket",
    "pyo3",
    "fuser",
];

/// Rows for every admitted workspace package.
// r[impl campaign.boundary]
pub(crate) const PACKAGES: &[Package] = &[
    Package {
        name: crate::policy::CORE_PACKAGE,
        dependencies: Allowlist::Allowed {
            normal: CORE_NORMAL_DEPENDENCIES,
            dev_only: &["proptest"],
        },
        features: Features::DefaultOnly,
        sources: Some(SourceProfile {
            needs_no_std: true,
            families: crate::policy::families::CORE,
        }),
    },
    Package {
        name: "campaign-choregraph",
        dependencies: Allowlist::Allowed {
            normal: &["campaign-core", "choregraph-history", "blake3"],
            dev_only: &[],
        },
        features: Features::Any,
        sources: Some(SourceProfile {
            needs_no_std: false,
            families: crate::policy::families::ADAPTER,
        }),
    },
    Package {
        name: "campaign-conformance",
        dependencies: Allowlist::Allowed {
            normal: &["campaign-core", "campaign-choregraph", "blake3"],
            dev_only: &["choregraph-history", "campaign-profile"],
        },
        features: Features::Any,
        sources: Some(SourceProfile {
            needs_no_std: false,
            families: crate::policy::families::CONFORMANCE,
        }),
    },
    Package {
        name: "campaign-profile",
        dependencies: Allowlist::Allowed {
            normal: &["campaign-core", "serde", "serde_json"],
            dev_only: &[],
        },
        features: Features::Any,
        sources: None,
    },
    Package {
        name: "campaign-infra-checker",
        dependencies: Allowlist::Exempt,
        features: Features::Any,
        sources: None,
    },
];

pub(crate) fn for_package(package: &str) -> Option<&'static Package> {
    PACKAGES.iter().find(|candidate| candidate.name == package)
}
