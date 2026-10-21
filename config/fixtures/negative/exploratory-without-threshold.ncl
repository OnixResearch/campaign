let E = import "../edit.ncl" in
E.profile |> E.drop_in "selection" "exploratory_threshold_parts_per_million"
|> E.checked_profile
