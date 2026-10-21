//! Reject known source-scope bypasses in every member file: module-wide lint suppressions and the
//! `include` identifier (`include!` under any path, rename, or macro body). This is syntactic
//! evidence, not a purity proof.

use syn::visit::Visit;

pub(crate) mod members;
pub(crate) mod targets;

const SUPPRESSIONS: &[&str] = &["allow", "expect"];

const EXCLUDED_COMPONENTS: &[&str] = &[
    "target",
    "ui_tests",
    "generated",
    "OUT_DIR",
    "rustlib",
    "registry",
    ".cargo",
];

pub(crate) fn validate(
    source: &crate::model::SourceFile,
) -> Result<(), crate::failure::CheckError> {
    if is_excluded(&source.display_path) {
        return Err(crate::failure::CheckError::failed(
            "Rust source occupies an excluded lint path",
        ));
    }
    let syntax = parse(source)?;
    if let Some(reason) = audit_syntax(&syntax) {
        return Err(crate::failure::CheckError::failed(format!(
            "{}: {reason}",
            source.display_path
        )));
    }
    assert!(
        !source.display_path.is_empty(),
        "an audited source names its file"
    );
    let scope = crate::patterns::indirection::ANY_MEMBER;
    let found = crate::patterns::indirection::find_in(source, scope)?;
    assert!(
        syntax
            .attrs
            .iter()
            .all(|attribute| !weakens_scope(&attribute.meta)),
        "suppressions are refused before the token walk"
    );
    match found {
        Some(found) => Err(crate::failure::CheckError::violation(
            crate::rules::Rule::FUNCTIONAL_CORE_EFFECT,
            format!("{}: {}", source.display_path, found.describe()),
        )),
        None => Ok(()),
    }
}

fn parse(source: &crate::model::SourceFile) -> Result<syn::File, crate::failure::CheckError> {
    syn::parse_file(&source.content).map_err(|error| {
        crate::failure::CheckError::failed(format!(
            "invalid Rust in {}: {error}",
            source.display_path
        ))
    })
}

#[allow(
    unknown_lints,
    reason = "rustscope uses an item-level Octet lint that plain Rust does not register"
)]
#[allow(
    missing_const_fn,
    reason = "rustscope owns native path admission and Path::components is not const on the pinned toolchain"
)]
fn is_excluded(path: &str) -> bool {
    std::path::Path::new(path).components().any(|component| {
        EXCLUDED_COMPONENTS
            .iter()
            .any(|excluded| component.as_os_str() == *excluded)
    })
}

fn audit_syntax(syntax: &syn::File) -> Option<&'static str> {
    let mut audit = Audit {
        violation: scope_violation(&syntax.attrs),
    };
    audit.visit_file(syntax);
    audit.violation
}

struct Audit {
    violation: Option<&'static str>,
}

impl<'ast> Visit<'ast> for Audit {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        self.violation = self.violation.or_else(|| scope_violation(&module.attrs));
        syn::visit::visit_item_mod(self, module);
    }
}

fn scope_violation(attributes: &[syn::Attribute]) -> Option<&'static str> {
    attributes.iter().find_map(|attribute| {
        if weakens_scope(&attribute.meta) {
            Some("file-wide or module-wide lint suppression is not admitted")
        } else {
            None
        }
    })
}

fn weakens_scope(meta: &syn::Meta) -> bool {
    applies_attribute(meta, SUPPRESSIONS)
}

/// Whether the attribute is one of `names`, directly or through any `cfg_attr` nesting; an
/// unparsable `cfg_attr` counts as applying.
pub(crate) fn applies_attribute(meta: &syn::Meta, names: &[&str]) -> bool {
    let mut pending = vec![meta.clone()];
    while let Some(current) = pending.pop() {
        if names.iter().any(|name| current.path().is_ident(name)) {
            return true;
        }
        if let syn::Meta::List(list) = current
            && list.path.is_ident("cfg_attr")
        {
            match list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) {
                Ok(nested) => pending.extend(nested),
                Err(_) => return true,
            }
        }
    }
    false
}

#[cfg(test)]
mod tests;
