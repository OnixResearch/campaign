//! Source discovery controls use isolated temporary trees.

#[test]
fn orders_nested_sources_by_path() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("a"))?;
    std::fs::write(root.path().join("z.rs"), "fn z() {}")?;
    std::fs::write(root.path().join("a/lib.rs"), "fn a() {}")?;
    let files = super::sources(root.path(), "")?;
    let expected_paths = ["a/lib.rs", "z.rs"];
    assert_eq!(files.len(), expected_paths.len());
    for (file, expected) in files.iter().zip(expected_paths) {
        assert_eq!(file.display_path, expected);
        assert!(!file.content.is_empty());
    }
    Ok(())
}

#[test]
fn rejects_missing_root() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    assert!(super::sources(root.path(), "missing").is_err());
    Ok(())
}

#[test]
fn rejects_oversized_source() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let file = std::fs::File::create(root.path().join("lib.rs"))?;
    file.set_len(crate::policy::SOURCE_READ_LIMIT)?;
    assert!(super::sources(root.path(), "").is_err());
    Ok(())
}

#[test]
fn rejects_invalid_utf8_source() -> Result<(), Box<dyn std::error::Error>> {
    const INVALID_UTF8_BYTE: u8 = 0xff;
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("lib.rs"), [INVALID_UTF8_BYTE])?;
    assert!(super::sources(root.path(), "").is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn rejects_symlink_member_and_root() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    std::fs::write(other.path().join("lib.rs"), "fn outside() {}")?;
    let link = root.path().join("link");
    std::os::unix::fs::symlink(other.path(), &link)?;
    assert!(super::sources(root.path(), "").is_err());
    assert!(super::sources(root.path(), "link").is_err());
    Ok(())
}
