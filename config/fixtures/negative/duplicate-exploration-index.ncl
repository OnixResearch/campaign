let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "exploration_order" [1, 1]
|> E.checked_profile
