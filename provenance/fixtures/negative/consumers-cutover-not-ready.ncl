# expect: parity must be cutover_ready
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { parity.cutover_ready | force = false },
  ],
} | Contracts.ConsumerRecord
