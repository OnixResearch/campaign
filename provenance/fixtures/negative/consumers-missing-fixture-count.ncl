# expect: missing definition for `fixtures`
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture & { conformance | force = std.record.remove "fixtures" Base.fixture.conformance },
    Base.migrated,
  ],
} | Contracts.ConsumerRecord
