# expect: license NOASSERTION is admitted only for reference_only sources
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [std.record.update "adoption" 'dependency (std.record.update "license" Contracts.constants.no_assertion Entry)],
} | Contracts.Inventory
