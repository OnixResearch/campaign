//! A member's compiled production file set, derived from its manifest: the `[lib]` root, every
//! `[[bin]]` root, and the build script (`package.build`, or an implicit `build.rs`).
//!
//! Library and binary roots must lie under `<member>/src`, where every Rust file is scanned; the
//! only admitted production file outside it is `<member>/build.rs`, which is scanned under the same
//! source profile. Test-only targets (`[[example]]`, `[[test]]`, `[[bench]]`) must lie under the
//! member's `src`, `tests`, `examples`, or `benches` directory. Anything else compiles code from
//! outside the member, which no scan reads (`campaign.functional_core.effect`).

const LIBRARY_TABLE: &str = "lib";
const BINARY_ARRAY: &str = "bin";
/// Target arrays built only by test, example, and bench builds (the `--all-targets` gates).
const TEST_ONLY_ARRAYS: &[&str] = &["example", "test", "bench"];
/// Member directories a test-only target may compile from.
const TEST_ONLY_DIRECTORIES: &[&str] = &["src", "tests", "examples", "benches"];
const PACKAGE_TABLE: &str = "package";
const PATH_KEY: &str = "path";
const BUILD_KEY: &str = "build";
const BUILD_SCRIPT: &str = "build.rs";
const DEFAULT_BINARY: &str = "src/main.rs";
const BINARY_DIRECTORY: &str = "src/bin";
/// `src/bin/<name>/main.rs`, a binary root Cargo discovers in a directory.
const BINARY_MAIN: &str = "/main.rs";

/// One member manifest whose target paths are resolved against the member directory.
#[derive(Clone, Copy)]
pub(crate) struct MemberManifest<'data> {
    pub(crate) member_path: &'data str,
    pub(crate) manifest_path: &'data str,
    pub(crate) manifest: &'data toml::Table,
}

/// Repository-relative compiled roots of one member.
#[derive(Debug)]
pub(crate) struct Targets {
    /// The library root, which the core requires to be `#![no_std]`.
    pub(crate) library: String,
    /// Library, binary, and build-script roots.
    roots: Vec<String>,
    /// `<member>/src/bin`, whose direct children are implicit binary roots.
    binary_directory: String,
    build_script: Option<String>,
    /// `<member>/src`.
    source_directory: String,
}

/// A member manifest and the `<member>/src` directory its targets must stay under.
#[derive(Clone, Copy)]
struct Scope<'data> {
    member: MemberManifest<'data>,
    source_directory: &'data str,
}

impl Targets {
    /// Whether the file is compiled production source: under `src/` or the build script.
    pub(crate) fn compiles(&self, path: &str) -> bool {
        self.build_script.as_deref() == Some(path)
            || crate::syntax::is_contained(crate::syntax::Containment {
                path,
                ancestor: &self.source_directory,
            })
    }

    /// Whether the file is a crate root: the library, a binary (declared, `src/main.rs`, or
    /// auto-discovered under `src/bin`), or the build script.
    pub(crate) fn is_root(&self, path: &str) -> bool {
        let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        let is_binary_main = path.ends_with(BINARY_MAIN)
            && parent
                .rsplit_once('/')
                .is_some_and(|(grandparent, _)| grandparent.eq(self.binary_directory.as_str()));
        self.roots.iter().any(|root| root.eq(path))
            || parent.eq(self.binary_directory.as_str())
            || is_binary_main
    }

    pub(crate) fn module_roots(&self) -> crate::patterns::gating::Roots<'_> {
        crate::patterns::gating::Roots {
            files: &self.roots,
            binary_directory: &self.binary_directory,
            build_script: self.build_script.as_deref(),
        }
    }
}

/// Resolve and admit every compiled root the manifest declares.
// r[impl campaign.functional_core.effect]
pub(crate) fn derive(member: MemberManifest<'_>) -> Result<Targets, crate::failure::CheckError> {
    let source_directory = format!(
        "{}/{}",
        member.member_path,
        crate::policy::MEMBER_SOURCE_DIRECTORY
    );
    let scope = Scope {
        member,
        source_directory: &source_directory,
    };
    let library = member
        .manifest
        .get(LIBRARY_TABLE)
        .and_then(|library| library.get(PATH_KEY));
    let library = scope.inside_source(library, crate::policy::LIBRARY_ROOT_FILE)?;
    let mut roots = vec![
        library.clone(),
        format!("{}/{DEFAULT_BINARY}", member.member_path),
    ];
    let binaries = target_array(member.manifest, BINARY_ARRAY);
    roots.reserve(binaries.len().saturating_add(1));
    for binary in binaries {
        if let Some(path) = binary.get(PATH_KEY) {
            roots.push(scope.inside_source(Some(path), DEFAULT_BINARY)?);
        }
    }
    for kind in TEST_ONLY_ARRAYS {
        for target in target_array(member.manifest, kind) {
            if let Some(path) = target.get(PATH_KEY) {
                scope.test_only(path)?;
            }
        }
    }
    let build_script = scope.build_script()?;
    roots.extend(build_script.iter().cloned());
    assert!(roots.iter().all(|root| !root.is_empty()));
    assert!(roots.contains(&library), "the library is a crate root");
    Ok(Targets {
        library,
        roots,
        binary_directory: format!("{}/{BINARY_DIRECTORY}", member.member_path),
        build_script,
        source_directory,
    })
}

/// The entries of one `[[kind]]` target array.
fn target_array<'manifest>(
    manifest: &'manifest toml::Table,
    kind: &str,
) -> &'manifest [toml::Value] {
    manifest
        .get(kind)
        .and_then(toml::Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

impl Scope<'_> {
    /// A test-only target path that must resolve under one of the member's target directories.
    fn test_only(self, declared: &toml::Value) -> Result<(), crate::failure::CheckError> {
        let relative = declared
            .as_str()
            .ok_or_else(|| not_a_path(self.member, PATH_KEY))?;
        let resolved = crate::syntax::resolve(crate::syntax::RelativePath {
            base: self.member.member_path,
            path: relative,
        })
        .ok_or_else(|| outside_test_directories(self.member, relative))?;
        let is_admitted = TEST_ONLY_DIRECTORIES.iter().any(|directory| {
            crate::syntax::is_contained(crate::syntax::Containment {
                path: &resolved,
                ancestor: &format!("{}/{directory}", self.member.member_path),
            })
        });
        if !is_admitted {
            return Err(outside_test_directories(self.member, &resolved));
        }
        assert!(
            resolved.starts_with(self.member.member_path),
            "a test-only target stays inside its member"
        );
        Ok(())
    }

    /// A target path (or its default) that must resolve under `<member>/src`.
    fn inside_source(
        self,
        declared: Option<&toml::Value>,
        default: &str,
    ) -> Result<String, crate::failure::CheckError> {
        let relative = match declared {
            None => default,
            Some(value) => value
                .as_str()
                .ok_or_else(|| not_a_path(self.member, PATH_KEY))?,
        };
        let resolved = self.resolve(relative)?;
        if !self.is_scanned(&resolved) {
            return Err(outside(self.member, &resolved));
        }
        Ok(resolved)
    }

    /// The build script: `package.build` as a path or flag, else the implicit `build.rs`, which may
    /// lie only at `<member>/build.rs` or under `<member>/src`.
    fn build_script(self) -> Result<Option<String>, crate::failure::CheckError> {
        let declared = self
            .member
            .manifest
            .get(PACKAGE_TABLE)
            .and_then(|package| package.get(BUILD_KEY));
        let relative = match declared {
            None | Some(toml::Value::Boolean(true)) => BUILD_SCRIPT,
            Some(toml::Value::Boolean(false)) => return Ok(None),
            Some(toml::Value::String(path)) => path.as_str(),
            Some(_) => return Err(not_a_path(self.member, BUILD_KEY)),
        };
        let resolved = self.resolve(relative)?;
        let root_script = format!("{}/{BUILD_SCRIPT}", self.member.member_path);
        if resolved != root_script && !self.is_scanned(&resolved) {
            return Err(outside(self.member, &resolved));
        }
        assert!(!resolved.is_empty(), "a build script names a file");
        assert!(
            resolved.starts_with(self.member.member_path),
            "the build script stays inside its member"
        );
        Ok(Some(resolved))
    }

    fn is_scanned(self, path: &str) -> bool {
        crate::syntax::is_contained(crate::syntax::Containment {
            path,
            ancestor: self.source_directory,
        })
    }

    fn resolve(self, relative: &str) -> Result<String, crate::failure::CheckError> {
        crate::syntax::resolve(crate::syntax::RelativePath {
            base: self.member.member_path,
            path: relative,
        })
        .ok_or_else(|| outside(self.member, relative))
    }
}

fn outside(member: MemberManifest<'_>, path: &str) -> crate::failure::CheckError {
    crate::failure::CheckError::violation(
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        format!(
            "{} compiles {path} outside {}/{}, which the source scan does not read",
            member.manifest_path,
            member.member_path,
            crate::policy::MEMBER_SOURCE_DIRECTORY
        ),
    )
}

fn outside_test_directories(member: MemberManifest<'_>, path: &str) -> crate::failure::CheckError {
    let directory = member.member_path;
    crate::failure::CheckError::violation(
        crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
        format!(
            "{} compiles test-only target {path} outside {directory}/src, {directory}/tests, \
             {directory}/examples, and {directory}/benches",
            member.manifest_path
        ),
    )
}

fn not_a_path(member: MemberManifest<'_>, key: &str) -> crate::failure::CheckError {
    crate::failure::CheckError::failed(format!(
        "{} declares a {key} that is not a path",
        member.manifest_path
    ))
}
