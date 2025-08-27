// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::{Config, Ipv6PrefixLen, Ipv6PrefixLenOrLan, UpdateProgram, User};
use crate::ipv6lanprefix::{Ipv6LanPrefix, Ipv6LanPrefixError};
use argon2::{password_hash::PasswordVerifier, Argon2};
use color_eyre::eyre::Result;
use log::{debug, error, info, trace, warn};
use serde_derive::Deserialize;
use std::io::Write;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::{Command, Stdio};
use warp::{http::StatusCode, Reply};

#[derive(Deserialize)]
pub struct QueryParameters {
	#[allow(dead_code)]
	domain: Option<String>, // Ignored, we use the username to determine the list of domains to be updated
	user: String,
	pass: String,
	ipv4: Option<Ipv4Addr>,
	ipv6: Option<Ipv6Addr>,
	#[allow(dead_code)]
	dualstack: Option<String>, // Unused for now. On a FRITZ!Box with IPv6 this was set to 1
	ipv6lanprefix: Option<Ipv6LanPrefix>,
}

#[derive(Deserialize)]
pub struct RawQueryParameters {
	domain: Option<String>, // Ignored, we use the username to determine the list of domains to be updated
	user: String,
	pass: String,
	ipv4: Option<Ipv4Addr>,
	ipv6: Option<Ipv6Addr>,
	dualstack: Option<String>, // Unused for now. On a FRITZ!Box with IPv6 this was set to 1
	ipv6lanprefix: Option<String>,
}

impl TryFrom<&RawQueryParameters> for QueryParameters {
	type Error = Ipv6LanPrefixError;

	fn try_from(raw_q: &RawQueryParameters) -> std::result::Result<Self, Self::Error> {
		let ipv6lanprefix = match raw_q.ipv6lanprefix.clone() {
			None => None,
			Some(s) => {
				let ipv6lanprefix: Ipv6LanPrefix = s.as_str().try_into()?;
				Some(ipv6lanprefix)
			}
		};
		let q = Self {
			domain: raw_q.domain.clone(),
			user: raw_q.user.clone(),
			pass: raw_q.pass.clone(),
			ipv4: raw_q.ipv4,
			ipv6: raw_q.ipv6,
			dualstack: raw_q.dualstack.clone(),
			ipv6lanprefix,
		};
		Ok(q)
	}
}

fn splice_ipv6_addrs(prefixlen: &Ipv6PrefixLen, prefix: Ipv6Addr, suffix: Ipv6Addr) -> Ipv6Addr {
	let prefix_bits = u128::from(prefix);
	let suffix_bits = u128::from(suffix);
	let hostlen = 128u8 - u8::from(prefixlen);
	let suffix_mask = 2u128.pow(u32::from(hostlen)) - 1;
	let masked_prefix = prefix_bits & !suffix_mask;
	let masked_suffix = suffix_bits & suffix_mask;
	Ipv6Addr::from(masked_prefix | masked_suffix)
}

fn build_domain_command_v4(
	mut command: String,
	update_program: &UpdateProgram,
	domain: &str,
	ttl: &str,
	ipv4: Ipv4Addr,
) -> String {
	let ipv4 = ipv4.to_string();
	command.push_str(
		#[allow(clippy::literal_string_with_formatting_args)]
		update_program
			.ipv4
			.stdin
			.replace("{domain}", domain)
			.replace("{ttl}", ttl)
			.replace("{ipv4}", &ipv4)
			.as_str(),
	);
	command
}

fn build_domain_command_v6(
	mut command: String,
	update_program: &UpdateProgram,
	domain: &str,
	ttl: &str,
	prefix_length: &Ipv6PrefixLen,
	prefix: Ipv6Addr,
	ipv6suffix: Ipv6Addr,
) -> String {
	let assembled_addr = splice_ipv6_addrs(prefix_length, prefix, ipv6suffix);
	let ipv6 = &assembled_addr.to_string();
	command.push_str(
		#[allow(clippy::literal_string_with_formatting_args)]
		update_program
			.ipv6
			.stdin
			.replace("{domain}", domain)
			.replace("{ttl}", ttl)
			.replace("{ipv6}", ipv6)
			.as_str(),
	);
	command
}

fn build_command_string(config: &Config, user: &User, q: &QueryParameters) -> String {
	// TODO: stream stdin to the process instead of building a string and then pushing it all at once
	let mut command = String::new();
	if let Some(initial_stdin) = &config.update_program.initial_stdin {
		command.push_str(initial_stdin);
	}
	let domains = &user.domains;
	for (domain, props) in domains {
		trace!("Domain: {domain:?} {props:?}");
		let ttl = &props.ttl.to_string();
		if let Some(ipv4) = q.ipv4 {
			command = build_domain_command_v4(command, &config.update_program, domain, ttl, ipv4);
		}
		match &props.ipv6prefixlen {
			Ipv6PrefixLenOrLan::Len(prefix_length) => {
				if u8::from(prefix_length) == 0 {
					warn!("IPv6 prefix length for domain {domain} is zero, ignoring update to IPv6 address");
				} else if let Some(prefix) = q.ipv6 {
					command = build_domain_command_v6(
						command,
						&config.update_program,
						domain,
						ttl,
						prefix_length,
						prefix,
						props.ipv6suffix,
					);
				}
			}
			Ipv6PrefixLenOrLan::Lan => {
				if let Some(ipv6lanprefix) = &q.ipv6lanprefix {
					command = build_domain_command_v6(
						command,
						&config.update_program,
						domain,
						ttl,
						&ipv6lanprefix.prefix_length,
						ipv6lanprefix.prefix,
						props.ipv6suffix,
					);
				}
			}
		}
		command.push_str(config.update_program.stdin_per_zone_update.as_str());
	}
	command.push_str(config.update_program.final_stdin.as_str());
	debug!("Commands for update program:\n{command}");
	command
}

pub fn update(config: &Config, raw_q: &RawQueryParameters) -> Result<impl Reply, impl Reply> {
	info!("Incoming request from user `{}`", &raw_q.user);
	debug!("domain: {:?}, user: {:?}, pass: <redacted>, ipv4: {:?}, ipv6: {:?}, dualstack: {:?}, ipv6lanprefix: {:?}", &raw_q.domain, &raw_q.user, &raw_q.ipv4, &raw_q.ipv6, &raw_q.dualstack, &raw_q.ipv6lanprefix);

	let q_result: std::result::Result<QueryParameters, Ipv6LanPrefixError> = raw_q.try_into();
	let q = match q_result {
		Ok(q) => q,
		Err(e) => {
			warn!("Error parsing QueryParameters: {e}");
			return Err(warp::reply::with_status(
				"Invalid query parameters".to_string(),
				StatusCode::BAD_REQUEST,
			));
		}
	};

	let Some(user) = config.users.get(&q.user) else {
		warn!("User {} does not exist.", q.user);
		return Err(warp::reply::with_status(
			"Not authorized".to_string(),
			StatusCode::FORBIDDEN,
		));
	};

	if let Err(e) = Argon2::default().verify_password(q.pass.as_bytes(), &user.hash) {
		warn!("Error verifying password: {e}");
		return Err(warp::reply::with_status(
			"Not authorized".to_string(),
			StatusCode::FORBIDDEN,
		));
	}

	info!("Authentication successful");

	let command = build_command_string(config, user, &q);

	let mut child = match Command::new(&config.update_program.bin)
		.args(&config.update_program.args)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
	{
		Ok(v) => v,
		Err(e) => {
			error!("Error spawning child process: {e}");
			return Err(warp::reply::with_status(
				e.to_string(),
				StatusCode::INTERNAL_SERVER_ERROR,
			));
		}
	};

	if let Some(mut stdin) = child.stdin.take() {
		if let Err(e) = stdin.write_all(command.as_bytes()) {
			error!("Error writing command to child process: {e}");
			return Err(warp::reply::with_status(
				e.to_string(),
				StatusCode::INTERNAL_SERVER_ERROR,
			));
		}
	}

	let output = match child.wait_with_output() {
		Ok(v) => v,
		Err(e) => {
			error!("Error waiting for the output of the child process: {e}");
			return Err(warp::reply::with_status(
				e.to_string(),
				StatusCode::INTERNAL_SERVER_ERROR,
			));
		}
	};

	let status = output.status;
	if !status.success() {
		error!("The update program failed with {status}");
		let stdout = String::from_utf8_lossy(&output.stdout);
		if !stdout.is_empty() {
			error!("and stdout: `{stdout}`");
		}
		let stderr = String::from_utf8_lossy(&output.stderr);
		if !stderr.is_empty() {
			error!("and stderr: `{stderr}`");
		}
		return Err(warp::reply::with_status(
			"ERROR".to_string(),
			StatusCode::INTERNAL_SERVER_ERROR,
		));
	}
	info!("Successfully processed update request");
	Ok(warp::reply::with_status("ok".to_string(), StatusCode::OK))
}
