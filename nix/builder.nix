# SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
# SPDX-License-Identifier: AGPL-3.0-only

{ crane
, fenix
, pkgs
, system
}:
rec {
  inherit (pkgs) lib;

  craneLib = crane.mkLib pkgs;
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
    meta = {
      description = "Simple but configurable web server for dynamic DNS updates";
      mainProgram = "dyndnsd";
    };
  });

  dyndnsd-systemd-unit = pkgs.runCommand "dyndnsd-systemd-unit" { } ''
    install --mode=444 -D '${../systemd/dyndnsd.service}' "$out/etc/systemd/system/dyndnsd.service"
  '';

  dyndnsd-full = pkgs.symlinkJoin {
    name = "dyndnsd-full";
    paths = [ dyndnsd dyndnsd-systemd-unit ];
    meta = {
      description = dyndnsd.meta.description + " (with systemd unit file)";
      inherit (dyndnsd.meta) mainProgram;
    };
  };
}
