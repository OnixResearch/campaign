//! SPDX license-expression admission without a license list: identifiers joined by AND, OR, WITH.

const OPERATORS: &[&str] = &["AND", "OR", "WITH"];

/// A declared license expression and the adoption it must permit.
#[derive(Clone, Copy, Debug)]
pub(super) struct Declared<'text> {
    pub(super) expression: &'text str,
    pub(super) adoption: super::Adoption,
}

/// Parser state: whether an operand comes next and the open parenthesis depth.
#[derive(Clone, Copy, Debug)]
struct Expression {
    is_operand_expected: bool,
    depth: usize,
}

impl Expression {
    const START: Self = Self {
        is_operand_expected: true,
        depth: 0,
    };

    fn accept(self, token: &str) -> Result<Self, &'static str> {
        let next = if token == "(" && self.is_operand_expected {
            Self {
                depth: self.depth.checked_add(1).ok_or("nesting overflow")?,
                ..self
            }
        } else if token == ")" && !self.is_operand_expected {
            Self {
                depth: self.depth.checked_sub(1).ok_or("unbalanced parenthesis")?,
                ..self
            }
        } else if OPERATORS.contains(&token) && !self.is_operand_expected {
            Self {
                is_operand_expected: true,
                ..self
            }
        } else if self.is_operand_expected && is_identifier(token) {
            Self {
                is_operand_expected: false,
                ..self
            }
        } else {
            return Err("not an SPDX license expression");
        };
        assert_ne!(
            (next.is_operand_expected, next.depth),
            (self.is_operand_expected, self.depth),
            "every accepted token advances the parser"
        );
        assert!(next.depth.abs_diff(self.depth) <= 1);
        Ok(next)
    }

    const fn is_complete(self) -> bool {
        !self.is_operand_expected && self.depth == 0
    }
}

/// Admit an SPDX expression; `NOASSERTION` alone is admitted only for reference-only sources.
pub(super) fn validate(declared: Declared<'_>) -> Result<(), &'static str> {
    if declared.expression == crate::policy::NO_ASSERTION_LICENSE {
        return if declared.adoption == super::Adoption::ReferenceOnly {
            Ok(())
        } else {
            Err("NOASSERTION is admitted only for reference_only sources")
        };
    }
    let spaced = declared.expression.replace('(', " ( ").replace(')', " ) ");
    let state = spaced
        .split_whitespace()
        .try_fold(Expression::START, Expression::accept)?;
    if !state.is_complete() {
        return Err("incomplete SPDX license expression");
    }
    Ok(())
}

/// An SPDX license or exception identifier, never an operator or `NOASSERTION`.
pub(super) fn is_identifier(token: &str) -> bool {
    token != crate::policy::NO_ASSERTION_LICENSE
        && !OPERATORS.contains(&token)
        && token.starts_with(|character: char| character.is_ascii_alphanumeric())
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

#[cfg(test)]
mod tests {
    fn check((expression, adoption): (&str, super::super::Adoption)) -> Result<(), &'static str> {
        super::validate(super::Declared {
            expression,
            adoption,
        })
    }

    #[test]
    fn admits_spdx_expressions() {
        let admitted = [
            "MIT",
            "MIT OR Apache-2.0",
            "AGPL-3.0-or-later AND MIT",
            "(MIT OR Apache-2.0) AND LicenseRef-owner-grant",
            "Apache-2.0 WITH LLVM-exception",
        ];
        assert_eq!(
            check(("NOASSERTION", super::super::Adoption::ReferenceOnly)),
            Ok(())
        );
        for expression in admitted {
            assert_eq!(
                check((expression, super::super::Adoption::Dependency)),
                Ok(()),
                "{expression}"
            );
        }
    }

    #[test]
    fn rejects_malformed_and_transferred_unknown_licenses() {
        let rejected = [
            "MIT or Apache-2.0",
            "MIT OR",
            "(MIT",
            "MIT)",
            "MIT Apache-2.0",
            "MIT/Apache-2.0",
            "NOASSERTION OR MIT",
        ];
        assert_eq!(
            check(("NOASSERTION", super::super::Adoption::VendoredCopy)),
            Err("NOASSERTION is admitted only for reference_only sources")
        );
        for expression in rejected {
            assert!(
                check((expression, super::super::Adoption::Dependency)).is_err(),
                "{expression}"
            );
        }
    }
}
