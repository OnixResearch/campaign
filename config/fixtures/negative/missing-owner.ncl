let E = import "../edit.ncl" in
E.profile |> E.drop_in "owners" "stopping"
|> E.checked_profile
