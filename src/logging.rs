// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

use env_logger::{Builder, Env};
use std::io::Write;

pub fn setup() {
	let env = Env::default().filter_or("RUST_LOG", "dyndnsd=info");

	match std::env::var("RUST_LOG_STYLE") {
		Ok(s) if s == "SYSTEMD" => Builder::from_env(env)
			.format(|buf, record| {
				for line in record.args().to_string().lines() {
					writeln!(
						buf,
						"<{}>{}: {}",
						match record.level() {
							log::Level::Error => 3,
							log::Level::Warn => 4,
							log::Level::Info => 6,
							log::Level::Debug | log::Level::Trace => 7,
						},
						record.target(),
						line
					)?;
				}
				Ok(())
			})
			.init(),
		_ => env_logger::init_from_env(env),
	}
}
