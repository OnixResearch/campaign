# The exploratory threshold accepts its inclusive lower edge.
let E = import "../edit.ncl" in
E.profile
|> E.set_in "selection" "exploratory_threshold_parts_per_million" 0
|> E.checked_profile
