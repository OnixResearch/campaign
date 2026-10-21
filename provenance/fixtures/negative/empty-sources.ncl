# expect: array must not be empty
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [],
} | Contracts.Inventory
