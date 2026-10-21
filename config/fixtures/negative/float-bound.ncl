let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_depth" 1.5
|> E.checked_profile
