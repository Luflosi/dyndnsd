# SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
# SPDX-License-Identifier: AGPL-3.0-only

{ config, lib, pkgs, ... }:

let
  cfg = config.services.dyndnsd;
  settingsFormat = pkgs.formats.toml { };
  RuntimeDirectory = "dyndnsd";

  userOpts = { lib, name, ... }: {
    options = {
      hash = lib.mkOption {
        type = lib.types.str;
        example = "$argon2id$v=19$m=65536,t=3,p=1$ZFRHDlJOQ3UNQRN7em14R08FIRE$0SqSQRj45ZBz1MfCPq9DVMWt7VSl96m7XtW6maIcUB0";
        description = lib.mdDoc ''
          The encoded Argon2 password hash for the user.
          To generate the password hash with a strong salt and without leaving the password in the shell history, execute
          `nix run nixpkgs#libargon2 -- "$(LC_ALL=C tr -dc '[:print:][:cntrl:]' </dev/urandom | head -c 20)" -id -m 16`
          then type the password, then press Ctrl-d twice.
        '';
      };
      domains = lib.mkOption {
        type = lib.types.attrsOf (lib.types.submodule domainOpts);
        default = {};
        description = lib.mdDoc "Attribute set of domains this user updates after authenticating.";
        example = {
          "example.com" = {
            ttl = 60;
            ipv6prefixlen = 48;
            ipv6suffix = "0:0:0:1::5";
          };
        };
      };
    };
  };

  domainOpts = { lib, name, ... }: {
    options = {
      ttl = lib.mkOption {
        type = lib.types.ints.positive;
        default = 60;
        description = lib.mdDoc ''
          The TTL of the DNS record.
          See `update_program.ipv4.stdin` and `update_program.ipv6.stdin`.
        '';
      };
      ipv6prefixlen = lib.mkOption {
        type = lib.types.ints.between 0 128;
        default = 128;
        example = 56;
        description = lib.mdDoc ''
          Change this option if you want to update the IPv6 address of a host other than the one which is making the request.
          Use this for example if you run the DNS update client on your router but want to update the IP address of a separate server in your lokal network.
          In this example, your router gets assigned a dynamic prefix from your ISP periodically but the part of the IPv6 address which is under your control (the host part) always stays the same.
          In order for the host part to always stay the same, you must either assigh the server an IPv6 address via DHCPv6 and set the host part statically in your router
          or change the network settings on your server to not randomly generate new addresses.
          The IPv6 address in the URL query parameter is spliced together with the IPv6 address from the `ipv6suffix` option
          by taking the first `ipv6prefixlen` bits from the IPv6 address from the URL query parameter with the last 128 - `ipv6prefixlen` bits from the `ipv6suffix` option.
          If you do not want to update the IPv6 address, set this to 0.
        '';
      };
      ipv6suffix = lib.mkOption {
        type = lib.types.str;
        default = "::";
        example = "0:0:0:1::5";
        description = lib.mdDoc ''
          Change this option if you want to update the IPv6 address of a host other than the one which is making the request.
          Use this for example if you run the DNS update client on your router but want to update the IP address of a separate server in your lokal network.
          In this example, your router gets assigned a dynamic prefix from your ISP periodically but the part of the IPv6 address which is under your control (the host part) always stays the same.
          In order for the host part to always stay the same, you must either assigh the server an IPv6 address via DHCPv6 and set the host part statically in your router
          or change the network settings on your server to not randomly generate new addresses.
          The IPv6 address in the URL query parameter is spliced together with the IPv6 address from the `ipv6suffix` option
          by taking the first `ipv6prefixlen` bits from the IPv6 address from the URL query parameter with the last 128 - `ipv6prefixlen` bits from the `ipv6suffix` option.
        '';
      };
    };
  };

in
{
  options = {
    services.dyndnsd = {
      enable = lib.mkEnableOption (lib.mdDoc "the DynDNS server");

      settings = {
        listen = {
          ip = lib.mkOption {
            type = lib.types.str;
            default = "::1";
            example = "::";
            description = lib.mdDoc ''
              Only listen to incoming requests on a specific IP address.
              The default is to listen on IPv6 localhost.
              The special address :: will listen on all IPv4 and IPv6 addresses.
            '';
          };
          port = lib.mkOption {
            type = lib.types.port;
            default = 9841;
            description = lib.mdDoc ''
              The port on which to listen.
            '';
          };
        };

        update_program = {
          bin = lib.mkOption {
            type = lib.types.path;
            default = "${pkgs.coreutils}/bin/false";
            example = lib.literalExpression ''"''${pkgs.dig.dnsutils}/bin/nsupdate"'';
            description = lib.mdDoc ''
              Path to a program which will be used to forward the updated (dynamic) IP addresses to the actual DNS server.
            '';
          };
          args = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [];
            example = [ "-k" "/etc/bind/ddns.key" ];
            description = lib.mdDoc ''
              Command line arguments the update program will be called with.
            '';
          };
          stdin_per_zone_update = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "send\n";
            description = lib.mdDoc ''
              String to send to the stdin of the update program after each zone (domain) was updated.
            '';
          };
          final_stdin = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "quit\n";
            description = lib.mdDoc ''
              String to send to the stdin of the update program when we're done.
            '';
          };
          ipv4 = {
            stdin = lib.mkOption {
              type = lib.types.str;
              default = "";
              example = "update delete {domain}. IN A\nupdate add {domain}. {ttl} IN A {ipv4}\n";
              description = lib.mdDoc ''
                String template to send to the stdin of the update program for updating the IPv4 DNS record.
                The three different variables are replaced with the appropriate values before the string is sent to the update program.
              '';
            };
          };
          ipv6 = {
            stdin = lib.mkOption {
              type = lib.types.str;
              default = "";
              example = "update delete {domain}. IN AAAA\nupdate add {domain}. {ttl} IN AAAA {ipv6}\n";
              description = lib.mdDoc ''
                String template to send to the stdin of the update program for updating the IPv6 DNS record.
                The three different variables are replaced with the appropriate values before the string is sent to the update program.
              '';
            };
          };
        };

        users = lib.mkOption {
          type = lib.types.attrsOf (lib.types.submodule userOpts);
          default = {};
          description = lib.mdDoc "Attribute set of users.";
          example = {
            alice = {
              hash = "$argon2id$v=19$m=65536,t=3,p=1$ZFRHDlJOQ3UNQRN7em14R08FIRE$0SqSQRj45ZBz1MfCPq9DVMWt7VSl96m7XtW6maIcUB0";
              domains = {
                "example.com" = {
                  ttl = 60;
                  ipv6prefixlen = 48;
                  ipv6suffix = "0:0:0:1::5";
                };
              };
            };
          };
        };
      };

      environmentFiles = lib.mkOption {
        default = [];
        type = lib.types.listOf lib.types.path;
        example = [ "/run/secrets/dyndnsd.env" ];
        description = lib.mdDoc ''
          Files to load as environment file. Environment variables from this file
          will be substituted into the static configuration file using [envsubst](https://github.com/a8m/envsubst).
          If you change this option, make sure to either not have any password hashes in the configuration
          or replace all the `$` with `$$` since `envsubst` would otherwise delete parts of the password hash.
        '';
      };
    };
  };


  config = lib.mkIf cfg.enable {
    systemd.packages = [ pkgs.dyndnsd ];

    systemd.services.dyndnsd = {
      description = "Service that updates a dynamic DNS record";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      startLimitBurst = 1;

      serviceConfig = let
        settingsFile = settingsFormat.generate "dyndnsd.toml" cfg.settings;
        runtimeConfigPath = if cfg.environmentFiles != []
          then "/run/${RuntimeDirectory}/dyndnsd.toml"
          else settingsFile;
      in {
        inherit RuntimeDirectory;
        EnvironmentFile = cfg.environmentFiles;
        ExecStartPre = lib.mkIf (cfg.environmentFiles != []) [ "'${pkgs.envsubst}/bin/envsubst' -no-unset -i '${settingsFile}' -o '${runtimeConfigPath}'" ];
        ExecStart = [ "" "${pkgs.dyndnsd}/bin/dyndnsd --config '${runtimeConfigPath}'" ];
      };
    };
  };
}
