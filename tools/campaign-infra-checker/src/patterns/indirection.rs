//! Compile-time reads and source indirection, refused as tokens rather than as parsed items.
//!
//! The walk visits every token tree of a file: attribute bodies (`#[doc = ..]`, `cfg_attr`), macro
//! arguments, and macro bodies, none of which `syn` or the identifier scan expands. A compile-time
//! file or build-environment macro is refused wherever its identifier appears, so neither a
//! `macro_rules!` body, an `$m:ident` argument, nor a `use` rename can reach it without naming it.
//! `macro_rules!` definitions, `use` renames, and `path` attributes are refused outright in the
//! shared crates; every member refuses the `include` identifier.

/// `include!`, refused in every member.
const INCLUDE_MACRO: &str = "include";
const INCLUDE: &[&str] = &[INCLUDE_MACRO];
/// Identifiers of the compile-time file and build-environment macros.
const READS: &[&str] = &[
    INCLUDE_MACRO,
    "include_bytes",
    "include_str",
    "env",
    "option_env",
];
const MACRO_DEFINITION: &str = "macro_rules";
const PATH_ATTRIBUTE: &str = "path";
const CONFIGURED_ATTRIBUTE: &str = "cfg_attr";
const USE: &str = "use";
const RENAME: &str = "as";
const UNNAMED: &str = "_";
const RAW_PREFIX: &str = "r#";

/// The first indirection a token walk found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Finding {
    /// A refused identifier, anywhere in the token stream.
    Identifier(String),
    /// A `macro_rules!` definition.
    MacroDefinition,
    /// A `path` attribute, directly or through `cfg_attr`.
    PathAttribute,
    /// `use .. as <alias>`.
    Rename(String),
}

/// What a walk refuses.
#[derive(Clone, Copy)]
pub(crate) struct Scope {
    identifiers: &'static [&'static str],
    is_shared: bool,
}

/// Every member: the `include` identifier.
pub(crate) const ANY_MEMBER: Scope = Scope {
    identifiers: INCLUDE,
    is_shared: false,
};
/// Core, adapter, and conformance sources: every compile-time read, macro definitions, renames,
/// and `path` attributes.
pub(super) const SHARED: Scope = Scope {
    identifiers: READS,
    is_shared: true,
};

/// One token-tree level and whether it lies inside a `use` item.
struct Level {
    trees: Vec<proc_macro2::TokenTree>,
    position: usize,
    is_use: bool,
}

impl Finding {
    /// The refusal text after the file path.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Identifier(name) if name == INCLUDE_MACRO => format!(
                "identifier `{name}` names include! source indirection, which is not admitted"
            ),
            Self::Identifier(name) => format!(
                "identifier `{name}` names a compile-time file or build-environment read, which is \
                 not admitted"
            ),
            Self::MacroDefinition => {
                "macro_rules! definitions hide tokens from the scan and are not admitted".to_owned()
            }
            Self::PathAttribute => {
                "a #[path] attribute is source indirection and is not admitted".to_owned()
            }
            Self::Rename(alias) => {
                format!("`use .. as {alias}` renames an import, which is not admitted")
            }
        }
    }
}

/// Tokenize `source` and walk it.
// r[impl campaign.functional_core.effect]
pub(crate) fn find_in(
    source: &crate::model::SourceFile,
    scope: Scope,
) -> Result<Option<Finding>, crate::failure::CheckError> {
    let stream: proc_macro2::TokenStream = source.content.parse().map_err(|error| {
        crate::failure::CheckError::failed(format!(
            "invalid Rust tokens in {}: {error}",
            source.display_path
        ))
    })?;
    find(stream, scope)
}

/// The first refused token shape in `stream`, depth first.
fn find(
    stream: proc_macro2::TokenStream,
    scope: Scope,
) -> Result<Option<Finding>, crate::failure::CheckError> {
    assert!(
        !scope.identifiers.is_empty(),
        "a walk refuses at least one identifier"
    );
    let mut levels = vec![Level::new(stream, false)];
    while let Some(mut level) = levels.pop() {
        if levels.len() >= crate::policy::MAX_TOKEN_NESTING {
            return Err(crate::failure::CheckError::failed(
                "Rust token nesting bound exceeded",
            ));
        }
        while let Some(tree) = level.trees.get(level.position).cloned() {
            level.position = level.position.saturating_add(1);
            if let proc_macro2::TokenTree::Group(group) = tree {
                let child = Level::new(group.stream(), level.is_use);
                levels.push(level);
                levels.push(child);
                break;
            }
            if let Some(found) = level.inspect(&tree, scope) {
                return Ok(Some(found));
            }
        }
    }
    assert!(levels.is_empty(), "every level was walked");
    Ok(None)
}

impl Level {
    fn new(stream: proc_macro2::TokenStream, is_use: bool) -> Self {
        Self {
            trees: stream.into_iter().collect(),
            position: 0,
            is_use,
        }
    }

    /// Check the token just consumed; `position` already points past it.
    fn inspect(&mut self, tree: &proc_macro2::TokenTree, scope: Scope) -> Option<Finding> {
        if let proc_macro2::TokenTree::Ident(ident) = tree {
            return self.identifier(&bare(ident), scope);
        }
        let proc_macro2::TokenTree::Punct(punct) = tree else {
            return None;
        };
        let character = punct.as_char();
        if character.eq(&';') {
            self.is_use = false;
        }
        if character.eq(&'#') && scope.is_shared {
            return self
                .attribute_body()
                .filter(is_path_attribute)
                .map(|_| Finding::PathAttribute);
        }
        None
    }

    fn identifier(&mut self, name: &str, scope: Scope) -> Option<Finding> {
        if scope.identifiers.contains(&name) {
            return Some(Finding::Identifier(name.to_owned()));
        }
        if !scope.is_shared {
            return None;
        }
        if name.eq(MACRO_DEFINITION) {
            return Some(Finding::MacroDefinition);
        }
        if name.eq(USE) && !self.next_is_punct('<') {
            self.is_use = true;
        }
        if name.eq(RENAME) && self.is_use {
            return self
                .next_ident()
                .filter(|alias| alias.ne(UNNAMED))
                .map(Finding::Rename);
        }
        None
    }

    fn next_is_punct(&self, character: char) -> bool {
        matches!(
            self.trees.get(self.position),
            Some(proc_macro2::TokenTree::Punct(punct)) if punct.as_char().eq(&character)
        )
    }

    fn next_ident(&self) -> Option<String> {
        if let Some(proc_macro2::TokenTree::Ident(ident)) = self.trees.get(self.position) {
            Some(bare(ident))
        } else {
            None
        }
    }

    /// The bracketed body of the `#[..]` or `#![..]` attribute whose `#` was just consumed.
    fn attribute_body(&self) -> Option<proc_macro2::TokenStream> {
        let bracket = if self.next_is_punct('!') {
            self.position.checked_add(1)?
        } else {
            self.position
        };
        match self.trees.get(bracket) {
            Some(proc_macro2::TokenTree::Group(group))
                if group.delimiter() == proc_macro2::Delimiter::Bracket =>
            {
                Some(group.stream())
            }
            Some(_) | None => None,
        }
    }
}

/// An attribute body that is `path ..`, or a `cfg_attr` that applies one at any nesting.
fn is_path_attribute(body: &proc_macro2::TokenStream) -> bool {
    let mut pending = vec![body.clone()];
    while let Some(current) = pending.pop() {
        let trees: Vec<proc_macro2::TokenTree> = current.into_iter().collect();
        let name = match trees.first() {
            Some(proc_macro2::TokenTree::Ident(ident)) => bare(ident),
            Some(_) | None => continue,
        };
        if name == PATH_ATTRIBUTE {
            return true;
        }
        if let (true, Some(proc_macro2::TokenTree::Group(arguments))) =
            (name == CONFIGURED_ATTRIBUTE, trees.get(1))
        {
            pending.extend(applied_attributes(arguments.stream()));
        }
    }
    false
}

/// The attributes a `cfg_attr(predicate, a, b, ..)` applies: every comma-separated item after the
/// predicate.
fn applied_attributes(arguments: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
    let trees: Vec<proc_macro2::TokenTree> = arguments.into_iter().collect();
    let mut items = Vec::with_capacity(trees.len().saturating_add(1));
    let mut current = proc_macro2::TokenStream::new();
    for tree in trees {
        if matches!(&tree, proc_macro2::TokenTree::Punct(punct) if punct.as_char().eq(&',')) {
            items.push(std::mem::take(&mut current));
        } else {
            current.extend([tree]);
        }
    }
    items.push(current);
    assert!(!items.is_empty(), "the predicate item always exists");
    items.into_iter().skip(1).collect()
}

/// An identifier's text with any raw prefix removed.
fn bare(ident: &proc_macro2::Ident) -> String {
    let text = ident.to_string();
    text.strip_prefix(RAW_PREFIX).unwrap_or(&text).to_owned()
}

#[cfg(test)]
mod tests;
