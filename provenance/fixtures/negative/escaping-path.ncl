# expect: contract broken by the value of `campaign_paths`
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [std.record.update "campaign_paths" ["../outside"] Entry],
} | Contracts.Inventory
