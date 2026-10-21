let E = import "../edit.ncl" in
E.repository
|> E.set "license" "MIT OR Apache-2.0"
|> E.checked_repository
