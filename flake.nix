{
  description = "Campaign adaptive exploration policy infrastructure";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    cairn = {
      url = "github:OnixResearch/cairn/5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    octet = {
      url = "github:OnixResearch/octet/890f424cd7b1b2015f39c0eb5747f1bd1e576028";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
    choregraph-history = {
      url = "git+https://git.onix.computer/zL2ncTUeASVYwcoGkEXv9JKgGbAF.git?rev=b3e08e19750f53bdbcae970cdf58a47a791ed20b";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      cairn,
      octet,
      choregraph-history,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      eachSystem = function: nixpkgs.lib.genAttrs systems (system: function system);
    in
    {
      checks = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
          octetDenyAllHook = "${octet}/hooks/octet-deny-all.sh";
          cairnPolicySourceRoot = "${cairn}/cairn-policy";
          campaignCargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "choregraph-history-0.1.0" = "sha256-BO/AlwJBIuW+WXGRQgd2LymvGIYtK0fqd3YYMDtfGBw=";
            };
          };
          campaignCargoVendorDir = rustPlatform.importCargoLock campaignCargoLock;
          prepareOfflineCargo = ''
            export HOME="$TMPDIR/home"
            export CARGO_HOME="$TMPDIR/cargo-home"
            export CARGO_TARGET_DIR="$TMPDIR/cargo-target"
            export CARGO_NET_OFFLINE=true
            mkdir -p "$HOME" "$CARGO_HOME"
            cp -Lr --reflink=auto "${campaignCargoVendorDir}" ../cargo-vendor-dir
            chmod -R u+w ../cargo-vendor-dir
            cp "${campaignCargoVendorDir}/.cargo/config.toml" "$CARGO_HOME/config.toml"
          '';
        in
        {
          nickel-config =
            pkgs.runCommand "campaign-nickel-config"
              {
                nativeBuildInputs = [ pkgs.nickel ];
                src = self;
              }
              ''
                set -eu
                cp -R --no-preserve=mode "$src" source
                cd source
                sh scripts/check-config.sh
                touch "$out"
              '';

          dependency-boundaries =
            pkgs.runCommand "campaign-dependency-boundaries"
              {
                nativeBuildInputs = [
                  toolchain
                  pkgs.nickel
                ];
                src = self;
                CHOREGRAPH_HISTORY_SOURCE = "${choregraph-history}";
                CAIRN_POLICY_SOURCE_ROOT = cairnPolicySourceRoot;
              }
              ''
                set -eu
                cp -R --no-preserve=mode "$src" source
                cd source
                test -d "$CHOREGRAPH_HISTORY_SOURCE"
                test -f "$CHOREGRAPH_HISTORY_SOURCE/crates/choregraph-history/Cargo.toml"
                ${prepareOfflineCargo}
                sh scripts/check-boundaries.sh
                touch "$out"
              '';

          cairn-policy =
            pkgs.runCommand "campaign-cairn-policy"
              {
                nativeBuildInputs = [
                  cairn.packages.${system}.cairn
                  pkgs.nickel
                  toolchain
                ];
                src = self;
                CAIRN_POLICY_SOURCE_ROOT = cairnPolicySourceRoot;
              }
              ''
                set -eu
                cp -R --no-preserve=mode "$src" source
                cd source
                ${prepareOfflineCargo}
                sh scripts/check-cairn.sh
                touch "$out"
              '';

          local-checker =
            pkgs.runCommand "campaign-local-checker"
              {
                nativeBuildInputs = [
                  cairn.packages.${system}.cairn
                  octet.packages.${system}.cargo-octet
                  pkgs.nickel
                  toolchain
                ];
                src = self;
                CAIRN_POLICY_SOURCE_ROOT = cairnPolicySourceRoot;
                OCTET_DENY_ALL_HOOK = octetDenyAllHook;
                OCTET_PRECOMMIT_USE_INSTALLED = "true";
              }
              ''
                set -eu
                cp -R --no-preserve=mode "$src" source
                cd source
                ${prepareOfflineCargo}
                sh scripts/check.sh
                touch "$out"
              '';

          cargo-format =
            pkgs.runCommand "campaign-cargo-format"
              {
                nativeBuildInputs = [ toolchain ];
                src = self;
              }
              ''
                set -eu
                cp -R --no-preserve=mode "$src" source
                cd source
                cargo fmt --all -- --check
                touch "$out"
              '';

          rust-workspace = rustPlatform.buildRustPackage {
            pname = "campaign-rust-workspace";
            version = "0.1.0";
            src = self;
            cargoLock = campaignCargoLock;
            cargoBuildFlags = [
              "--workspace"
              "--all-targets"
              "--all-features"
            ];
            cargoTestFlags = [
              "--workspace"
              "--all-targets"
              "--all-features"
            ];
            postCheck = ''
              cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
              cargo check -p campaign-core --no-default-features --target wasm32-unknown-unknown --offline
              cargo check -p campaign-choregraph --no-default-features --target wasm32-unknown-unknown --offline
              cargo check -p campaign-conformance --no-default-features --target wasm32-unknown-unknown --offline
            '';
            installPhase = ''
              touch "$out"
            '';
          };

          octet-deny-all =
            pkgs.runCommand "campaign-octet-deny-all"
              {
                nativeBuildInputs = [
                  octet.packages.${system}.cargo-octet
                  toolchain
                ];
                src = self;
                OCTET_DENY_ALL_HOOK = octetDenyAllHook;
                OCTET_PRECOMMIT_USE_INSTALLED = "true";
              }
              ''
                set -eu
                cp -R --no-preserve=mode "$src" source
                cd source
                ${prepareOfflineCargo}
                "$OCTET_DENY_ALL_HOOK" --workspace -- --all-targets --all-features
                touch "$out"
              '';
        }
      );

      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          default = pkgs.mkShell {
            OCTET_DENY_ALL_HOOK = "${octet}/hooks/octet-deny-all.sh";
            OCTET_PRECOMMIT_USE_INSTALLED = "true";
            CAIRN_POLICY_SOURCE_ROOT = "${cairn}/cairn-policy";
            CHOREGRAPH_HISTORY_SOURCE = "${choregraph-history}";
            packages = [
              toolchain
              cairn.packages.${system}.cairn
              octet.packages.${system}.cargo-octet
              pkgs.b3sum
              pkgs.git
              pkgs.jq
              pkgs.nickel
              pkgs.nixfmt
              pkgs.pre-commit
            ];
          };
        }
      );
    };
}
