# Inclusive edges: u64-maximum bounds with the i64-maximum rank magnitude, cross-field equalities, full-scale
# threshold, and every rank dimension.
let E = import "../edit.ncl" in
let max = 18446744073709551615 in
let with_bounds = fun profile =>
  std.record.update "bounds" (std.record.map (fun _field _value => max) profile.bounds) profile
in
E.profile
|> with_bounds
|> E.set_in "bounds" "maximum_rank_dimensions" 4
|> E.set_in "bounds" "maximum_rank_magnitude" 9223372036854775807
|> E.set_in "selection" "exploratory_threshold_parts_per_million" 1000000
|> E.set_in "selection" "exploration_order" [3, 2, 1, 0]
|> E.set_in "stopping" "finding_limit" max
|> E.set_in "retention" "rank_order" [0, 1, 2, 3]
|> E.checked_profile
