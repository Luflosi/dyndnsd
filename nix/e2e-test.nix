# SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
# SPDX-License-Identifier: AGPL-3.0-only

self:
{ lib, pkgs, ... }: {
  name = "dyndns";
  # TODO: test both with and without `environmentFiles`
  nodes.machine = { config, pkgs, ... }: {
    imports = [
      self.outputs.nixosModules.dyndnsd
    ];

    systemd.tmpfiles.settings."bind"."/var/lib/bind/zones/example.org/".d = {
      user = "named";
      group = "named";
    };

    systemd.services.bind.preStart = let
      zoneFile = pkgs.writeText "root.zone" ''
        $ORIGIN example.org.
        $TTL 3600
        @ IN SOA ns.example.org. admin.example.org. ( 1 3h 1h 1w 1d )
        @ IN NS ns.example.org.

        ns IN A    127.0.0.1
        ns IN AAAA ::1

        1.0.0.127.in-addr.arpa IN PTR ns.example.org.
        1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.ip6.arpa IN PTR ns.example.org.

        @ IN A    1.2.3.4
        @ IN AAAA 1:2:3:4:5:6:7:8

        test IN A    4.3.2.1
        test IN AAAA 8:7:6:5:4:3:2:1
      '';
    in ''
      cp '${zoneFile}' '/var/lib/bind/zones/example.org/example.org.zone'
    '';

    services.bind = {
      enable = true;
      forward = "only";
      forwarders = [];
      extraOptions = ''
        empty-zones-enable no;
      '';
      zones = {
        "example.org" = {
          file = "/var/lib/bind/zones/example.org/example.org.zone";
          master = true;
          extraConfig = ''
            update-policy {
              grant ddns name example.org A AAAA;
              grant ddns name test.example.org A AAAA;
            };
          '';
        };
      };
    };

    environment.etc = {
      # Declaring this password hash here defeats the purpose of keeping the password hash outside of the Nix store
      # In a real deployment either manage it outside of Nix or just put the password hash into the `services.dyndnsd.settings.users.<user>.hash` option.
      "dyndnsd/vars.env".text = ''
        HASH="$argon2id$v=19$m=65536,t=3,p=1$ZFRHDlJOQ3UNQRN7em14R08FIRE$0SqSQRj45ZBz1MfCPq9DVMWt7VSl96m7XtW6maIcUB0"
      '';
    };
    services.dyndnsd = {
      enable = true;
      useNsupdateProgram = true;
      settings = {
        users = {
          alice = {
            hash = "$HASH";
            domains = {
              "example.org" = {
                ttl = 60;
                ipv6prefixlen = 48;
                ipv6suffix = "0:0:0:1::5";
              };
              "test.example.org" = {
                ttl = 300;
                ipv6prefixlen = 48;
                ipv6suffix = "0:0:0:1::6";
              };
            };
          };
        };
      };
      environmentFiles = [ "/etc/dyndnsd/vars.env" ];
    };

    environment.systemPackages = [ pkgs.dig.dnsutils ]; # Provide the `dig` command in the test script
  };
  testScript = ''
    def query(
        query: str,
        query_type: str,
        expected: str,
    ):
        """
        Execute a single query and compare the result with expectation
        """
        out = machine.succeed(
            f"dig {query} {query_type} +short"
        ).strip()
        machine.log(f"DNS server replied with {out}")
        assert expected == out, f"Expected `{expected}` but got `{out}`"

    start_all()
    machine.wait_for_unit("dyndnsd.service")
    machine.wait_for_unit("bind.service")
    query("example.org", "A", "1.2.3.4")
    query("example.org", "AAAA", "1:2:3:4:5:6:7:8")
    query("test.example.org", "A", "4.3.2.1")
    query("test.example.org", "AAAA", "8:7:6:5:4:3:2:1")
    machine.succeed("curl --fail-with-body -v 'http://[::1]:9841/update?user=alice&pass=123456&ipv4=2.3.4.5&ipv6=2:3:4:5:6:7:8:9'")
    query("example.org", "A", "2.3.4.5")
    query("example.org", "AAAA", "2:3:4:1::5")
    query("test.example.org", "A", "2.3.4.5")
    query("test.example.org", "AAAA", "2:3:4:1::6")
  '';
}
