# expect: missing definition for `retained_authority`
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [std.record.remove "retained_authority" Entry],
} | Contracts.Inventory
