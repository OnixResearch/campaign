//! The armed pinned tests and whether one runs the suite: an armed test is a top-level `#[test]`
//! without disarming attributes whose body names every pinned constant, and it runs the suite when
//! it calls `run_suite` in its body or calls a top-level function of the same module whose body
//! calls `run_suite` (D93). A call is a path expression called by its last segment, or, inside
//! macro arguments, an identifier followed by a parenthesized group.

const TEST_ATTRIBUTE: &str = "test";
/// Attributes that stop a `#[test]` from running or from failing, directly or through `cfg_attr`.
const DISARMING_ATTRIBUTES: &[&str] = &["ignore", "should_panic", "cfg"];
/// The pinned constants an armed test names.
pub(super) const PINNED_CONSTANTS: &[&str] = &[
    crate::policy::PINNED_FIXTURE_COUNT,
    crate::policy::PINNED_TRANSCRIPT_DIGEST,
    crate::policy::PINNED_REORDERED_DIGEST,
];

/// The conformance suite entry point a pinned test must call.
pub(super) const SUITE_ENTRY: &str = "run_suite";

/// `test` calls `run_suite` in its body, or calls a top-level function of `module` that does.
pub(super) fn reaches_suite(module: &syn::File, test: &syn::ItemFn) -> bool {
    let direct = Callees::of(&test.block);
    direct.names(SUITE_ENTRY)
        || module.items.iter().any(|item| {
            let syn::Item::Fn(helper) = item else {
                return false;
            };
            direct.names(&helper.sig.ident.to_string())
                && Callees::of(&helper.block).names(SUITE_ENTRY)
        })
}

/// The names a block calls.
#[derive(Default)]
struct Callees {
    found: Vec<String>,
}

impl Callees {
    fn of(block: &syn::Block) -> Self {
        let mut calls = Self::default();
        syn::visit::Visit::visit_block(&mut calls, block);
        calls
    }

    fn names(&self, name: &str) -> bool {
        self.found.iter().any(|found| found.eq(name))
    }
}

impl<'ast> syn::visit::Visit<'ast> for Callees {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(callee) = call.func.as_ref()
            && let Some(last) = callee.path.segments.last()
        {
            self.found.push(last.ident.to_string());
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        let mut pending = vec![invocation.tokens.clone()];
        while let Some(current) = pending.pop() {
            let mut callee: Option<proc_macro2::Ident> = None;
            for tree in current {
                match tree {
                    proc_macro2::TokenTree::Ident(ident) => callee = Some(ident),
                    proc_macro2::TokenTree::Group(group) => {
                        if let Some(ident) = callee.take()
                            && group.delimiter() == proc_macro2::Delimiter::Parenthesis
                        {
                            self.found.push(ident.to_string());
                        }
                        pending.push(group.stream());
                    }
                    proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {
                        callee = None;
                    }
                }
            }
        }
        syn::visit::visit_macro(self, invocation);
    }
}

/// The top-level `#[test]` functions without disarming attributes whose bodies name every pinned
/// constant.
pub(super) fn armed_tests(pinned: &syn::File) -> Vec<&syn::ItemFn> {
    let armed: Vec<&syn::ItemFn> = pinned
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        })
        .filter(|function| {
            let is_test = function
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident(TEST_ATTRIBUTE));
            let is_armed = !function.attrs.iter().any(|attribute| {
                crate::rustscope::applies_attribute(&attribute.meta, DISARMING_ATTRIBUTES)
            });
            let mut body = Identifiers::default();
            syn::visit::Visit::visit_block(&mut body, &function.block);
            is_test && is_armed && PINNED_CONSTANTS.iter().all(|name| body.names(name))
        })
        .collect();
    debug_assert!(
        armed.iter().all(|test| test.attrs.iter().all(|attribute| {
            !crate::rustscope::applies_attribute(&attribute.meta, DISARMING_ATTRIBUTES)
        })),
        "no armed test carries a disarming attribute"
    );
    debug_assert!(armed.len() <= pinned.items.len());
    armed
}

/// Every identifier a syntax tree names, macro arguments included.
#[derive(Default)]
struct Identifiers {
    found: Vec<String>,
}

impl Identifiers {
    fn names(&self, name: &str) -> bool {
        self.found.iter().any(|found| found.eq(name))
    }
}

impl<'ast> syn::visit::Visit<'ast> for Identifiers {
    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        self.found.push(ident.to_string());
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        let mut pending = vec![invocation.tokens.clone()];
        while let Some(current) = pending.pop() {
            for tree in current {
                match tree {
                    proc_macro2::TokenTree::Ident(ident) => self.found.push(ident.to_string()),
                    proc_macro2::TokenTree::Group(group) => pending.push(group.stream()),
                    proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
                }
            }
        }
        syn::visit::visit_macro(self, invocation);
    }
}
