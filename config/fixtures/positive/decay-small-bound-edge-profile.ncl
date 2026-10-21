# The exact decay edge at a small selection bound: 2^61 * (3 - 1) + 2^62 == 2^63.
let E = import "../edit.ncl" in
E.profile
|> E.as_ranked
|> E.set_in "bounds" "maximum_selections" 3
|> E.set_in "bounds" "maximum_outstanding_selections" 3
|> E.set_in "bounds" "maximum_rank_magnitude" 4611686018427387904
|> E.set_in "selection" "decay" { rule = 'subtract_from_first_rank, penalty_per_selection = 2305843009213693952 }
|> E.checked_profile
