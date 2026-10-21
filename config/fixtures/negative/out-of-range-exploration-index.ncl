let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "exploration_order" [16]
|> E.checked_profile
