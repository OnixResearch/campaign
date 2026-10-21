# expect: must be a positive integer
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture & { conformance.fixtures | force = 0 },
    Base.migrated & { conformance.fixtures | force = 0 },
  ],
} | Contracts.ConsumerRecord
