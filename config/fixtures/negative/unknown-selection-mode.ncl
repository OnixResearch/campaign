let E = import "../edit.ncl" in
E.profile |> E.set_in "selection" "mode" 'greedy
|> E.checked_profile
