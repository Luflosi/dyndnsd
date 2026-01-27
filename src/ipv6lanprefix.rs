// SPDX-FileCopyrightText: 2025 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::{Ipv6PrefixLen, Ipv6PrefixLenError};
use color_eyre::eyre::Result;
use serde_derive::Deserialize;
use std::net::{AddrParseError, Ipv6Addr};
use std::num::ParseIntError;

// Parsing of Ipv6LanPrefix inspired by https://stackoverflow.com/questions/78180964/deserialize-a-field-into-2-fields

#[derive(Debug, Deserialize)]
#[serde(try_from = "&str")]
pub struct Ipv6LanPrefix {
	pub prefix: Ipv6Addr,
	pub prefix_length: Ipv6PrefixLen,
}

#[derive(thiserror::Error, Debug)]
pub enum Ipv6LanPrefixError {
	#[error(
		"Could not parse ipv6lanprefix because it does not contain a / to separate the address from the prefix: {string}"
	)]
	NoSlash { string: String },

	#[error(
		"Could not parse ipv6lanprefix because the prefix is not a valid IPv6 address: {prefix}"
	)]
	InvalidAddress {
		prefix: String,
		source: AddrParseError,
	},

	#[error(
		"Could not parse ipv6lanprefix because the prefix length is not a valid number (u8): {prefix_length}"
	)]
	PrefixLengthNotANumber {
		prefix_length: String,
		source: ParseIntError,
	},

	#[error(
		"Could not parse ipv6lanprefix because the prefix length is not valid: {prefix_length}"
	)]
	InvalidPrefixLength {
		prefix_length: u8,
		source: Ipv6PrefixLenError,
	},
}

impl<'a> TryFrom<&'a str> for Ipv6LanPrefix {
	type Error = Ipv6LanPrefixError;

	fn try_from(value: &'a str) -> Result<Self, Self::Error> {
		let Some((prefix_str, prefix_length_str)) = value.split_once('/') else {
			return Err(Ipv6LanPrefixError::NoSlash {
				string: value.to_string(),
			});
		};
		let prefix = prefix_str.parse::<Ipv6Addr>().map_err(|source| {
			Ipv6LanPrefixError::InvalidAddress {
				prefix: prefix_str.to_string(),
				source,
			}
		})?;
		let prefix_length_u8 = prefix_length_str.parse::<u8>().map_err(|source| {
			Ipv6LanPrefixError::PrefixLengthNotANumber {
				prefix_length: prefix_str.to_string(),
				source,
			}
		})?;
		let prefix_length = prefix_length_u8.try_into().map_err(|source| {
			Ipv6LanPrefixError::InvalidPrefixLength {
				prefix_length: prefix_length_u8,
				source,
			}
		})?;
		Ok(Self {
			prefix,
			prefix_length,
		})
	}
}
