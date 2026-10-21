let E = import "../edit.ncl" in
E.profile |> E.drop_in "selection" "exploration_order"
|> E.checked_profile
