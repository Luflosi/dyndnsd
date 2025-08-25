[SPDX-FileCopyrightText: 2025 Luflosi <dyndnsd@luflosi.de>]::
[SPDX-License-Identifier: AGPL-3.0-only]::

# Setting up DynDNS with FRITZ!Box routers

- Log in to the web interface of your FRITZ!Box router
- Click on 'Internet'
- Click on 'Permit Access' in the 'Internet' menu
- Click on the 'DynDNS' tab
- Enable the option 'Use DynDNS'
- Enter fhe following URL in the 'Update URL' field (but replace example.org with your own domain name):
  ```
  https://example.org/update?user=<username>&pass=<pass>&domain=<domain>&ipv4=<ipaddr>&ipv6=<ip6addr>&ipv6lanprefix=<ip6lanprefix>&dualstack=<dualstack>
  ```
- Enter the domain you want to update in the 'Domain name' field
- Enter your dyndnsd username in the 'Username' field
- Enter your dyndnsd password in the 'Password' field
- Click on 'Apply' to save the settings

## Notes

The domain name is not used directly.
Instead, we use the username to determine the list of domains to be updated.
The FRITZ!Box only queries the IP addresses of the domain name in the 'Domain name' field after updating it to verify that the update was successful.
For this reason, the `ipv6prefixlen` setting for this domain name should be left at the default of `128`.
Otherwise, the FRITZ!Box might get confused, since it will see an IPv6 address it did not expect.
I usually use a subdomain like `dyn.example.org` for this purpose.
You can then add `example.org` to the same account (username) in dyndnsd and set the prefix length however you want for this domain.

## Links (English)
[Setting up dynamic DNS in the FRITZ!Box](https://en.fritz.com/service/knowledge-base/dok/FRITZ-Box-7590/30_Setting-up-dynamic-DNS-in-the-FRITZ-Box/)

## Links (German)
### Wissensdatenbank
[Dynamic DNS in FRITZ!Box einrichten](https://fritz.com/service/wissensdatenbank/dok/FRITZ-Box-7590/30_Dynamic-DNS-in-FRITZ-Box-einrichten/)

### Hilfe
[Dynamic DNS in der FRITZ!Box einrichten](https://fritzhelp.avm.de/help/de/FRITZ-Box-7530/avm/024p1/hilfe_dyndns)
[Update-URL selbst zusammensetzen](https://fritzhelp.avm.de/help/de/FRITZ-Box-7530/avm/024p1/hilfe_dyndns_update_url_bauen)
