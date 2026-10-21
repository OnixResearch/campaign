# expect: cargo and nix Campaign pins must be equal
let Contracts = import "../../contracts.ncl" in
let Base = import "../valid-consumers.ncl" in
{
  schema = Contracts.constants.schema_consumers,
  reordered_profile = Base.reordered_profile,
  consumers = [
    Base.fixture,
    Base.migrated & { pins.nix_campaign_revision | force = "3333333333333333333333333333333333333333" },
  ],
} | Contracts.ConsumerRecord
