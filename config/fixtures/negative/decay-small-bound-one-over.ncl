# One unit of penalty past the small-bound decay edge: (2^61 + 1) * (3 - 1) + 2^62 exceeds 2^63.
let E = import "../edit.ncl" in
E.profile
|> E.as_ranked
|> E.set_in "bounds" "maximum_selections" 3
|> E.set_in "bounds" "maximum_outstanding_selections" 3
|> E.set_in "bounds" "maximum_rank_magnitude" 4611686018427387904
|> E.set_in "selection" "decay" { rule = 'subtract_from_first_rank, penalty_per_selection = 2305843009213693953 }
|> E.checked_profile
