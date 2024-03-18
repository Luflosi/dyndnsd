# SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
# SPDX-License-Identifier: AGPL-3.0-only

{ crane
, fenix
, pkgs
, system
}:
rec {
  inherit (pkgs) lib;

  craneLib = crane.lib.${system};
  src = craneLib.cleanCargoSource (craneLib.path ../.);

  # Common arguments can be set here to avoid repeating them later
  commonArgs = {
    inherit src;
    strictDeps = true;

    buildInputs = [
      # Add additional build inputs here
    ] ++ lib.optionals pkgs.stdenv.isDarwin [
      # Additional darwin specific inputs can be set here
      pkgs.libiconv
    ];

    # Additional environment variables can be set directly
    # MY_CUSTOM_VAR = "some value";
  };

  craneLibLLvmTools = craneLib.overrideToolchain
    (fenix.packages.${system}.complete.withComponents [
      "cargo"
      "llvm-tools"
      "rustc"
    ]);

  # Build *just* the cargo dependencies, so we can reuse
  # all of that work (e.g. via cachix) when running in CI
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  # Build the actual crate itself, reusing the dependency
  # artifacts from above.
  dyndnsd = craneLib.buildPackage (commonArgs // {
    inherit cargoArtifacts;
  });
}
