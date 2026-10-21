//! Flatten Rust tokens into identifiers and punctuation, dropping comments, literals, attributes,
//! and lifetimes, and marking tokens that belong to `#[cfg(test)]` items.
//!
//! An outer `#[cfg(test)]` gates the next item of its level only. An item introduced by an item
//! keyword (`fn`, `impl`, `struct`, `let`, ...) ends at `;` or at its brace body; any other gated
//! member (an enum variant, a field, a match arm, an element) also ends at the next `,` of its
//! level, so later siblings stay production code.

/// Keywords that open an item or statement whose own commas (generics, where clauses) never end
/// it.
const ITEM_KEYWORDS: &[&str] = &[
    "async",
    "const",
    "enum",
    "extern",
    "fn",
    "impl",
    "let",
    "macro_rules",
    "mod",
    "static",
    "struct",
    "trait",
    "type",
    "union",
    "unsafe",
    "use",
];
/// Visibility, which precedes both items and fields without deciding which one follows.
const VISIBILITY: &str = "pub";

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Token {
    Ident(String),
    Punct(char),
}

#[derive(Debug)]
pub(super) struct Scanned {
    pub(super) token: Token,
    pub(super) is_test_only: bool,
}

impl Scanned {
    /// The identifier text when this token is an identifier outside test-only code.
    pub(super) fn production_ident(&self) -> Option<&str> {
        match &self.token {
            Token::Ident(identifier) if !self.is_test_only => Some(identifier),
            Token::Ident(_) | Token::Punct(_) => None,
        }
    }
}

/// One token-tree level: its trees, the next position, and its test context.
pub(super) struct Frame {
    trees: Vec<proc_macro2::TokenTree>,
    position: usize,
    is_test_level: bool,
    pending: Pending,
}

/// The `#[cfg(test)]`-gated member currently being scanned at one level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pending {
    /// No gated member.
    None,
    /// A gate whose member has not yet shown its first identifier.
    Undecided,
    /// A gated item that ends at `;` or its brace body.
    Item,
    /// A gated variant, field, arm, or element that also ends at `,`.
    Member,
}

/// An outer `#[...]` or inner `#![...]` attribute and the position after it.
struct Attribute {
    body: proc_macro2::Group,
    is_inner: bool,
    next: usize,
}

/// Depth-first flattening with an explicit frame stack instead of recursion.
struct Scanner {
    frames: Vec<Frame>,
    scanned: Vec<Scanned>,
}

pub(super) fn scan(
    source: &crate::model::SourceFile,
    is_test_file: bool,
) -> Result<Vec<Scanned>, crate::failure::CheckError> {
    let stream: proc_macro2::TokenStream = source.content.parse().map_err(|error| {
        crate::failure::CheckError::failed(format!(
            "invalid Rust tokens in {}: {error}",
            source.display_path
        ))
    })?;
    let mut scanner = Scanner {
        frames: Vec::with_capacity(crate::policy::MAX_TOKEN_NESTING),
        scanned: Vec::with_capacity(source.content.len()),
    };
    scanner.frames.push(Frame::new(stream, is_test_file));
    scanner.run()
}

impl Frame {
    fn new(stream: proc_macro2::TokenStream, is_test_level: bool) -> Self {
        Self {
            trees: stream.into_iter().collect(),
            position: 0,
            is_test_level,
            pending: Pending::None,
        }
    }

    const fn is_test_only(&self) -> bool {
        self.is_test_level || !matches!(self.pending, Pending::None)
    }

    /// The attribute starting at the `#` just consumed, if any.
    fn attribute(&self) -> Option<Attribute> {
        let (is_inner, bracket) = match self.trees.get(self.position) {
            Some(proc_macro2::TokenTree::Punct(bang)) if bang.as_char() == '!' => {
                (true, self.position.checked_add(1)?)
            }
            Some(_) | None => (false, self.position),
        };
        let proc_macro2::TokenTree::Group(body) = self.trees.get(bracket)? else {
            return None;
        };
        (body.delimiter() == proc_macro2::Delimiter::Bracket).then(|| Attribute {
            body: body.clone(),
            is_inner,
            next: bracket.saturating_add(1),
        })
    }

    /// The first identifier after a gate decides whether it gates an item or a member.
    fn decide(&mut self, identifier: &str) {
        if !matches!(self.pending, Pending::Undecided) || identifier.eq(VISIBILITY) {
            return;
        }
        self.pending = if ITEM_KEYWORDS.contains(&identifier) {
            Pending::Item
        } else {
            Pending::Member
        };
        debug_assert!(matches!(self.pending, Pending::Item | Pending::Member));
    }

    fn next_is_ident(&self) -> bool {
        matches!(
            self.trees.get(self.position),
            Some(proc_macro2::TokenTree::Ident(_))
        )
    }
}

impl Attribute {
    /// Exactly `cfg(test)`; other configuration predicates stay production code.
    pub(super) fn is_cfg_test(&self) -> bool {
        let trees: Vec<proc_macro2::TokenTree> = self.body.stream().into_iter().collect();
        let [
            proc_macro2::TokenTree::Ident(name),
            proc_macro2::TokenTree::Group(arguments),
        ] = trees.as_slice()
        else {
            return false;
        };
        let predicate: Vec<proc_macro2::TokenTree> = arguments.stream().into_iter().collect();
        *name == "cfg"
            && arguments.delimiter() == proc_macro2::Delimiter::Parenthesis
            && matches!(predicate.as_slice(), [proc_macro2::TokenTree::Ident(test)] if *test == "test")
    }
}

impl Scanner {
    fn run(mut self) -> Result<Vec<Scanned>, crate::failure::CheckError> {
        while let Some(frame) = self.frames.pop() {
            if self.frames.len() >= crate::policy::MAX_TOKEN_NESTING {
                return Err(crate::failure::CheckError::failed(
                    "Rust token nesting bound exceeded",
                ));
            }
            self.advance(frame);
        }
        assert!(self.frames.is_empty());
        assert!(self.scanned.len() <= self.scanned.capacity());
        Ok(self.scanned)
    }

    /// Consume one frame until it opens a group, which is scanned before the frame resumes.
    fn advance(&mut self, mut frame: Frame) {
        while let Some(tree) = frame.trees.get(frame.position).cloned() {
            frame.position = frame.position.saturating_add(1);
            let is_test_only = frame.is_test_only();
            match tree {
                proc_macro2::TokenTree::Group(group) => {
                    if group.delimiter() == proc_macro2::Delimiter::Brace {
                        frame.pending = Pending::None;
                    }
                    self.frames.push(frame);
                    self.frames.push(Frame::new(group.stream(), is_test_only));
                    return;
                }
                proc_macro2::TokenTree::Punct(punct) => {
                    self.punct(&mut frame, punct.as_char());
                }
                proc_macro2::TokenTree::Ident(ident) => {
                    let text = ident.to_string();
                    let bare = text.strip_prefix("r#").unwrap_or(&text).to_owned();
                    frame.decide(&bare);
                    self.push(Token::Ident(bare), is_test_only);
                }
                proc_macro2::TokenTree::Literal(_) => {}
            }
        }
    }

    /// Handle one punctuation character: attributes and lifetimes are dropped, `;` ends gated
    /// items, and `,` ends gated members.
    pub(super) fn punct(&mut self, frame: &mut Frame, character: char) {
        let is_test_only = frame.is_test_only();
        if character == '#'
            && let Some(attribute) = frame.attribute()
        {
            frame.position = attribute.next;
            let is_cfg_test = attribute.is_cfg_test();
            frame.is_test_level |= is_cfg_test && attribute.is_inner;
            if is_cfg_test && !attribute.is_inner && frame.pending == Pending::None {
                frame.pending = Pending::Undecided;
            }
            return;
        }
        if character == '\'' && frame.next_is_ident() {
            frame.position = frame.position.saturating_add(1);
            return;
        }
        let is_member_end = character == ',' && frame.pending != Pending::Item;
        if character == ';' || is_member_end {
            frame.pending = Pending::None;
        }
        self.push(Token::Punct(character), is_test_only);
    }

    fn push(&mut self, token: Token, is_test_only: bool) {
        self.scanned.push(Scanned {
            token,
            is_test_only,
        });
    }
}

#[cfg(test)]
mod tests;
