let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_control_events" 18446744073709551616
|> E.checked_profile
