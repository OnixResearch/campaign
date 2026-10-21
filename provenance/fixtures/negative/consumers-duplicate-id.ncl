# expect: consumer ids must be unique
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { id | force = Base.fixture.id },
  ],
} | Contracts.ConsumerRecord
