//! The suite evidence campaign-conformance pins in `tests::consumers`, read as exact top-level
//! `const` literals, and the claim every recorded pilot must make: exactly that evidence.
//!
//! A suite change without re-pinning fails that conformance test; a record that is not refreshed
//! with the re-pinned constants fails here (`campaign.consumer_pilots.valid`).

type Admitted<T> = super::fields::Admitted<T>;

/// A pinned constant of the suite module, named for diagnostics.
#[derive(Clone, Copy)]
struct Declared<'text> {
    path: &'text str,
    name: &'static str,
}

/// Every pilot claims exactly the pinned fixture count and digests under the pinned profile.
// r[impl campaign.consumer_pilots.valid]
pub(super) fn validate(pilots: &super::Pilots, members: &[crate::model::Member]) -> Admitted<()> {
    let pinned = parse(locate(members)?)?;
    assert!(pinned.fixtures > 0);
    require_profile(&pilots.reordered_profile)?;
    pilots
        .pilots
        .iter()
        .try_for_each(|pilot| require_claim(pilot, &pinned))
}

fn locate(members: &[crate::model::Member]) -> Admitted<&crate::model::SourceFile> {
    members
        .iter()
        .flat_map(|member| member.files.iter())
        .find(|file| file.display_path == crate::policy::CONSUMER_PINNED_SUITE)
        .ok_or_else(|| {
            super::fields::violation(
                super::VALID,
                format!(
                    "{} is absent; campaign-conformance must pin the suite evidence consumer \
                     pilots claim",
                    crate::policy::CONSUMER_PINNED_SUITE
                ),
            )
        })
}

/// The three pinned constants, each an exact literal of its declared type.
pub(super) fn parse(source: &crate::model::SourceFile) -> Admitted<super::Conformance> {
    let path = source.display_path.as_str();
    let file = syn::parse_file(&source.content).map_err(|error| {
        super::fields::violation(super::VALID, format!("{path} is not valid Rust: {error}"))
    })?;
    let declared = |name| Declared { path, name };
    Ok(super::Conformance {
        fixtures: count(&file, declared(crate::policy::PINNED_FIXTURE_COUNT))?,
        transcript_digest: digest(&file, declared(crate::policy::PINNED_TRANSCRIPT_DIGEST))?,
        reordered_profile_digest: digest(&file, declared(crate::policy::PINNED_REORDERED_DIGEST))?,
    })
}

/// The one top-level `const` `name`, which must be private, non-generic, and carry no attribute
/// but doc comments.
fn declaration<'file>(
    file: &'file syn::File,
    declared: Declared<'_>,
) -> Admitted<&'file syn::ItemConst> {
    let Declared { path, name } = declared;
    assert!(!name.is_empty());
    assert!(!path.is_empty());
    let found: Vec<&syn::ItemConst> = file
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Const(constant) = item
                && constant.ident == name
            {
                Some(constant)
            } else {
                None
            }
        })
        .collect();
    let constant = match found.as_slice() {
        [constant] => *constant,
        [] => {
            return Err(super::fields::violation(
                super::VALID,
                format!("{path} declares no top-level const {name}"),
            ));
        }
        [_, _, ..] => {
            return Err(super::fields::violation(
                super::VALID,
                format!("{path} declares const {name} more than once"),
            ));
        }
    };
    assert_eq!(constant.ident, name);
    if is_plain(constant) {
        Ok(constant)
    } else {
        Err(malformed(
            declared,
            "a private, attribute-free, non-generic top-level const",
        ))
    }
}

fn is_plain(constant: &syn::ItemConst) -> bool {
    let is_private = matches!(constant.vis, syn::Visibility::Inherited);
    let is_generic =
        !constant.generics.params.is_empty() || constant.generics.where_clause.is_some();
    let is_documented_only = constant
        .attrs
        .iter()
        .all(|attribute| attribute.path().is_ident("doc"));
    is_private && !is_generic && is_documented_only
}

fn malformed(declared: Declared<'_>, form: &str) -> crate::failure::CheckError {
    let Declared { path, name } = declared;
    super::fields::violation(super::VALID, format!("{path}: const {name} must be {form}"))
}

fn is_named_type(ty: &syn::Type, name: &str) -> bool {
    matches!(ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(name))
}

/// The literal of a `const` whose initializer is a bare literal expression.
fn literal(constant: &syn::ItemConst) -> Option<&syn::Lit> {
    if let syn::Expr::Lit(syn::ExprLit { attrs, lit }) = &*constant.expr
        && attrs.is_empty()
    {
        Some(lit)
    } else {
        None
    }
}

/// `const NAME: usize = <positive decimal literal>;`: the literal is exactly the canonical decimal
/// of its value (no suffix, separator, radix prefix, or leading zero).
fn count(file: &syn::File, declared: Declared<'_>) -> Admitted<u64> {
    const FORM: &str = "`usize = <positive decimal literal>`";
    let constant = declaration(file, declared)?;
    let Some(syn::Lit::Int(integer)) = literal(constant) else {
        return Err(malformed(declared, FORM));
    };
    let digits = integer.to_string();
    let value = integer
        .base10_parse::<u64>()
        .ok()
        .filter(|value| *value > 0);
    match value {
        Some(value) if digits == value.to_string() && is_named_type(&constant.ty, "usize") => {
            assert!(value > 0, "a positive fixture count");
            assert!(digits.bytes().all(|byte| byte.is_ascii_digit()));
            Ok(value)
        }
        Some(_) | None => Err(malformed(declared, FORM)),
    }
}

/// `const NAME: &str = "<lowercase 64-hex BLAKE3 digest>";`, a plain string literal.
fn digest(file: &syn::File, declared: Declared<'_>) -> Admitted<String> {
    const FORM: &str = "`&str = \"<lowercase 64-hex BLAKE3 digest>\"`";
    let constant = declaration(file, declared)?;
    let Some(syn::Lit::Str(text)) = literal(constant) else {
        return Err(malformed(declared, FORM));
    };
    let is_str_reference = matches!(
        &*constant.ty,
        syn::Type::Reference(reference)
            if reference.lifetime.is_none()
                && reference.mutability.is_none()
                && is_named_type(&reference.elem, "str")
    );
    let value = text.value();
    let is_plain_literal = text.token().to_string() == format!("\"{value}\"");
    let is_digest = crate::syntax::is_blake3_digest(&value);
    let is_exact_form = is_str_reference && is_plain_literal;
    if !(is_exact_form && is_digest) {
        return Err(malformed(declared, FORM));
    }
    assert_eq!(value.len(), crate::policy::BLAKE3_DIGEST_LENGTH);
    assert!(value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    Ok(value)
}

fn require_profile(profile: &super::ReorderedProfile) -> Admitted<()> {
    let pinned = crate::policy::PINNED_REORDERED_RANK_ORDER;
    if profile.exploration_order == pinned && profile.retention_rank_order == pinned {
        return Ok(());
    }
    Err(super::fields::violation(
        super::VALID,
        format!(
            "reordered_profile must be exploration_order {pinned:?} and retention_rank_order \
             {pinned:?}, the profile {} computes {} under",
            crate::policy::CONSUMER_PINNED_SUITE,
            crate::policy::PINNED_REORDERED_DIGEST
        ),
    ))
}

/// Every differing claim, reported together so a stale pilot names all it must refresh.
// r[impl campaign.consumer_pilots.valid]
fn require_claim(pilot: &super::Pilot, pinned: &super::Conformance) -> Admitted<()> {
    let claimed = &pilot.conformance;
    let comparisons = [
        (
            "fixtures",
            crate::policy::PINNED_FIXTURE_COUNT,
            claimed.fixtures.to_string(),
            pinned.fixtures.to_string(),
        ),
        (
            "transcript_digest",
            crate::policy::PINNED_TRANSCRIPT_DIGEST,
            claimed.transcript_digest.clone(),
            pinned.transcript_digest.clone(),
        ),
        (
            "reordered_profile_digest",
            crate::policy::PINNED_REORDERED_DIGEST,
            claimed.reordered_profile_digest.clone(),
            pinned.reordered_profile_digest.clone(),
        ),
    ];
    let stale: Vec<String> = comparisons
        .iter()
        .filter(|(_, _, claim, pin)| claim != pin)
        .map(|(field, name, claim, pin)| format!("{field} {claim} but {name} = {pin}"))
        .collect();
    assert!(stale.len() <= comparisons.len());
    if stale.is_empty() {
        assert_eq!(claimed, pinned);
        return Ok(());
    }
    Err(super::fields::violation(
        super::VALID,
        format!(
            "consumer pilot {} claims conformance evidence {} does not pin: {}",
            pilot.id,
            crate::policy::CONSUMER_PINNED_SUITE,
            stale.join("; ")
        ),
    ))
}
