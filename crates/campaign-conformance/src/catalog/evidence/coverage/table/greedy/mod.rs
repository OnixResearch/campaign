//! The greedy covering that selects the pinned table: repeatedly the declared class that carries
//! the most combinations no selected class carries yet (4-way feature combinations and
//! (receipt kind, 3-way combination) pairs of its compatible kinds), the lowest code first among
//! equals, until every combination the declared classes carry is carried.

/// Codes of the greedy covering of `classes`, in selection order.
pub(in crate::catalog::evidence) fn covering(
    classes: &[super::super::features::StateClass],
) -> alloc::vec::Vec<u16> {
    let bits: alloc::vec::Vec<alloc::vec::Vec<usize>> = classes
        .iter()
        .map(|class| {
            super::super::combos::Carried::bits(*class, super::super::kinds::compatible(*class))
        })
        .collect();
    let universe = bits
        .iter()
        .flatten()
        .max()
        .map_or(0, |bit| bit.saturating_add(1));
    let mut covered = alloc::vec![false; universe];
    let gain = |covered: &[bool], class: &[usize]| {
        class
            .iter()
            .filter(|bit| covered.get(**bit).is_some_and(|is_covered| !*is_covered))
            .count()
    };
    let mut heap: alloc::collections::BinaryHeap<(usize, core::cmp::Reverse<u16>, usize)> = classes
        .iter()
        .zip(&bits)
        .enumerate()
        .map(|(index, (class, bits))| {
            (
                gain(&covered, bits),
                core::cmp::Reverse(class.code()),
                index,
            )
        })
        .collect();
    let mut selected = alloc::vec::Vec::with_capacity(classes.len());
    while let Some((_, code, index)) = heap.pop() {
        let Some(class_bits) = bits.get(index) else {
            continue;
        };
        let fresh = gain(&covered, class_bits);
        if fresh == 0 {
            continue;
        }
        let key = (fresh, code, index);
        if heap.peek().is_some_and(|top| *top > key) {
            heap.push(key);
            continue;
        }
        for bit in class_bits {
            if let Some(slot) = covered.get_mut(*bit) {
                *slot = true;
            }
        }
        selected.push(code.0);
    }
    debug_assert!(
        bits.iter()
            .flatten()
            .all(|bit| covered.get(*bit).is_some_and(|is_covered| *is_covered)),
        "the selected classes carry every combination the classes carry"
    );
    debug_assert!(selected.len() <= classes.len());
    selected
}
