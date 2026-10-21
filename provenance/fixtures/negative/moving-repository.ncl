# expect: contract broken by the value of `repository`
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [std.record.update "repository" "https://github.com/example/fixture/tree/main" Entry],
} | Contracts.Inventory
