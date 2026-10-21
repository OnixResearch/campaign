# expect: source ids must be unique
let Contracts = import "../../contracts.ncl" in
let Entry = import "../valid-entry.ncl" in
{
  schema = Contracts.constants.schema_sources,
  sources = [Entry, Entry],
} | Contracts.Inventory
