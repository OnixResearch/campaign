let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_entropy_value" 4294967295
|> E.checked_profile
