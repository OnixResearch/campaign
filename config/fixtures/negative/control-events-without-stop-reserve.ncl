let E = import "../edit.ncl" in
E.profile |> E.set_in "bounds" "maximum_control_events" 1
|> E.checked_profile
