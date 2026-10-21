//! File-module declarations in a member's compiled sources.
//!
//! A module is test-only when every declaration naming it (file or inline, found anywhere in the
//! member) is gated: it carries exactly `#[cfg(test)]` and no other `cfg` attribute, or it sits
//! inside such a gated inline module or file. A file is test-only when its module or one of its
//! ancestor modules is; a `tests.rs` file or a `tests` directory is production code unless so
//! declared, and a twin declaration without the gate (or with a second `cfg` beside it) keeps the
//! module production code (D93). The build script may declare no file module because its module
//! files lie outside `src/`; a `#[path]` attribute is refused by `indirection` wherever it
//! appears.

use syn::visit::Visit;

/// Files whose child modules resolve beside them rather than in a directory named after them.
const DIRECTORY_OWNER_FILES: &[&str] = &["lib.rs", "main.rs", "mod.rs"];
const RUST_SUFFIX: &str = ".rs";
const RAW_PREFIX: &str = "r#";
const CONFIGURATION_ATTRIBUTES: &[&str] = &["cfg"];

/// Crate roots of one member, whose child modules resolve beside them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Roots<'data> {
    /// The library, explicit binary, and build-script roots.
    pub(crate) files: &'data [String],
    /// `<member>/src/bin`, whose direct children are implicit binary roots.
    pub(crate) binary_directory: &'data str,
    pub(crate) build_script: Option<&'data str>,
}

/// Modules only gated declarations name, each as its path without `.rs`: the module is the file
/// `<base>.rs` or `<base>/mod.rs`, and its descendants lie below `<base>/`.
#[derive(Debug)]
pub(crate) struct TestModules {
    bases: Vec<String>,
}

impl TestModules {
    /// Whether the file is a `#[cfg(test)]` module or one of its descendants.
    pub(crate) fn contains(&self, path: &str) -> bool {
        self.bases.iter().any(|base| {
            path.strip_prefix(base.as_str())
                .is_some_and(|rest| rest == RUST_SUFFIX || rest.starts_with('/'))
        })
    }
}

/// Collect the test-only modules of `files`, refusing build-script file modules.
// r[impl campaign.functional_core.effect]
// r[impl campaign.boundary.test_code]
pub(crate) fn collect(
    files: &[&crate::model::SourceFile],
    roots: Roots<'_>,
) -> Result<TestModules, crate::failure::CheckError> {
    let mut walk = Walk {
        directory: String::new(),
        is_build_script: false,
        inline: Vec::with_capacity(crate::policy::MAX_TOKEN_NESTING),
        gated_inline: 0,
        bases: Vec::new(),
        ungated: Vec::new(),
        violation: None,
    };
    for file in files {
        let syntax = syn::parse_file(&file.content).map_err(|error| {
            crate::failure::CheckError::failed(format!(
                "invalid Rust in {}: {error}",
                file.display_path
            ))
        })?;
        walk.directory = module_directory(&file.display_path, roots);
        walk.is_build_script = roots.build_script == Some(file.display_path.as_str());
        walk.visit_file(&syntax);
        if let Some(reason) = walk.violation.take() {
            return Err(crate::failure::CheckError::violation(
                crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
                format!("{}: {reason}", file.display_path),
            ));
        }
        assert!(
            walk.inline.is_empty(),
            "every inline module is left after its walk"
        );
        assert_eq!(
            walk.gated_inline, 0,
            "every inline gate is closed after its walk"
        );
    }
    assert!(walk.bases.iter().all(|base| !base.ends_with(RUST_SUFFIX)));
    let Walk { bases, ungated, .. } = walk;
    let bases: Vec<String> = bases
        .into_iter()
        .filter(|base| !ungated.contains(base))
        .collect();
    debug_assert!(bases.iter().all(|base| !ungated.contains(base)));
    Ok(TestModules { bases })
}

/// The directory in which the modules a file declares resolve.
fn module_directory(path: &str, roots: Roots<'_>) -> String {
    let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
    let is_directory_owner = DIRECTORY_OWNER_FILES.contains(&name)
        || roots.files.iter().any(|root| root == path)
        || parent == roots.binary_directory
        || roots.build_script == Some(path);
    if is_directory_owner {
        String::from(parent)
    } else {
        String::from(path.strip_suffix(RUST_SUFFIX).unwrap_or(path))
    }
}

/// The walk of one file at a time: its module directory, the enclosing inline modules, how many of
/// them are gated, the gated file-module bases found so far, and every module path some ungated
/// declaration names.
struct Walk {
    directory: String,
    is_build_script: bool,
    inline: Vec<String>,
    gated_inline: usize,
    bases: Vec<String>,
    ungated: Vec<String>,
    violation: Option<String>,
}

impl<'ast> Visit<'ast> for Walk {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if self.violation.is_some() {
            return;
        }
        let text = module.ident.to_string();
        let name = text.strip_prefix(RAW_PREFIX).unwrap_or(&text).to_owned();
        let is_gated = self.gated_inline.checked_sub(1).is_some() || is_test_gate(&module.attrs);
        if !is_gated {
            self.ungated.push(self.base(&name));
        }
        if module.content.is_none() {
            if self.is_build_script {
                self.violation = Some(format!(
                    "the build script declares file module {name}, whose source lies outside src"
                ));
            } else if is_gated {
                self.bases.push(self.base(&name));
            }
            return;
        }
        self.inline.push(name);
        self.gated_inline = self.gated_inline.saturating_add(usize::from(is_gated));
        syn::visit::visit_item_mod(self, module);
        self.gated_inline = self.gated_inline.saturating_sub(usize::from(is_gated));
        self.inline.pop();
    }
}

impl Walk {
    /// `<directory>/<inline modules>/<name>`.
    fn base(&self, name: &str) -> String {
        let mut base = self.directory.clone();
        for part in self.inline.iter().map(String::as_str).chain([name]) {
            if !base.is_empty() {
                base.push('/');
            }
            base.push_str(part);
        }
        assert!(base.ends_with(name));
        base
    }
}

/// Exactly `#[cfg(test)]` and no other `cfg` (directly or through `cfg_attr`): the only gate that
/// exempts a module from identifier families.
fn is_test_gate(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(is_cfg_test)
        && attributes.iter().all(|attribute| {
            is_cfg_test(attribute)
                || !crate::rustscope::applies_attribute(&attribute.meta, CONFIGURATION_ATTRIBUTES)
        })
}

fn is_cfg_test(attribute: &syn::Attribute) -> bool {
    matches!(attribute.style, syn::AttrStyle::Outer)
        && matches!(
            &attribute.meta,
            syn::Meta::List(list) if list.path.is_ident("cfg") && list.tokens.to_string().as_str().eq("test")
        )
}

#[cfg(test)]
mod tests;
