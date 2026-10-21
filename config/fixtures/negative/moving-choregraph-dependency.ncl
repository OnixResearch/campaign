let E = import "../edit.ncl" in
E.repository
|> E.set_in "choregraph" "history_revision" "main"
|> E.checked_repository
