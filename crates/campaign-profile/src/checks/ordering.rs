//! Declared rank-dimension orders (`selection.exploration_order`, `retention.rank_order`).

/// Key of the exploratory selection order.
pub(crate) const EXPLORATION_ORDER: &str = "exploration_order";
/// Key of the retention rank order.
pub(crate) const RETENTION_RANK_ORDER: &str = "rank_order";

/// Accepts a non-empty order of unique rank-dimension indices, each below `rank_dimensions`.
pub(crate) fn validate(
    order_key: &'static str,
    order: Vec<u32>,
    rank_dimensions: u64,
) -> crate::failure::ProfileResult<Vec<u32>> {
    if order.is_empty() {
        return Err(crate::failure::ProfileError::EmptyOrder { order: order_key });
    }
    if let Some(index) = order
        .iter()
        .find(|index| u64::from(**index) >= rank_dimensions)
    {
        return Err(crate::failure::ProfileError::OrderIndexOutOfRange {
            order: order_key,
            index: *index,
        });
    }
    let mut sorted = order.clone();
    sorted.sort_unstable();
    let duplicate = sorted.windows(2).find_map(|pair| match pair {
        [first, second] if first == second => Some(*first),
        _ => None,
    });
    if let Some(index) = duplicate {
        return Err(crate::failure::ProfileError::DuplicateOrderIndex {
            order: order_key,
            index,
        });
    }
    debug_assert_eq!(sorted.len(), order.len());
    Ok(order)
}
