let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "tie_break" 'insertion_order
|> E.checked_profile
