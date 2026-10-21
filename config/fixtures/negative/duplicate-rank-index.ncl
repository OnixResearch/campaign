let E = import "../edit.ncl" in
E.profile |> E.set_in "retention" "rank_order" [0, 1, 0]
|> E.checked_profile
