# SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Build dyndnsd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ self.outputs.overlays.dyndnsd ];
        };

        builder = import ./nix/builder.nix { inherit crane fenix pkgs system; };
        inherit (builder)
          lib
          craneLib
          src
          commonArgs
          craneLibLLvmTools
          cargoArtifacts
          dyndnsd
          dyndnsd-full
        ;
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit dyndnsd;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          dyndnsd-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          dyndnsd-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          dyndnsd-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          dyndnsd-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          dyndnsd-deny = craneLib.cargoDeny {
            inherit src;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `dyndnsd` if you do not want
          # the tests to run twice
          dyndnsd-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          dyndnsd-reuse = pkgs.runCommand "run-reuse" {
            src = ./.;
            nativeBuildInputs = with pkgs; [ reuse ];
          } ''
            cd "$src"
            reuse lint
            touch "$out"
          '';

        # NixOS tests don't run on macOS
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          dyndnsd-e2e-test = pkgs.testers.runNixOSTest (import ./nix/e2e-test.nix self);
        };

        packages = {
          dyndnsd = dyndnsd-full;
          default = self.packages.${system}.dyndnsd;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          dyndnsd-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.dyndnsd = flake-utils.lib.mkApp {
          drv = dyndnsd;
        };
        apps.default = self.apps.${system}.dyndnsd;

        devShells.dyndnsd = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            # pkgs.ripgrep
          ];
        };
        devShells.default = self.devShells.${system}.dyndnsd;
      }) // {
        nixosModules.dyndnsd = import ./nix/module.nix;
        nixosModules.default = self.nixosModules.dyndnsd;

        overlays.dyndnsd = import ./nix/overlay.nix (import ./nix/builder.nix) crane fenix;
        overlays.default = self.overlays.dyndnsd;
      };
}
