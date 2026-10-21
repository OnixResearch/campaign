# expect: retained_authority lacks snapshot
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { retained_authority | force = std.array.filter (fun authority => authority != 'snapshot) Base.migrated.retained_authority },
  ],
} | Contracts.ConsumerRecord
