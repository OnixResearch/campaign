let E = import "../edit.ncl" in
E.profile |> E.set_in "versions" "campaign_schema" 2
|> E.checked_profile
