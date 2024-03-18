[SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>]::
[SPDX-License-Identifier: AGPL-3.0-only]::

# dyndnsd
## A simple but configurable web server for dynamic DNS updates.
It's a small webserver that receives DNS update requests and uses an external tool like `nsupdate` to talk to the authoritative DNS server.

The dyndnsd program is written in such a way that it can interface with the `nsupdate` program but I'll be using a different program (not yet written) that will allow me to not hand complete control of the Zone file over to the DNS server so that I can still manually edit the non-dynamic parts of the Zone file.

## Setup on NixOS
Take a look at `nix/e2e-test.nix` for an example. You need to import the module and overlay provided by this flake.

I use this service only on NixOS but it should just work on other Linux distributions as well.

## Manual installation and usage instructions (Non-NixOS) (probably incomplete)
- Install a rust compiler
- Compile the program from source with `cargo build`
- Copy the binary into a sensible location like `/usr/local/bin`
- Copy the systemd unit from `systemd/dyndnsd.service` to `/etc/systemd/system/dyndnsd.service` and adapt it to your needs
- Copy the example configuration file to `/etc/dyndnsd/config.toml`
- Modify or add users and domains in the configuration file
- You need to generate a new password hasn for each user
- Choose a strong password!
- Do not reuse the provided insecure password hashes!
- Enable and start the systemd unit
- Set up a reverse proxy with `nginx` to add TLS
- Set up an update client to use the correct URL

If you want to update an IP address in your local network at home, I recommend using your router for this if possible.
Your router knows when its IP address changes and can send an update immediately.
This removes the need for prequent periodic updates.

## List of tested routers (please add to this list)
- FRITZ!Box routers
- OpenWrt

## IPv6 addresses
This daemon provides the most flexible IPv6 support of any DNS update client I know of.

This is required if you run the update client on your router but want to update the DNS record of a device behind the router.

In this case your internet provider probably gives you a dynamic IPv6 prefix (often 56 bit).
Your router then picks some address range such that each host in your local network can pick 64 bit of the address.
With the above example of 56 bit your router would pick the next 8 bit and each device would choose the remaining 64 bit.
To make this work with dynamic DNS updates, edit the configuratin and change `ipv6prefixlen` to the prefix length your ISP gives you (e.g. 56).
Then make sure that the host for which you want to update the DNS record has a predictable IPv6 suffix.
For example if your IP address happens to be 2001:db8:0123:4567:8901:2345:6789:0123 right now, the last 128-56=72 bits should never change (67:8901:2345:6789:0123))
Then add zeroes to the front of the suffix to make it a valid IPv6 address (0::67:8901:2345:6789:0123 in this example) and set it as the `ipv6suffix`.

If your router sends an IPv6 address in the URL but you do not want to update the corresponding AAAA DNS record, set `ipv6prefixlen`.
This will cause `dyndnsd` to ignore the update for IPv6.


## Notes
`curl` command illustrating the URL syntax:
```sh
curl -v 'https://[::1]:9841/update?user=bob&pass=123456&ipv4=1.2.3.4&ipv6=1::2'
```

You should use a reverse proxy server like Nginx for TLS so that passwords are encrypted while they are transmitted over the internet.


This is one of my first Rust projects so the code will not look very idiomatic. If you have any suggestions for improvements, please do not hesitate to create an issue or even a PR!


If you would like to see any of the following TODO items implemented, please file an issue so I know that it is important to someone.

## TODO:
- Listen on more than one IP address
- Package in Nixpkgs
- Test on platforms other than `x86_64-linux`
- Use systemd socket activation for lower resource usage
- Support HTTP basic auth
- Implement proper logging with different log levels to reduce log spam
- Improve documentation
- Add some simpler tests in Rust in addition to the NixOS test
- Make use of the `ipv6lanprefix` sent by FRITZ!Boxes


## License
The license is the GNU AGPLv3 (AGPL-3.0-only).
