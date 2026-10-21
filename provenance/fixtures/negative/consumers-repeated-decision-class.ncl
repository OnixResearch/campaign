# expect: agreeing_decision_classes must not repeat an entry
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { parity.agreeing_decision_classes | force = Base.migrated.parity.agreeing_decision_classes @ [{ class = 'pruning, compared_fields = ["Admitted"] }] },
  ],
} | Contracts.ConsumerRecord
