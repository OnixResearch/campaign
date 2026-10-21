# expect: extra field `parity`
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture & { parity = Base.migrated.parity },
    Base.migrated,
  ],
} | Contracts.ConsumerRecord
