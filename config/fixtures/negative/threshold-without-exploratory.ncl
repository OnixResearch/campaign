let E = import "../edit.ncl" in
E.profile |> E.as_ranked |> E.set_in "selection" "exploratory_threshold_parts_per_million" 100000
|> E.checked_profile
