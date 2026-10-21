# expect: agreeing_decision_classes lacks budget_use
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { parity.agreeing_decision_classes | force = std.array.filter (fun agreement => agreement.class != 'budget_use) Base.migrated.parity.agreeing_decision_classes },
  ],
} | Contracts.ConsumerRecord
