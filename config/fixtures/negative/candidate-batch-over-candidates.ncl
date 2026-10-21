let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_candidate_batch" 65537
|> E.checked_profile
