let SchemaSources = "campaign.provenance.v1" in
let NoAssertion = "NOASSERTION" in
let NonEmpty = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.length value > 0
) in
let SourceId = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[a-z0-9]+(-[a-z0-9]+)*$" value
) in
let FullRevision = std.contract.from_predicate (fun value =>
  std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value
) in
let ImmutableRepository = std.contract.from_predicate (fun value =>
  std.is_string value
  && std.string.is_match "^(https://[^?#[:space:]]+|rad:z[1-9A-HJ-NP-Za-km-z]+)$" value
  && !(std.string.is_match "/(tree|blob|commits?|releases)/" value)
) in
let RelativePath = std.contract.from_predicate (fun value =>
  std.is_string value
  && std.string.length value > 0
  && !(std.string.is_match "(^/|^\\.\\.?(/|$)|/\\.\\.?(/|$)|//)" value)
) in
let NonEmptyArray = fun element =>
  std.contract.Sequence [
    Array element,
    std.contract.from_validator (fun values =>
      if std.array.length values > 0 then 'Ok else 'Error { message = "array must not be empty" }
    ),
  ]
in
let Adoption = [| 'dependency, 'vendored_copy, 'reference_only |] in
let EntryFields = {
  id | SourceId,
  repository | ImmutableRepository,
  revision | FullRevision,
  license | NonEmpty,
  owner | NonEmpty,
  adoption | Adoption,
  reviewed_files | NonEmptyArray RelativePath,
  adopted_mechanisms | NonEmptyArray NonEmpty,
  campaign_paths | NonEmptyArray RelativePath,
  retained_authority | NonEmpty,
  non_adoption | NonEmptyArray NonEmpty,
  notes | Array NonEmpty,
} in
# An unknown license is recorded as SPDX NOASSERTION and is admitted only for
# reference-only sources that transfer no code.
let LicenseKnownForTransfer = std.contract.from_validator (fun entry =>
  if entry.license != NoAssertion || entry.adoption == 'reference_only then
    'Ok
  else
    'Error { message = "license NOASSERTION is admitted only for reference_only sources" }
) in
let Entry = std.contract.Sequence [EntryFields, LicenseKnownForTransfer] in
let UniqueIds = std.contract.from_validator (fun entries =>
  let ids = std.array.map (fun entry => entry.id) entries in
  if std.array.all
    (fun id => std.array.length (std.array.filter (fun other => other == id) ids) == 1)
    ids
  then
    'Ok
  else
    'Error { message = "source ids must be unique" }
) in

# Consumer pilot evidence (`campaign.consumer_pilots.valid`, `campaign.chaoscontrol_migration`).
let SchemaConsumers = "campaign.consumers.v1" in
let Blake3Digest = std.contract.from_validator (fun value =>
  if std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value then
    'Ok
  else
    'Error { message = "must be a lowercase 64-hex BLAKE3 digest" }
) in
let PositiveInteger = std.contract.from_validator (fun value =>
  if std.is_number value && std.number.is_integer value && value > 0 then
    'Ok
  else
    'Error { message = "must be a positive integer" }
) in
let NaturalInteger = std.contract.from_validator (fun value =>
  if std.is_number value && std.number.is_integer value && value >= 0 then
    'Ok
  else
    'Error { message = "must be a non-negative integer" }
) in
let repeats_entry = fun values =>
  !(std.array.all
    (fun value => std.array.length (std.array.filter (fun other => other == value) values) == 1)
    values)
in
# `None` when `values` names every enum tag in `required`, else the error naming the missing tags.
let missing_tags_error = fun field required values =>
  let missing = std.array.filter (fun tag => !(std.array.elem tag values)) required in
  if missing == [] then
    'None
  else
    'Some { message = "%{field} lacks %{std.string.join ", " (std.array.map std.string.from_enum missing)}" }
in
let Unique = fun field =>
  std.contract.from_validator (fun values =>
    if repeats_entry values then 'Error { message = "%{field} must not repeat an entry" } else 'Ok
  )
in
let Covers = fun field required =>
  std.contract.from_validator (fun values =>
    missing_tags_error field required values
    |> match {
      'None => 'Ok,
      'Some error => 'Error error,
    }
  )
in
let Authority = [|
  'execution,
  'snapshot,
  'coverage,
  'fault,
  'assertion,
  'finding,
  'replay,
  'minimization,
  'evidence,
  'promotion,
  'acceptance
|] in
# Authority a migrated consumer keeps outside Campaign (`campaign.chaoscontrol_migration.authority`).
let MigrationAuthorities = [
  'execution,
  'snapshot,
  'coverage,
  'fault,
  'assertion,
  'finding,
  'replay,
  'minimization,
  'evidence,
] in
let DecisionClass = [|
  'candidate_order,
  'selected_parents,
  'priority_results,
  'pruning,
  'budget_use,
  'stop_classes
|] in
# Decision classes the migration parity corpus must compare (`campaign.chaoscontrol_migration.parity`).
let RequiredDecisionClasses = [
  'candidate_order,
  'selected_parents,
  'priority_results,
  'pruning,
  'budget_use,
  'stop_classes,
] in
let Conformance = {
  fixtures | PositiveInteger,
  transcript_digest | Blake3Digest,
  reordered_profile_digest | Blake3Digest,
  command | NonEmpty,
} in
let Pins = std.contract.Sequence [
  {
    cargo_campaign_revision | FullRevision,
    nix_campaign_revision | FullRevision,
    choregraph_revision | FullRevision,
  },
  std.contract.from_validator (fun pins =>
    if pins.cargo_campaign_revision == pins.nix_campaign_revision then
      'Ok
    else
      'Error { message = "cargo and nix Campaign pins must be equal" }
  ),
] in
let Adoption = {
  change | SourceId,
  branch | NonEmpty,
  commit | FullRevision,
} in
# A file in the consumer repository identified by its content digest.
let Artifact = {
  path | RelativePath,
  blake3 | Blake3Digest,
} in
let DecisionAgreement = {
  class | DecisionClass,
  compared_fields | NonEmptyArray NonEmpty,
} in
let Parity = {
  corpus_tests | RelativePath,
  receipt | Artifact,
  cutover_ready
    | std.contract.from_validator (fun ready =>
      if ready == true then 'Ok else 'Error { message = "parity must be cutover_ready" }
    ),
  agreeing_decision_classes
    | std.contract.Sequence [
      NonEmptyArray DecisionAgreement,
      std.contract.from_validator (fun agreements =>
        let classes = std.array.map (fun agreement => agreement.class) agreements in
        if repeats_entry classes then
          'Error { message = "agreeing_decision_classes must not repeat an entry" }
        else
          missing_tags_error "agreeing_decision_classes" RequiredDecisionClasses classes
          |> match {
            'None => 'Ok,
            'Some error => 'Error error,
          }
      ),
    ],
} in
let KvmBounds = {
  vms | PositiveInteger,
  rounds | PositiveInteger,
  branches | PositiveInteger,
  ticks | PositiveInteger,
  seed | NaturalInteger,
  mode | NonEmpty,
  memory_mb | PositiveInteger,
} in
let KvmSmoke = {
  record | Artifact,
  receipt | Artifact,
  bounds | KvmBounds,
} in
let Authorities = std.contract.Sequence [NonEmptyArray Authority, Unique "retained_authority"] in
# The in-repo non-VM pilot: a Campaign conformance adapter and the tests that run it.
let InRepoFixture = {
  id | SourceId,
  kind | [| 'in_repo_fixture |],
  adapter | RelativePath,
  tests | NonEmptyArray RelativePath,
  conformance | Conformance,
  retained_authority | Authorities,
  non_claims | NonEmptyArray NonEmpty,
} in
# A consumer migrated from its own exploration policy onto a pinned Campaign revision.
let MigratedConsumer = {
  id | SourceId,
  kind | [| 'migrated_consumer |],
  repository | ImmutableRepository,
  adoption | Adoption,
  pins | Pins,
  conformance | Conformance,
  parity | Parity,
  kvm_smoke | KvmSmoke,
  retained_authority
    | std.contract.Sequence [Authorities, Covers "retained_authority" MigrationAuthorities],
  non_claims | NonEmptyArray NonEmpty,
} in
let Consumer = std.contract.custom (fun label value =>
  if std.is_record value && std.record.has_field "kind" value then
    value.kind
    |> match {
      'in_repo_fixture => 'Ok (std.contract.apply InRepoFixture label value),
      'migrated_consumer => 'Ok (std.contract.apply MigratedConsumer label value),
      _ => 'Error { message = "consumer kind must be in_repo_fixture or migrated_consumer" },
    }
  else
    'Error { message = "consumer lacks a kind" }
) in
let UniqueConsumerIds = std.contract.from_validator (fun consumers =>
  let ids = std.array.map (fun consumer => consumer.id) consumers in
  if std.array.all
    (fun id => std.array.length (std.array.filter (fun other => other == id) ids) == 1)
    ids
  then
    'Ok
  else
    'Error { message = "consumer ids must be unique" }
) in
let BothPilotKinds = std.contract.from_validator (fun consumers =>
  let kinds = std.array.map (fun consumer => consumer.kind) consumers in
  if std.array.elem 'in_repo_fixture kinds && std.array.elem 'migrated_consumer kinds then
    'Ok
  else
    'Error { message = "consumers must include an in-repo fixture and a migrated consumer" }
) in
let RankOrder = std.contract.Sequence [NonEmptyArray NaturalInteger, Unique "rank order"] in
{
  constants = {
    schema_sources = SchemaSources,
    schema_consumers = SchemaConsumers,
    no_assertion = NoAssertion,
  },
  Inventory = {
    schema | std.contract.Equal SchemaSources,
    sources | std.contract.Sequence [NonEmptyArray Entry, UniqueIds],
  },
  ConsumerRecord = {
    schema | std.contract.Equal SchemaConsumers,
    reordered_profile | {
      exploration_order | RankOrder,
      retention_rank_order | RankOrder,
    },
    consumers
      | std.contract.Sequence [
        NonEmptyArray Consumer,
        UniqueConsumerIds,
        BothPilotKinds,
        # Each pilot's fixture count and digests are compared with the constants pinned in
        # campaign-conformance's `tests::consumers` by `campaign-infra-checker boundaries`.
      ],
  },
}
