let E = import "../edit.ncl" in
E.repository
|> E.set_in "choregraph" "dependency_status" "moving-branch"
|> E.checked_repository
