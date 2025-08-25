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

#[derive(Debug, Deserialize)]
struct RawUser {
	hash: String,
	domains: HashMap<String, Domain>,
}

#[derive(Clone, Debug)]
pub struct User<'a> {
	pub hash: PasswordHash<'a>,
	pub domains: HashMap<String, Domain>,
}

#[derive(thiserror::Error, Debug)]
pub enum UserConvertError {
	#[error("Cannot parse ipv6prefixlen for domain {domain_name} because the prefix is longer than 128 bits: {prefixlen}")]
	InvalidIPv6PrefixLen { domain_name: String, prefixlen: u8 },

	#[error("Cannot parse password hash {hash}")]
	InvalidPasswordHash {
		hash: String,
		source: argon2::password_hash::Error,
	},
}

impl TryFrom<RawUser> for User<'_> {
	type Error = UserConvertError;

	fn try_from(raw_user: RawUser) -> std::result::Result<Self, Self::Error> {
		let domains = &raw_user.domains;
		for (domain, props) in domains {
			if props.ipv6prefixlen > 128 {
				let prefixlen = props.ipv6prefixlen;
				return Err(UserConvertError::InvalidIPv6PrefixLen {
					domain_name: domain.to_string(),
					prefixlen,
				});
			}
		}
		// TODO: figure out how to do this without leaking memory. I wish PasswordHash::new() took a String instead of &str
		let raw_hash = Box::leak(Box::new(raw_user.hash));
		let user = User {
			// TODO: get rid of this piece of the code by somehow implementing deserialization for PasswordHash
			hash: PasswordHash::new(raw_hash).map_err(|source| {
				UserConvertError::InvalidPasswordHash {
					hash: (*raw_hash).to_string(),
					source,
				}
			})?,
			domains: raw_user.domains,
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

#[derive(Clone, Debug, Deserialize)]
pub struct Domain {
	pub ttl: u32,
	pub ipv6prefixlen: u8,
	pub ipv6suffix: Ipv6Addr,
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
	#[error("Cannot validate user `{username}`")]
	UserConvert {
		username: String,
		source: UserConvertError,
	},
}

impl TryFrom<RawConfig> for Config<'_> {
	type Error = ConfigConvertError;

	fn try_from(raw_config: RawConfig) -> std::result::Result<Self, Self::Error> {
		let listen = raw_config.listen.map(std::convert::Into::into);
		let users: std::result::Result<HashMap<_, _>, ConfigConvertError> = raw_config
			.users
			.into_iter()
			.map(|(username, raw_user)| {
				let user: User =
					raw_user
						.try_into()
						.map_err(|source| ConfigConvertError::UserConvert {
							username: username.clone(),
							source,
						})?;
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
