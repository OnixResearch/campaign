let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.fixture & { id | force = "fixture-other-pilot" },
    Base.migrated & { retained_authority | force = Base.migrated.retained_authority @ ['promotion] },
  ],
} | Contracts.ConsumerRecord
