let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_outstanding_selections" 0
|> E.checked_profile
