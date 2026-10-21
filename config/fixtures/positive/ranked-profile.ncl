# Ranked mode without exploration fields, subtractive decay, an explicit finding limit, and coalescing admission.
let E = import "../edit.ncl" in
E.profile
|> E.as_ranked
|> E.set_in "selection" "decay" { rule = 'subtract_from_first_rank, penalty_per_selection = 1 }
|> E.set_in "stopping" "finding_limit" 1
|> E.set_in "admission" "duplicate_candidate" 'coalesce
|> E.checked_profile
