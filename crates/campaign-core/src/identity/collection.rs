const ADJACENT_PAIR_LENGTH: usize = 2;
const PREVIOUS_ID_INDEX: usize = 0;
const NEXT_ID_INDEX: usize = 1;

/// Returns true when adjacent canonical identities are distinct.
#[must_use]
pub fn unique_sorted_ids(ids: &[super::Id]) -> bool {
    ids.windows(ADJACENT_PAIR_LENGTH)
        .all(|window| window[PREVIOUS_ID_INDEX] != window[NEXT_ID_INDEX])
}

/// Returns a canonically sorted identity copy.
#[must_use]
pub fn sorted_copy(ids: &[super::Id]) -> alloc::vec::Vec<super::Id> {
    let mut out = ids.to_vec();
    out.sort();
    out
}

/// Returns the index of the entry keyed by `id` in `entries` sorted by that key, in logarithmic
/// time. Every keyed vector of a canonical frontier state is sorted by its key.
#[must_use]
pub(crate) fn sorted_position<T>(
    entries: &[T],
    id: super::Id,
    key: impl FnMut(&T) -> super::Id,
) -> Option<usize> {
    entries.binary_search_by_key(&id, key).ok()
}

/// Returns true when the keys of `entries` strictly increase: canonical order without a repeated
/// key. Every keyed vector of a current frontier state satisfies this.
#[must_use]
pub(crate) fn strictly_increasing_by_key<T>(
    entries: &[T],
    mut key: impl FnMut(&T) -> super::Id,
) -> bool {
    entries.is_sorted_by(|left, right| key(left) < key(right))
}
