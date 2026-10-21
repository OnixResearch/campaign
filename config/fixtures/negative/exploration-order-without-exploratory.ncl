let E = import "../edit.ncl" in
E.profile |> E.as_ranked |> E.set_in "selection" "exploration_order" [0]
|> E.checked_profile
