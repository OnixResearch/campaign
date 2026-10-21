# One past i64::MAX: an admitted i64::MIN rank would have no negation.
let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_rank_magnitude" 9223372036854775808
|> E.checked_profile
