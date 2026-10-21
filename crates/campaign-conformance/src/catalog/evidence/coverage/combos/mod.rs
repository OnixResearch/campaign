//! Combinations of declared feature values: every 4-way combination a set of classes carries, and
//! every receipt kind paired with every 3-way combination of the classes it is presented at.

/// Number of values of one feature: the statuses for feature zero, two for a boolean.
const fn radix(feature: usize) -> usize {
    if feature == 0 {
        super::features::STATUSES.len()
    } else {
        2
    }
}

/// Number of sets of `width` distinct features: `FEATURE_COUNT` choose `width`.
const fn set_count(width: usize) -> usize {
    let mut count = 1_usize;
    let mut chosen = 0_usize;
    while chosen < width {
        count = count.saturating_mul(super::features::FEATURE_COUNT.saturating_sub(chosen));
        chosen = chosen.saturating_add(1);
        count /= chosen;
    }
    count
}

/// Every set of `WIDTH` distinct features, in lexicographic order.
fn feature_sets<const WIDTH: usize>() -> alloc::vec::Vec<[usize; WIDTH]> {
    debug_assert!(WIDTH <= super::features::FEATURE_COUNT);
    let mut sets = alloc::vec::Vec::with_capacity(set_count(WIDTH));
    let mut set = [0_usize; WIDTH];
    for (position, slot) in set.iter_mut().enumerate() {
        *slot = position;
    }
    loop {
        sets.push(set);
        let Some(position) = (0..WIDTH).rev().find(|position| {
            set[*position]
                < super::features::FEATURE_COUNT
                    .saturating_sub(WIDTH)
                    .saturating_add(*position)
        }) else {
            debug_assert_eq!(sets.len(), set_count(WIDTH));
            return sets;
        };
        set[position] = set[position].saturating_add(1);
        for next in position.saturating_add(1)..WIDTH {
            set[next] = set[next.saturating_sub(1)].saturating_add(1);
        }
    }
}

/// Position of the values `class` takes on `features` among every value combination of them
/// (mixed radix, at most `8 * 2 * 2 * 2` combinations).
fn position<const WIDTH: usize>(
    class: super::features::StateClass,
    features: &[usize; WIDTH],
) -> usize {
    features.iter().fold(0, |position, feature| {
        position
            .saturating_mul(radix(*feature))
            .saturating_add(usize::from(class.value(*feature)))
    })
}

/// The 3-way combinations `class` carries: for every set of three features (in lexicographic
/// order), the bit of the class's value combination among every value combination of them (at
/// most `8 * 2 * 2` combinations).
pub(super) fn triple_bits(class: super::features::StateClass) -> alloc::vec::Vec<u32> {
    let bits: alloc::vec::Vec<u32> = feature_sets::<3>()
        .iter()
        .map(|features| 1_u32 << position(class, features))
        .collect();
    debug_assert_eq!(bits.len(), set_count(3));
    bits
}

/// The combinations a set of presentations carries: for every set of four features, the value
/// combinations some class carries (one bit per combination); for every set of three features
/// and every receipt kind, the value combinations of the classes the kind is presented at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) struct Carried {
    quads: alloc::vec::Vec<u64>,
    triples: alloc::vec::Vec<[u32; super::kinds::KIND_COUNT]>,
}

impl Carried {
    /// No combination carried.
    pub(in crate::catalog::evidence) fn empty() -> Self {
        Self {
            quads: alloc::vec![0; feature_sets::<4>().len()],
            triples: alloc::vec![[0; super::kinds::KIND_COUNT]; feature_sets::<3>().len()],
        }
    }

    /// The combinations `presented` carries: each class with the receipt kinds presented at it.
    pub(in crate::catalog::evidence) fn of(
        presented: &[(super::features::StateClass, u32)],
    ) -> Self {
        let mut carried = Self::empty();
        let quads = feature_sets::<4>();
        let triples = feature_sets::<3>();
        for (class, kinds) in presented {
            for (mask, features) in carried.quads.iter_mut().zip(&quads) {
                *mask |= 1_u64 << position(*class, features);
            }
            for (masks, features) in carried.triples.iter_mut().zip(&triples) {
                let bit = 1_u32 << position(*class, features);
                for (kind, mask) in masks.iter_mut().enumerate() {
                    if kinds & (1 << kind) != 0 {
                        *mask |= bit;
                    }
                }
            }
        }
        carried
    }

    /// The combinations of `required` this set does not carry: missing 4-way combinations and
    /// missing (kind, 3-way combination) pairs.
    pub(in crate::catalog::evidence) fn missing(&self, required: &Self) -> (u32, u32) {
        let quads = self
            .quads
            .iter()
            .zip(&required.quads)
            .map(|(carried, needed)| (needed & !carried).count_ones())
            .sum();
        let triples = self
            .triples
            .iter()
            .zip(&required.triples)
            .flat_map(|(carried, needed)| carried.iter().zip(needed))
            .map(|(carried, needed)| (needed & !carried).count_ones())
            .sum();
        (quads, triples)
    }

    #[cfg(test)]
    /// The requirement bits one class with `kinds` carries, as global positions: each 4-way
    /// combination, then each (3-way combination, kind) pair.
    pub(in crate::catalog::evidence) fn bits(
        class: super::features::StateClass,
        kinds: u32,
    ) -> alloc::vec::Vec<usize> {
        let quads = feature_sets::<4>();
        let triples = feature_sets::<3>();
        let mut bits = alloc::vec::Vec::with_capacity(quads.len().saturating_add(triples.len()));
        let mut base = 0_usize;
        for features in &quads {
            bits.push(base.saturating_add(position(class, features)));
            base = base.saturating_add(64);
        }
        for features in &triples {
            let at = position(class, features);
            for kind in 0..super::kinds::KIND_COUNT {
                if kinds & (1 << kind) != 0 {
                    bits.push(
                        base.saturating_add(kind.saturating_mul(32))
                            .saturating_add(at),
                    );
                }
            }
            base = base.saturating_add(32 * 32);
        }
        bits
    }
}
