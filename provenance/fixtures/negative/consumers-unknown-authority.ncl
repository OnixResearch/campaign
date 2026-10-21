# expect: contract broken by the value of `retained_authority`
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture & { retained_authority | force = ['execution, 'scheduling] },
    Base.migrated,
  ],
} | Contracts.ConsumerRecord
