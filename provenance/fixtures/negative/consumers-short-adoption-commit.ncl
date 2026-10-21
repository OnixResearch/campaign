# expect: contract broken by the value of `commit`
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { adoption.commit | force = "0123456" },
  ],
} | Contracts.ConsumerRecord
