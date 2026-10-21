let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_prune_batch" 65537
|> E.checked_profile
