# The former boundary decay: a u64-maximum penalty over u64-maximum selections leaves the i64 rank domain.
let E = import "../edit.ncl" in
let max = 18446744073709551615 in
let with_bounds = fun profile =>
  std.record.update "bounds" (std.record.map (fun _field _value => max) profile.bounds) profile
in
E.profile
|> with_bounds
|> E.set_in "bounds" "maximum_rank_dimensions" 4
|> E.set_in "bounds" "maximum_rank_magnitude" 9223372036854775807
|> E.set_in "selection" "exploration_order" [3, 2, 1, 0]
|> E.set_in "selection" "decay" { rule = 'subtract_from_first_rank, penalty_per_selection = max }
|> E.checked_profile
