let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "decay" { rule = 'divide_by_selections }
|> E.checked_profile
