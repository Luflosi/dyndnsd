# SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
# SPDX-License-Identifier: AGPL-3.0-only

builder: crane: fenix:
final: prev: let
  system = prev.stdenv.hostPlatform.system;
  builder' = builder {
    inherit crane fenix system;
    pkgs = final;
  };
in {
  dyndnsd = builder'.dyndnsd-full;
}
