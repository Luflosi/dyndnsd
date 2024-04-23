// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::{Config, User};
use argon2::{password_hash::PasswordVerifier, Argon2};
use log::{debug, error, info, trace, warn};
use serde_derive::Deserialize;
use std::io::Write;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::{Command, Stdio};
use warp::{http::StatusCode, Reply};

#[derive(Deserialize)]
pub struct QueryParameters {
	domain: Option<String>, // Ignored, we use the username to determine the list of domains to be updated
	user: String,
	pass: String,
	ipv4: Option<Ipv4Addr>,
	ipv6: Option<Ipv6Addr>,
	dualstack: Option<String>,
	ipv6lanprefix: Option<String>,
}

fn splice_ipv6_addrs(prefixlen: u8, prefix: Ipv6Addr, suffix: Ipv6Addr) -> Ipv6Addr {
	let prefix_bits = u128::from(prefix);
	let suffix_bits = u128::from(suffix);
	let hostlen = 128 - prefixlen;
	let suffix_mask = 2u128.pow(u32::from(hostlen)) - 1;
	let masked_prefix = prefix_bits & !suffix_mask;
	let masked_suffix = suffix_bits & suffix_mask;
	Ipv6Addr::from(masked_prefix | masked_suffix)
}

fn build_command_string(config: &Config, user: &User, q: &QueryParameters) -> String {
	// TODO: stream stdin to the process instead of building a string and then pushing it all at once
	let mut command = String::new();
	command.push_str(config.update_program.initial_stdin.as_str());
	let domains = &user.domains;
	for (domain, props) in domains {
		trace!("Domain: {domain:?} {props:?}");
		let ttl = &props.ttl.to_string();
		if let Some(ipv4) = q.ipv4 {
			let ipv4 = &ipv4.to_string();
			command.push_str(
				config
					.update_program
					.ipv4
					.stdin
					.replace("{domain}", domain)
					.replace("{ttl}", ttl)
					.replace("{ipv4}", ipv4)
					.as_str(),
			);
		}
		if let Some(prefix) = q.ipv6 {
			if props.ipv6prefixlen == 0 {
				warn!("IPv6 prefix length for domain {domain} is zero, ignoring update to IPv6 address");
			} else {
				let assembled_addr =
					splice_ipv6_addrs(props.ipv6prefixlen, prefix, props.ipv6suffix);
				let ipv6 = &assembled_addr.to_string();
				command.push_str(
					config
						.update_program
						.ipv6
						.stdin
						.replace("{domain}", domain)
						.replace("{ttl}", ttl)
						.replace("{ipv6}", ipv6)
						.as_str(),
				);
			};
		}
		command.push_str(config.update_program.stdin_per_zone_update.as_str());
	}
	command.push_str(config.update_program.final_stdin.as_str());
	debug!("Commands for update program:\n{command}");
	command
}

pub fn update(config: &Config, q: &QueryParameters) -> Result<impl Reply, impl Reply> {
	info!("Incoming request from user `{}`", &q.user);
	debug!("domain: {:?}, user: {:?}, pass: <redacted>, ipv4: {:?}, ipv6: {:?}, dualstack: {:?}, ipv6lanprefix: {:?}", &q.domain, &q.user, &q.ipv4, &q.ipv6, &q.dualstack, &q.ipv6lanprefix);

	let Some(user) = config.users.get(&q.user) else {
		eprintln!("User {} does not exist.", q.user);
		return Err(warp::reply::with_status(
			"Not authorized".to_string(),
			StatusCode::FORBIDDEN,
		));
	};

	if let Err(e) = Argon2::default().verify_password(q.pass.as_bytes(), &user.hash) {
		eprintln!("Error verifying password: {e}");
		return Err(warp::reply::with_status(
			"Not authorized".to_string(),
			StatusCode::FORBIDDEN,
		));
	};

	info!("Authentication successful");

	let command = build_command_string(config, user, q);

	let mut child = match Command::new(&config.update_program.bin)
		.args(&config.update_program.args)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
	{
		Ok(v) => v,
		Err(e) => {
			return Err(warp::reply::with_status(
				e.to_string(),
				StatusCode::INTERNAL_SERVER_ERROR,
			))
		}
	};

	if let Some(mut stdin) = child.stdin.take() {
		if let Err(e) = stdin.write_all(command.as_bytes()) {
			return Err(warp::reply::with_status(
				e.to_string(),
				StatusCode::INTERNAL_SERVER_ERROR,
			));
		};
	}

	let output = match child.wait_with_output() {
		Ok(v) => v,
		Err(e) => {
			return Err(warp::reply::with_status(
				e.to_string(),
				StatusCode::INTERNAL_SERVER_ERROR,
			))
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
