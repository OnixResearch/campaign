# expect: contract broken by the value of `revision`
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [std.record.update "revision" "0123456789ABCDEF0123456789ABCDEF01234567" Entry],
} | Contracts.Inventory
