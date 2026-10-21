let E = import "../edit.ncl" in
E.profile |> E.set_in "stopping" "finding_limit" 4097
|> E.checked_profile
