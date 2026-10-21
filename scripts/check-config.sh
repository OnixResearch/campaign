#!/bin/sh
set -eu

NICKEL=${NICKEL:-nickel}

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT INT TERM

$NICKEL typecheck config/contracts.ncl
$NICKEL typecheck config/repository.ncl
$NICKEL typecheck config/profiles/default.ncl
$NICKEL typecheck config/fixtures/edit.ncl

# Freshness: generated runtime values must equal a fresh export of their Nickel sources.
$NICKEL export --color never --format json config/repository.ncl > "$work_dir/repository.json"
$NICKEL export --color never --format json config/profiles/default.ncl > "$work_dir/default-profile.json"
cmp "$work_dir/repository.json" config/generated/repository.json \
  || fail "stale projection: config/generated/repository.json differs from config/repository.ncl"
cmp "$work_dir/default-profile.json" config/generated/default-profile.json \
  || fail "stale projection: config/generated/default-profile.json differs from config/profiles/default.ncl"
# The profile loader embeds a crate-local copy of the default profile; it must equal the checked projection.
cmp crates/campaign-profile/profiles/default.json config/generated/default-profile.json \
  || fail "stale crate copy: crates/campaign-profile/profiles/default.json differs from config/generated/default-profile.json"

positive_count=0
for fixture in config/fixtures/positive/*.ncl; do
  $NICKEL typecheck "$fixture"
  $NICKEL export --color never --format json "$fixture" > "$work_dir/positive.json" \
    || fail "positive fixture failed to export: $fixture"
  positive_count=$((positive_count + 1))
done
[ "$positive_count" -gt 0 ] || fail "no positive configuration fixtures found"

# Every negative fixture must parse and typecheck, then fail export with every line recorded in its `.expect`
# file, so a syntax error, a merge conflict, or an unrelated contract cannot masquerade as the intended violation.
negative_count=0
for fixture in config/fixtures/negative/*.ncl; do
  expect="${fixture%.ncl}.expect"
  [ -s "$expect" ] || fail "negative fixture has no expected contract violation: $expect"
  $NICKEL typecheck "$fixture" 2> "$work_dir/negative-typecheck.err" \
    || fail "negative fixture must typecheck before its contract runs: $fixture"
  if $NICKEL export --color never --format json "$fixture" > "$work_dir/negative.json" 2> "$work_dir/negative.err"; then
    fail "negative fixture unexpectedly passed: $fixture"
  fi
  expected_lines=0
  while IFS= read -r line || [ -n "$line" ]; do
    [ -n "$line" ] || continue
    grep -F -q -- "$line" "$work_dir/negative.err" || {
      cat "$work_dir/negative.err" >&2
      fail "negative fixture failed without expected contract violation '$line': $fixture"
    }
    expected_lines=$((expected_lines + 1))
  done < "$expect"
  [ "$expected_lines" -gt 0 ] || fail "negative fixture expectation is blank: $expect"
  negative_count=$((negative_count + 1))
done
[ "$negative_count" -gt 0 ] || fail "no negative configuration fixtures found"

for expect in config/fixtures/negative/*.expect; do
  [ -f "${expect%.expect}.ncl" ] || fail "orphan negative expectation without fixture: $expect"
done

printf '%s\n' "campaign configuration contracts and freshness passed ($positive_count positive, $negative_count negative fixtures)"
