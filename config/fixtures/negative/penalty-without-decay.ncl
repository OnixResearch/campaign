let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "decay" { rule = 'none, penalty_per_selection = 1 }
|> E.checked_profile
