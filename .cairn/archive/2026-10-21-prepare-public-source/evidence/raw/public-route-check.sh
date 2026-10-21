#!/usr/bin/env bash
# Credential-isolated run of the README's documented public route on an exported revision.
# Usage: campaign-public-route-check.sh <repo> <revision> <tools-store-path> <nix-binary> <work-dir>
set -euo pipefail

repo=$1
revision=$2
tools=$3
nix_bin=$4
work=$5

mkdir -p "$work/src" "$work/home" "$work/cargo-home" "$work/rustup-home" "$work/bin" "$work/cache" "$work/config"
ln -sf "$nix_bin" "$work/bin/nix"
git -C "$repo" archive "$revision" | tar -x -C "$work/src"

run() {
  printf '\n$ %s\n' "$*"
  env -i \
    HOME="$work/home" \
    XDG_CACHE_HOME="$work/cache" \
    XDG_CONFIG_HOME="$work/config" \
    CARGO_HOME="$work/cargo-home" \
    RUSTUP_HOME="$work/rustup-home" \
    PATH="$work/cargo-home/bin:$tools/bin:$work/bin" \
    SSL_CERT_FILE="$tools/etc/ssl/certs/ca-bundle.crt" \
    NIX_SSL_CERT_FILE="$tools/etc/ssl/certs/ca-bundle.crt" \
    GIT_SSL_CAINFO="$tools/etc/ssl/certs/ca-bundle.crt" \
    GIT_CONFIG_NOSYSTEM=1 \
    GIT_CONFIG_GLOBAL=/dev/null \
    GIT_TERMINAL_PROMPT=0 \
    GIT_SSH_COMMAND='ssh -F /dev/null -o IdentityFile=/dev/null -o IdentitiesOnly=yes -o IdentityAgent=none -o BatchMode=yes' \
    NIX_CONFIG=$'access-tokens = \nexperimental-features = nix-command flakes' \
    CARGO_TERM_COLOR=never \
    TMPDIR="$work" \
    "$@"
}

cd "$work/src"
printf 'revision %s exported to %s\n' "$revision" "$work/src"
run sh -c 'env | sort | sed -E "s/=.*//" | tr "\n" " "; echo'
run sh -c 'for d in "$PWD" "$(dirname "$PWD")" "$(dirname "$(dirname "$PWD")")" /var /; do test -e "$d/.cargo" && echo "ancestor cargo config: $d/.cargo"; done; echo "ancestor .cargo scan done"'
run rustup --version
run rustup toolchain install
run rustup show active-toolchain
run cargo --version
run rustc --version
run cargo fmt --all -- --check
run cargo test --workspace --all-targets --all-features --locked
run cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
run cargo check -p campaign-core --no-default-features --target wasm32-unknown-unknown --locked
run cargo check -p campaign-choregraph --no-default-features --target wasm32-unknown-unknown --locked
run cargo check -p campaign-conformance --no-default-features --target wasm32-unknown-unknown --locked
run sh -c 'nix shell "github:NixOS/nixpkgs/$(jq -r '\''.nodes[.nodes.root.inputs.nixpkgs].locked.rev'\'' flake.lock)#nickel" -c sh -c "nickel --version && sh scripts/check-config.sh && sh scripts/check-boundaries.sh"'
run git clone --quiet https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git "$work/choregraph"
run git -C "$work/choregraph" checkout --quiet --detach b3e08e19750f53bdbcae970cdf58a47a791ed20b
run env CHOREGRAPH_HISTORY_SOURCE="$work/choregraph" sh -c 'nix shell "github:NixOS/nixpkgs/$(jq -r '\''.nodes[.nodes.root.inputs.nixpkgs].locked.rev'\'' flake.lock)#nickel" -c sh scripts/check-boundaries.sh'
printf '\ndocumented public-route checks passed for %s\n' "$revision"
