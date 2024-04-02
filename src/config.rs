// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

mod errors {
	error_chain! {}
}
use errors::{Error, Result, ResultExt};

use argon2::password_hash::PasswordHash;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct RawConfig {
	listen: RawListen,
	update_program: UpdateProgram,
	users: HashMap<String, RawUser>,
}

#[derive(Debug, Deserialize)]
struct RawListen {
	ip: IpAddr,
	port: u16,
}

#[derive(Debug, Deserialize)]
struct RawUser {
	hash: String,
	domains: HashMap<String, Domain>,
}

#[derive(Clone, Debug)]
pub struct Config<'a> {
	pub listen: SocketAddr,
	pub update_program: UpdateProgram,
	pub users: HashMap<String, User<'a>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateProgram {
	pub bin: String,
	pub args: Vec<String>,
	pub initial_stdin: String,
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

#[derive(Clone, Debug)]
pub struct User<'a> {
	pub hash: PasswordHash<'a>,
	pub domains: HashMap<String, Domain>,
}

pub fn read(filename: &PathBuf) -> Result<Config<'static>> {
	let contents = fs::read_to_string(filename)
		.chain_err(|| format!("Cannot read config file `{}`", filename.display()))?;
	let config_parse_err_msg = || format!("Cannot parse config file `{}`", filename.display());
	let raw_config: RawConfig = toml::from_str(&contents).chain_err(config_parse_err_msg)?;
	let mut users = HashMap::new();

	for (username, raw_user) in raw_config.users {
		let domains = &raw_user.domains;
		for (domain, props) in domains {
			let ipv6prefixlen_parse_err_msg =
				|| format!("Cannot parse ipv6prefixlen for user {username} and domain {domain}");
			if props.ipv6prefixlen > 128 {
				let prefixlen = props.ipv6prefixlen;
				return Err(
					Error::from(format!("Prefix is longer than 128 bits: {prefixlen}"))
						.chain_err(ipv6prefixlen_parse_err_msg)
						.chain_err(config_parse_err_msg),
				);
			};
		}
		// TODO: figure out how to do this without leaking memory. I wish PasswordHash::new() took a String instead of &str
		let raw_hash = Box::leak(Box::new(raw_user.hash));
		let user = User {
			// TODO: get rid of this piece of the code by somehow implementing deserialization for PasswordHash
			hash: PasswordHash::new(raw_hash)
				.chain_err(|| format!("Cannot parse password hash of user {username}"))
				.chain_err(config_parse_err_msg)?,
			domains: raw_user.domains,
		};
		users.insert(username.to_string(), user);
	}
	let config = Config {
		listen: SocketAddr::from((raw_config.listen.ip, raw_config.listen.port)),
		update_program: raw_config.update_program,
		users,
	};

	Ok(config)
}
