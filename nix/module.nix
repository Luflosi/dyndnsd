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
        description = ''
          The encoded Argon2 password hash for the user.
          To generate the password hash with a strong salt and without leaving the password in the shell history, execute
          `nix run nixpkgs#libargon2 -- "$(LC_ALL=C tr -dc '[:print:][:cntrl:]' </dev/urandom | head -c 20)" -id -m 16`
          then type the password, then press Ctrl-d twice.
        '';
      };
      domains = lib.mkOption {
        type = lib.types.attrsOf (lib.types.submodule domainOpts);
        default = {};
        description = "Attribute set of domains this user updates after authenticating.";
        example = {
          "example.org" = {
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
        type = lib.types.ints.u32;
        default = 60;
        description = ''
          The TTL of the DNS record.
          See `update_program.ipv4.stdin` and `update_program.ipv6.stdin`.
        '';
      };
      ipv6prefixlen = lib.mkOption {
        type = lib.types.ints.between 0 128;
        default = 128;
        example = 56;
        description = ''
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
        description = ''
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
      enable = lib.mkEnableOption "the DynDNS server";

      useNsupdateProgram = lib.mkEnableOption ''
        the recommended default configuration for using nsupdate with BIND.
        This sets all of the `services.dyndnsd.settings.update_program` options.
        It also creates a key when the system starts and tells BIND where to find it.
        The key is readable by the `ddns` group, which is also created. `dyndnsd` is allowed access to the key.
        You still need to manually set an `update-policy` with which BIND allows updates to specific domain names.
        Look at `nix/e2e-test.nix` for an example of how to do that.
      '';

      useZonegen = lib.mkEnableOption ''
        the recommended default configuration for using zonegen.
        This sets all of the `services.dyndnsd.settings.update_program` options.
        It also creates the `zonegen` group and allows zonegen to access `/var/lib/bind/zones/dyn/`.
      '';

      localhost = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Assume that dyndnsd and the DNS server run on the same system.
          In this case, `nsupdate` can contact the server via one of the localhost addresses.
          This improves security slightly by hardening the dyndnsd systemd service to only allow contacting localhost and deny all other addresses.
        '';
      };

      settings = {
        listen = {
          ip = lib.mkOption {
            type = lib.types.str;
            default = "::1";
            example = "::";
            description = ''
              Only listen to incoming requests on a specific IP address.
              The default is to listen on IPv6 localhost.
              The special address :: will listen on all IPv4 and IPv6 addresses.
            '';
          };
          port = lib.mkOption {
            type = lib.types.port;
            default = 9841;
            description = ''
              The port on which to listen.
            '';
          };
        };

        update_program = {
          bin = lib.mkOption {
            type = lib.types.path;
            default = lib.getExe' pkgs.coreutils "false";
            example = lib.literalExpression ''lib.getExe' pkgs.coreutils "false"'';
            description = ''
              Path to a program which will be used to forward the updated (dynamic) IP addresses to the actual DNS server.
            '';
          };
          args = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [];
            example = [ "-k" "/etc/bind/ddns.key" ];
            description = ''
              Command line arguments the update program will be called with.
            '';
          };
          initial_stdin = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = lib.literalExpression ''
              if cfg.localhost then "server ::1\n" else null;
            '';
            description = ''
              String to send to the stdin of the update program before sending anything else.
            '';
          };
          stdin_per_zone_update = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "send\n";
            description = ''
              String to send to the stdin of the update program after each zone (domain) was updated.
            '';
          };
          final_stdin = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "quit\n";
            description = ''
              String to send to the stdin of the update program when we're done.
            '';
          };
          ipv4 = {
            stdin = lib.mkOption {
              type = lib.types.str;
              default = "";
              example = "update delete {domain}. IN A\nupdate add {domain}. {ttl} IN A {ipv4}\n";
              description = ''
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
              description = ''
                String template to send to the stdin of the update program for updating the IPv6 DNS record.
                The three different variables are replaced with the appropriate values before the string is sent to the update program.
              '';
            };
          };
        };

        users = lib.mkOption {
          type = lib.types.attrsOf (lib.types.submodule userOpts);
          default = {};
          description = "Attribute set of users.";
          example = {
            alice = {
              hash = "$argon2id$v=19$m=65536,t=3,p=1$ZFRHDlJOQ3UNQRN7em14R08FIRE$0SqSQRj45ZBz1MfCPq9DVMWt7VSl96m7XtW6maIcUB0";
              domains = {
                "example.org" = {
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
        description = ''
          Files to load as environment file. Environment variables from this file
          will be substituted into the static configuration file using [envsubst](https://github.com/a8m/envsubst).
          If you change this option, make sure to either not have any password hashes in the configuration
          or replace all the `$` with `$$` since `envsubst` would otherwise delete parts of the password hash.
        '';
      };
    };
  };


  config = lib.mkMerge [
    (lib.mkIf cfg.enable {
      systemd.packages = [ pkgs.dyndnsd ];

      systemd.services.dyndnsd = {
        description = "Service that updates a dynamic DNS record";
        after = [ "network.target" ];
        wantedBy = [ "multi-user.target" ];
        startLimitBurst = 1;

        serviceConfig = let
          settingsNoNulls = lib.filterAttrsRecursive (_: v: v != null) cfg.settings;
          settingsFile = settingsFormat.generate "dyndnsd.toml" settingsNoNulls;
          runtimeConfigPath = if cfg.environmentFiles != []
            then "/run/${RuntimeDirectory}/dyndnsd.toml"
            else settingsFile;
        in {
          inherit RuntimeDirectory;
          EnvironmentFile = cfg.environmentFiles;
          ExecStartPre = lib.mkIf (cfg.environmentFiles != []) [ "'${lib.getExe pkgs.envsubst}' -no-unset -i '${settingsFile}' -o '${runtimeConfigPath}'" ];
          ExecStart = [ "" "${lib.getExe pkgs.dyndnsd} --config '${runtimeConfigPath}'" ];
        } // lib.optionalAttrs cfg.localhost {
          IPAddressAllow = [ "localhost" ];
          IPAddressDeny = "any";
        };
      };

      assertions = lib.singleton {
        assertion = !(cfg.useNsupdateProgram && cfg.useZonegen);
        message = ''
          Only one of services.dyndnsd.useNsupdateProgram and services.dyndnsd.useZonegen can be set at once.
        '';
      };
    })

    (lib.mkIf (cfg.enable && cfg.useNsupdateProgram) {
      users.groups.ddns = {};
      systemd.services.dyndnsd.serviceConfig.SupplementaryGroups = [ "ddns" ];

      systemd.tmpfiles.settings."create-ddns-key"."/run/named".d = {
        user = "named";
        group = "named";
      };

      systemd.services.create-ddns-key = {
        description = "Service to create a key for the ddns group to authenticate to BIND";
        before = [ "bind.service" "dyndnsd.service" ];
        requiredBy = [ "bind.service" "dyndnsd.service" ];
        script = ''
          if ! [ -f "/run/named/ddns.key" ]; then
            ${lib.getExe' config.services.bind.package "rndc-confgen"} -c /run/named/ddns.key -u named -a -k ddns 2>/dev/null
            chmod 440 /run/named/ddns.key
          fi
        '';
        serviceConfig = {
          Type = "oneshot";
          StartLimitBurst = 1;
          User = "named";
          Group = "ddns";
          UMask = "0227";
        };
      };

      services.bind.extraConfig = ''
        include "/run/named/ddns.key";
      '';

      services.dyndnsd.settings.update_program = {
        bin = lib.mkDefault (lib.getExe' pkgs.dig.dnsutils "nsupdate");
        args = lib.mkDefault [ "-k" "/run/named/ddns.key" ];
        initial_stdin = lib.mkDefault (if cfg.localhost then "server ::1\n" else null);
        stdin_per_zone_update = lib.mkDefault "send\n";
        final_stdin = lib.mkDefault "quit\n";
        ipv4.stdin = lib.mkDefault "update delete {domain}. IN A\nupdate add {domain}. {ttl} IN A {ipv4}\n";
        ipv6.stdin = lib.mkDefault "update delete {domain}. IN AAAA\nupdate add {domain}. {ttl} IN AAAA {ipv6}\n";
      };
    })

    (lib.mkIf (cfg.enable && cfg.useZonegen) {
      users.groups.zonegen = {};

      systemd.services.dyndnsd.serviceConfig = {
        SupplementaryGroups = [ "zonegen" ];
        ReadWritePaths = [ "/var/lib/bind/zones/dyn/" ];

        # The tempfile-fast rust crate tries to keep the old permissions, so we need to allow this class of system calls
        SystemCallFilter = [ "@chown" ];
        UMask = "0022"; # Allow all processes (including BIND) to read the zone files (and database)
      };

      services.dyndnsd.settings.update_program = {
        bin = lib.getExe pkgs.zonegen;
        args = [ "--dir" "/var/lib/bind/zones/dyn/" ];
        stdin_per_zone_update = "send\n";
        final_stdin = "quit\n";
        ipv4.stdin = "update add {domain}. {ttl} IN A {ipv4}\n";
        ipv6.stdin = "update add {domain}. {ttl} IN AAAA {ipv6}\n";
      };
    })
  ];
}
