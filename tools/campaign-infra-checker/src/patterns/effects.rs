//! Ambient-effect token shapes, checked in production and test code alike. Compile-time file and
//! build-environment reads are refused as identifiers by `indirection`.

/// A token sequence that performs or enables an ambient effect.
enum Shape {
    /// The identifier anywhere.
    Ident(&'static str),
    /// Any identifier with this prefix.
    IdentPrefix(&'static str),
    /// `root ::`.
    PathRoot(&'static str),
    /// `root :: child`.
    Path(&'static str, &'static str),
    /// Two adjacent keywords.
    Keywords(&'static str, &'static str),
    /// `name !`.
    Macro(&'static str),
}

// r[impl campaign.functional_core.effect]
const EFFECTS: &[(Shape, &str)] = &[
    (
        Shape::Ident("std"),
        "`std` facade (std::fs/env/net/process/time/thread/sync/io)",
    ),
    (Shape::Ident("async"), "`async` runtime"),
    (Shape::Ident("await"), "`await` runtime"),
    (Shape::Ident("tokio"), "`tokio` runtime"),
    (Shape::Ident("getrandom"), "`getrandom` randomness"),
    (Shape::PathRoot("rand"), "`rand::` randomness"),
    (Shape::Path("core", "sync"), "`core::sync` synchronization"),
    (Shape::Ident("atomic"), "`atomic` shared state"),
    (Shape::IdentPrefix("Atomic"), "atomic shared state"),
    (
        Shape::Keywords("static", "mut"),
        "`static mut` global state",
    ),
    (Shape::Macro("thread_local"), "`thread_local!` state"),
    (Shape::Macro("println"), "`println!` process I/O"),
    (Shape::Macro("eprintln"), "`eprintln!` process I/O"),
    (Shape::Macro("print"), "`print!` process I/O"),
    (Shape::Macro("eprint"), "`eprint!` process I/O"),
    (Shape::Macro("dbg"), "`dbg!` process I/O"),
];

/// A read-only view of the flattened tokens at one position.
#[derive(Clone, Copy)]
struct Window<'tokens> {
    tokens: &'tokens [super::tokens::Scanned],
    position: usize,
}

/// The description of the first ambient effect in `tokens`.
pub(super) fn find(tokens: &[super::tokens::Scanned]) -> Option<&'static str> {
    for position in 0..tokens.len() {
        let window = Window { tokens, position };
        if let Some((_, description)) = EFFECTS.iter().find(|(shape, _)| shape.is_at(window)) {
            return Some(description);
        }
    }
    None
}

impl Shape {
    pub(super) fn is_at(&self, window: Window<'_>) -> bool {
        let first = window.ident(0);
        match self {
            Self::Ident(name) => first == Some(*name),
            Self::IdentPrefix(prefix) => first.is_some_and(|ident| ident.starts_with(prefix)),
            Self::PathRoot(root) => first == Some(*root) && window.is_path_separator(1),
            Self::Path(root, child) => {
                first == Some(*root)
                    && window.is_path_separator(1)
                    && window.ident(3) == Some(*child)
            }
            Self::Keywords(keyword, next) => {
                first == Some(*keyword) && window.ident(1) == Some(*next)
            }
            Self::Macro(name) => first == Some(*name) && window.punct(1) == Some('!'),
        }
    }
}

impl<'tokens> Window<'tokens> {
    const fn token(self, step: usize) -> Option<&'tokens super::tokens::Token> {
        let Some(index) = self.position.checked_add(step) else {
            return None;
        };
        if index < self.tokens.len() {
            Some(&self.tokens[index].token)
        } else {
            None
        }
    }

    const fn ident(self, step: usize) -> Option<&'tokens str> {
        match self.token(step) {
            Some(super::tokens::Token::Ident(ident)) => Some(ident.as_str()),
            Some(super::tokens::Token::Punct(_)) | None => None,
        }
    }

    const fn punct(self, step: usize) -> Option<char> {
        match self.token(step) {
            Some(super::tokens::Token::Punct(punct)) => Some(*punct),
            Some(super::tokens::Token::Ident(_)) | None => None,
        }
    }

    /// `::` starting `step` tokens ahead.
    pub(super) fn is_path_separator(self, step: usize) -> bool {
        self.punct(step) == Some(':')
            && step
                .checked_add(1)
                .is_some_and(|after| self.punct(after) == Some(':'))
    }
}
