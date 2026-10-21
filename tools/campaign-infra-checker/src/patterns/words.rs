//! Split Rust identifiers into lowercase words at underscores and case boundaries.

/// `ReplaySuccess` -> `replay`, `success`; `HTTPServer` -> `http`, `server`; `debug_assert` -> `debug`, `assert`.
pub(super) fn split(identifier: &str) -> Vec<String> {
    let characters: Vec<char> = identifier.chars().collect();
    let mut found = Vec::with_capacity(characters.len());
    let mut current = String::with_capacity(identifier.len());
    for (index, character) in characters.iter().enumerate() {
        let is_separator = *character == '_';
        if (is_separator || starts_word(&characters, index)) && !current.is_empty() {
            found.push(std::mem::take(&mut current));
        }
        if !is_separator {
            current.extend(character.to_lowercase());
        }
    }
    if !current.is_empty() {
        found.push(current);
    }
    assert!(found.len() <= characters.len());
    assert!(found.iter().all(|word| !word.is_empty()));
    found
}

/// The identifier lowercased with underscores removed, for proper-noun families.
pub(super) fn joined(identifier: &str) -> String {
    identifier
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

/// An uppercase letter after a lowercase letter or digit, or the last capital of an acronym.
fn starts_word(characters: &[char], index: usize) -> bool {
    let Some(character) = characters.get(index).filter(|found| found.is_uppercase()) else {
        return false;
    };
    let previous = index
        .checked_sub(1)
        .and_then(|before| characters.get(before));
    let next = index.checked_add(1).and_then(|after| characters.get(after));
    let is_after_lower =
        previous.is_some_and(|before| before.is_lowercase() || before.is_ascii_digit());
    let is_acronym_end = previous.is_some_and(|before| before.is_uppercase())
        && next.is_some_and(|after| after.is_lowercase());
    debug_assert!(character.is_uppercase());
    is_after_lower || is_acronym_end
}

#[cfg(test)]
mod tests {
    #[test]
    fn splits_snake_camel_and_acronyms() {
        assert_eq!(super::split("ReplaySuccess"), ["replay", "success"]);
        assert_eq!(super::split("debug_assert"), ["debug", "assert"]);
        assert_eq!(super::split("HTTPServer"), ["http", "server"]);
        assert_eq!(
            super::split("select_lineage_v2"),
            ["select", "lineage", "v2"]
        );
        assert_eq!(super::split("NON_CLAIM_REPLAY"), ["non", "claim", "replay"]);
        assert_eq!(super::split("Debug"), ["debug"]);
    }

    #[test]
    fn joins_proper_nouns_across_styles() {
        assert_eq!(super::joined("ChaosControl"), "chaoscontrol");
        assert_eq!(super::joined("chaos_control_fault"), "chaoscontrolfault");
    }
}
