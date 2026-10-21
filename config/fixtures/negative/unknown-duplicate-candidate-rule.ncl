let E = import "../edit.ncl" in
E.profile |> E.set_in "admission" "duplicate_candidate" 'merge
|> E.checked_profile
