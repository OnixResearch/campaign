let E = import "../edit.ncl" in
E.profile |> E.set "receipt_non_claims" (std.array.filter (fun claim => claim != 'safe_deletion) E.profile.receipt_non_claims)
|> E.checked_profile
