let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_retained_candidates" 65537
|> E.checked_profile
