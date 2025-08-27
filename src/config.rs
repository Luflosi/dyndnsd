// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

use argon2::password_hash::PasswordHash;
use color_eyre::eyre::{Result, WrapErr};
use log::info;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RawIpv6PrefixLenOrLan {
	Len(u8),
	Lan(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ipv6PrefixLen(u8);

#[derive(Clone, Debug, Deserialize)]
pub enum Ipv6PrefixLenOrLan {
	Lan,
	Len(Ipv6PrefixLen),
}

#[derive(thiserror::Error, Debug)]
pub enum Ipv6PrefixLenError {
	#[error("The prefix is longer than 128 bits: {prefixlen}")]
	TooLong { prefixlen: u8 },
}

#[derive(thiserror::Error, Debug)]
pub enum Ipv6PrefixLenOrLanError {
	#[error("The prefix is too long")]
	PrefixTooLong { source: Ipv6PrefixLenError },

	#[error("Unexpected String {string}")]
	UnexpectedString { string: String },

	#[error("The prefix is lan")]
	IsLan {},
}

impl TryFrom<u8> for Ipv6PrefixLen {
	type Error = Ipv6PrefixLenError;

	fn try_from(prefixlen: u8) -> std::result::Result<Self, Self::Error> {
		if prefixlen <= 128 {
			Ok(Self(prefixlen))
		} else {
			Err(Ipv6PrefixLenError::TooLong { prefixlen })
		}
	}
}

impl From<Ipv6PrefixLen> for u8 {
	fn from(prefixlen: Ipv6PrefixLen) -> Self {
		prefixlen.0
	}
}

impl From<&Ipv6PrefixLen> for u8 {
	fn from(prefixlen: &Ipv6PrefixLen) -> Self {
		prefixlen.0
	}
}

impl From<Ipv6PrefixLen> for Ipv6PrefixLenOrLan {
	fn from(prefixlen: Ipv6PrefixLen) -> Self {
		Self::Len(prefixlen)
	}
}

impl TryFrom<Ipv6PrefixLenOrLan> for Ipv6PrefixLen {
	type Error = Ipv6PrefixLenOrLanError;

	fn try_from(prefixlen: Ipv6PrefixLenOrLan) -> std::result::Result<Self, Self::Error> {
		if let Ipv6PrefixLenOrLan::Len(len) = prefixlen {
			Ok(len)
		} else {
			Err(Ipv6PrefixLenOrLanError::IsLan {})
		}
	}
}

impl TryFrom<RawIpv6PrefixLenOrLan> for Ipv6PrefixLenOrLan {
	type Error = Ipv6PrefixLenOrLanError;

	fn try_from(prefixlen: RawIpv6PrefixLenOrLan) -> std::result::Result<Self, Self::Error> {
		match prefixlen {
			RawIpv6PrefixLenOrLan::Len(l) => match Ipv6PrefixLen::try_from(l) {
				Ok(len) => Ok(Self::from(len)),
				Err(source) => Err(Ipv6PrefixLenOrLanError::PrefixTooLong { source }),
			},
			RawIpv6PrefixLenOrLan::Lan(s) => match s.as_str() {
				"lan" => Ok(Self::Lan),
				s => Err(Ipv6PrefixLenOrLanError::UnexpectedString {
					string: s.to_string(),
				}),
			},
		}
	}
}

#[derive(Debug, Deserialize)]
struct RawListen {
	ip: IpAddr,
	port: u16,
}

impl From<RawListen> for SocketAddr {
	fn from(raw_listen: RawListen) -> Self {
		Self::from((raw_listen.ip, raw_listen.port))
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct Domain {
	pub ttl: u32,
	pub ipv6prefixlen: Ipv6PrefixLenOrLan,
	pub ipv6suffix: Ipv6Addr,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawDomain {
	pub ttl: u32,
	pub ipv6prefixlen: RawIpv6PrefixLenOrLan,
	pub ipv6suffix: Ipv6Addr,
}

#[derive(thiserror::Error, Debug)]
pub enum DomainConvertError {
	#[error("Cannot parse ipv6prefixlen for domain {domain_name}")]
	InvalidIpv6PrefixLen {
		domain_name: String,
		source: Ipv6PrefixLenOrLanError,
	},
}

impl RawDomain {
	fn try_into(self, domain_name: &String) -> std::result::Result<Domain, DomainConvertError> {
		let ipv6prefixlen = self.ipv6prefixlen.try_into().map_err(|source| {
			DomainConvertError::InvalidIpv6PrefixLen {
				domain_name: domain_name.to_string(),
				source,
			}
		})?;
		let domain = Domain {
			ttl: self.ttl,
			ipv6prefixlen,
			ipv6suffix: self.ipv6suffix,
		};
		Ok(domain)
	}
}

#[derive(Debug, Deserialize)]
struct RawUser {
	hash: String,
	domains: HashMap<String, RawDomain>,
}

#[derive(Clone, Debug)]
pub struct User<'a> {
	pub hash: PasswordHash<'a>,
	pub domains: HashMap<String, Domain>,
}

#[derive(thiserror::Error, Debug)]
pub enum UserConvertError {
	#[error("Cannot parse domain config for username {username}")]
	DomainConvert {
		username: String,
		source: DomainConvertError,
	},

	#[error("Cannot parse password hash {hash} for username {username}")]
	InvalidPasswordHash {
		username: String,
		hash: String,
		source: argon2::password_hash::Error,
	},
}

impl RawUser {
	fn try_into(self, username: &String) -> std::result::Result<User<'static>, UserConvertError> {
		let raw_domains = &self.domains;
		let domains: std::result::Result<HashMap<_, _>, UserConvertError> = raw_domains
			.iter()
			.map(|(domain_name, raw_domain)| {
				let domain: Domain =
					raw_domain.clone().try_into(domain_name).map_err(|source| {
						UserConvertError::DomainConvert {
							username: username.clone(),
							source,
						}
					})?;
				Ok((domain_name.to_string(), domain))
			})
			.collect();
		// TODO: figure out how to do this without leaking memory. I wish PasswordHash::new() took a String instead of &str
		let raw_hash = Box::leak(Box::new(self.hash));
		let user = User {
			// TODO: get rid of this piece of the code by somehow implementing deserialization for PasswordHash
			hash: PasswordHash::new(raw_hash).map_err(|source| {
				UserConvertError::InvalidPasswordHash {
					username: username.to_string(),
					hash: (*raw_hash).to_string(),
					source,
				}
			})?,
			domains: domains?,
		};
		Ok(user)
	}
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateProgram {
	pub bin: String,
	pub args: Vec<String>,
	pub initial_stdin: Option<String>,
	pub stdin_per_zone_update: String,
	pub final_stdin: String,
	pub ipv4: SpecialUpdateProgram,
	pub ipv6: SpecialUpdateProgram,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SpecialUpdateProgram {
	pub stdin: String,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
	listen: Option<RawListen>,
	update_program: UpdateProgram,
	users: HashMap<String, RawUser>,
}

#[derive(Clone, Debug)]
pub struct Config<'a> {
	pub listen: Option<SocketAddr>,
	pub update_program: UpdateProgram,
	pub users: HashMap<String, User<'a>>,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigConvertError {
	#[error("The config is invalid")]
	UserConvert { source: UserConvertError },
}

impl TryFrom<RawConfig> for Config<'_> {
	type Error = ConfigConvertError;

	fn try_from(raw_config: RawConfig) -> std::result::Result<Self, Self::Error> {
		let listen = raw_config.listen.map(std::convert::Into::into);
		let users: std::result::Result<HashMap<_, _>, ConfigConvertError> = raw_config
			.users
			.into_iter()
			.map(|(username, raw_user)| {
				let user: User = raw_user
					.try_into(&username)
					.map_err(|source| ConfigConvertError::UserConvert { source })?;
				Ok((username, user))
			})
			.collect();
		let config = Config {
			listen,
			update_program: raw_config.update_program,
			users: users?,
		};

		Ok(config)
	}
}

impl Config<'_> {
	pub fn read(filename: &Path) -> Result<Config<'static>> {
		info!("Reading config file {}", filename.display());
		let contents = fs::read_to_string(filename)
			.wrap_err_with(|| format!("Cannot read config file `{}`", filename.display()))?;
		let config: Config<'_> = Config::parse(&contents)
			.wrap_err_with(|| format!("Cannot parse config file `{}`", filename.display()))?;
		Ok(config)
	}

	fn parse(contents: &str) -> Result<Self> {
		let raw_config: RawConfig = toml::from_str(contents)?;
		Ok(raw_config.try_into()?)
	}
}
