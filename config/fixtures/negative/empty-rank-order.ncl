let E = import "../edit.ncl" in
E.profile |> E.set_in "retention" "rank_order" []
|> E.checked_profile
