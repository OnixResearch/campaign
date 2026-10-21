let E = import "../edit.ncl" in
E.profile |> E.set_in "owners" "selection" ""
|> E.checked_profile
