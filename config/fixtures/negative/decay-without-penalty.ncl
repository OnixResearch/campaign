let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "decay" { rule = 'subtract_from_first_rank }
|> E.checked_profile
