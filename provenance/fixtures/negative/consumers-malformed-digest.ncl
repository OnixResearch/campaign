# expect: must be a lowercase 64-hex BLAKE3 digest
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { parity.receipt.blake3 | force = "0000000000000000000000000000000000000000000000000000000000000ABC" },
  ],
} | Contracts.ConsumerRecord
