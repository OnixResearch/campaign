let E = import "../edit.ncl" in
E.profile |> E.set "schema" "campaign.profile.v1"
|> E.checked_profile
