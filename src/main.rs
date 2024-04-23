// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

mod config;
mod logging;
mod process;

#[macro_use]
extern crate error_chain;

use crate::config::Config;
use crate::process::{update, QueryParameters};
use clap::Parser;
use log::{error, info};
use warp::Filter;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
	/// Path to the config file
	#[arg(short, long, default_value = "config.toml")]
	config: std::path::PathBuf,
}

#[tokio::main]
async fn main() {
	logging::setup();

	let args = Args::parse();

	let config = match Config::read(&args.config) {
		Ok(v) => v,
		Err(e) => {
			error!("ERROR: {e}");

			/////// look at the chain of errors... ///////
			for e in e.iter().skip(1) {
				error!("caused by: {e}");
			}

			std::process::exit(1);
		}
	};

	let listen = config.listen;
	let update = warp::get()
		.and(warp::path("update"))
		.and(warp::path::end())
		.and(warp::query::<QueryParameters>())
		.map(move |q: QueryParameters| update(&config, &q));

	info!("Listening on {listen}");
	warp::serve(update).run(listen).await;
}
