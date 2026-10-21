# expect: contract broken by the value of `revision`
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [std.record.update "revision" "b3e08e1" Entry],
} | Contracts.Inventory
