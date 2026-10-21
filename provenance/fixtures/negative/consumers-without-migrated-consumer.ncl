# expect: consumers must include an in-repo fixture and a migrated consumer
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [Base.fixture],
} | Contracts.ConsumerRecord
