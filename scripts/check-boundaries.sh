#!/bin/sh
set -eu

export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target/campaign-infra-checker}
NICKEL=${NICKEL:-nickel}

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT INT TERM

$NICKEL typecheck provenance/sources.ncl
$NICKEL export --format json provenance/sources.ncl > "$work_dir/sources.json"
cmp "$work_dir/sources.json" provenance/generated/sources.json
$NICKEL typecheck provenance/consumers.ncl
$NICKEL export --format json provenance/consumers.ncl > "$work_dir/consumers.json"
cmp "$work_dir/consumers.json" provenance/generated/consumers.json

for fixture in provenance/fixtures/positive/*.ncl; do
  $NICKEL export --format json "$fixture" > "$work_dir/positive.json"
done

for fixture in provenance/fixtures/negative/*.ncl; do
  expected=$(sed -n 's/^# expect: //p' "$fixture")
  if [ -z "$expected" ]; then
    echo "negative provenance fixture lacks an expected contract error: $fixture" >&2
    exit 1
  fi
  if $NICKEL export --format json "$fixture" > "$work_dir/negative.json" 2> "$work_dir/negative.err"; then
    echo "negative provenance fixture unexpectedly passed: $fixture" >&2
    exit 1
  fi
  if ! grep -F -q -- "$expected" "$work_dir/negative.err"; then
    echo "negative provenance fixture failed without the expected contract error: $fixture" >&2
    cat "$work_dir/negative.err" >&2
    exit 1
  fi
done

cargo test --manifest-path tools/campaign-infra-checker/Cargo.toml --locked
cargo run --manifest-path tools/campaign-infra-checker/Cargo.toml --locked -- boundaries --root .

printf '%s\n' "campaign provenance, dependency, and boundary checks passed"
