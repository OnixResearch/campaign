#!/bin/sh
set -eu

for required_file in \
  Cargo.lock \
  crates/campaign-core/src/lib.rs \
  crates/campaign-choregraph/src/lib.rs \
  crates/campaign-conformance/src/lib.rs
 do
  if [ ! -f "$required_file" ]; then
    echo "required Rust gate input is missing: $required_file" >&2
    exit 1
  fi
done

if [ -z "${OCTET_DENY_ALL_HOOK:-}" ]; then
  echo "OCTET_DENY_ALL_HOOK must point to the pinned octet-deny-all hook" >&2
  exit 1
fi

export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target/campaign-infra-check}
export OCTET_PRECOMMIT_USE_INSTALLED=${OCTET_PRECOMMIT_USE_INSTALLED:-true}

cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo check -p campaign-core --no-default-features --target wasm32-unknown-unknown --locked
cargo check -p campaign-choregraph --no-default-features --target wasm32-unknown-unknown --locked
cargo check -p campaign-conformance --no-default-features --target wasm32-unknown-unknown --locked
"$OCTET_DENY_ALL_HOOK" --workspace -- --all-targets --all-features

printf '%s\n' "campaign Rust, no_std, Clippy, and Octet deny-all gates passed"
