//! Pure lexical admission shared by inventory, pin, and declaration checks.

/// A repository-relative path and a candidate ancestor path.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Containment<'text> {
    pub(crate) path: &'text str,
    pub(crate) ancestor: &'text str,
}

/// A full lowercase 40-hex Git revision, never a short or symbolic reference.
pub(crate) fn is_full_revision(text: &str) -> bool {
    text.len() == crate::policy::FULL_REVISION_LENGTH && is_lowercase_hex(text)
}

/// A lowercase 64-hex BLAKE3 digest.
pub(crate) fn is_blake3_digest(text: &str) -> bool {
    text.len() == crate::policy::BLAKE3_DIGEST_LENGTH && is_lowercase_hex(text)
}

fn is_lowercase_hex(text: &str) -> bool {
    text.bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A kebab-case inventory identifier.
pub(crate) fn is_source_id(text: &str) -> bool {
    !text.is_empty()
        && text.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

/// A non-empty repository-relative path without absolute, parent, current, or empty components.
pub(crate) fn is_relative_path(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('/')
        && text
            .trim_end_matches('/')
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

/// Whether the path equals the ancestor or lies below it by whole components.
pub(crate) fn is_contained(containment: Containment<'_>) -> bool {
    let ancestor = containment.ancestor.trim_end_matches('/');
    containment.path == ancestor
        || containment
            .path
            .strip_prefix(ancestor)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// A manifest-relative `path` value and the repository directory it is relative to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RelativePath<'text> {
    pub(crate) base: &'text str,
    pub(crate) path: &'text str,
}

/// The repository-relative directory a path names, or `None` when it is absolute or leaves the
/// repository.
pub(crate) fn resolve(relative: RelativePath<'_>) -> Option<String> {
    let mut parts: Vec<&str> = relative
        .base
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for component in std::path::Path::new(relative.path).components() {
        match component {
            std::path::Component::Normal(part) => parts.push(part.to_str()?),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop()?;
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

/// Every standalone lowercase 40-hex token in `text`, in order.
pub(crate) fn full_revisions(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| is_full_revision(token))
        .collect()
}

#[cfg(test)]
mod tests {
    const REVISION: &str = "b3e08e19750f53bdbcae970cdf58a47a791ed20b";

    fn contained((path, ancestor): (&str, &str)) -> bool {
        super::is_contained(super::Containment { path, ancestor })
    }

    #[test]
    fn admits_only_full_lowercase_revisions() {
        assert!(super::is_full_revision(REVISION));
        assert!(!super::is_full_revision("b3e08e1"));
        assert!(!super::is_full_revision(&REVISION.to_uppercase()));
        assert!(!super::is_full_revision("main"));
    }

    #[test]
    fn admits_only_full_lowercase_blake3_digests() {
        let digest = REVISION.repeat(2);
        let digest = digest.get(..64).unwrap_or_default();
        assert!(super::is_blake3_digest(digest));
        assert!(!super::is_blake3_digest(&digest.to_uppercase()));
        assert!(!super::is_blake3_digest(REVISION));
        assert!(!super::is_blake3_digest(&format!("{digest}0")));
    }

    #[test]
    fn admits_kebab_source_ids() {
        let rejected = ["", "-a", "a-", "a--b", "A", "a_b", "a b"];
        assert!(super::is_source_id("choregraph-history"));
        for candidate in rejected {
            assert!(!super::is_source_id(candidate), "admitted {candidate:?}");
        }
    }

    #[test]
    fn admits_contained_relative_paths() {
        let rejected = ["", "/abs", "../up", "a/../b", "a//b", "./a"];
        assert!(super::is_relative_path("crates/campaign-core/src/"));
        for candidate in rejected {
            assert!(
                !super::is_relative_path(candidate),
                "admitted {candidate:?}"
            );
        }
    }

    #[test]
    fn contains_paths_by_whole_components() {
        assert!(contained(("crates/core/src/lib.rs", "crates/core")));
        assert!(contained(("crates/core", "crates/core/")));
        assert!(!contained(("crates/core-extra/src/lib.rs", "crates/core")));
    }

    #[test]
    fn extracts_standalone_revisions() {
        let text = format!("at `{REVISION}` and x{REVISION}y and {}", &REVISION[..7]);
        assert_eq!(super::full_revisions(&text), [REVISION]);
    }
}
