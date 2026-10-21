# Inclusive decay edge: penalty * (maximum_selections - 1) + maximum_rank_magnitude equals 2^63 exactly
# ((2^31 - 1) * 2^32 + 2^32), so the lowest decayed first rank is i64::MIN.
let E = import "../edit.ncl" in
E.profile
|> E.as_ranked
|> E.set_in "bounds" "maximum_selections" 4294967297
|> E.set_in "bounds" "maximum_rank_magnitude" 4294967296
|> E.set_in "selection" "decay" { rule = 'subtract_from_first_rank, penalty_per_selection = 2147483647 }
|> E.checked_profile
