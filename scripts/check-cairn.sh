#!/bin/sh
set -eu

CAIRN=${CAIRN:-cairn}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target/campaign-infra-checker}

nickel typecheck cairn-policy/source-manifest.ncl
cargo run --manifest-path tools/campaign-infra-checker/Cargo.toml --locked -- cairn-policy-source --root .
$CAIRN policy export --check
$CAIRN validate --root . --policy cairn-policy/generated/cairn-policy.json --strict
$CAIRN traceability coverage --root . --policy cairn-policy/generated/cairn-policy.json --json

printf '%s\n' "campaign Cairn policy and lifecycle validation passed"
