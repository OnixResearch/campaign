# Fixture edits build fresh field values with `std.record.update`/`std.record.remove`, never conflicting merges, so
# every fixture reaches the contract under test instead of failing on a merge conflict.
let Contracts = import "../contracts.ncl" in
let set_nested = fun group field value profile =>
  std.record.update group (std.record.update field value (std.record.get group profile)) profile
in
let drop_nested = fun group field profile =>
  std.record.update group (std.record.remove field (std.record.get group profile)) profile
in
{
  profile = import "../profiles/default.ncl",
  repository = import "../repository.ncl",
  set = fun field value record => std.record.update field value record,
  set_in = set_nested,
  drop_in = drop_nested,
  as_ranked = fun profile =>
    set_nested "selection" "mode" 'ranked profile
    |> drop_nested "selection" "exploratory_threshold_parts_per_million"
    |> drop_nested "selection" "exploration_order",
  checked_profile = fun profile => profile | Contracts.Profile,
  checked_repository = fun repository => repository | Contracts.Repository,
}
