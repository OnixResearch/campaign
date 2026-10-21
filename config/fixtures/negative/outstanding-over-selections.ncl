let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_outstanding_selections" 65537
|> E.checked_profile
