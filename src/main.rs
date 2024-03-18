// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate error_chain;

use crate::config::read;
use crate::process::{update, QueryParameters};
use clap::Parser;
use warp::Filter;

mod config;
mod process;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
	/// Path to the config file
	#[arg(short, long, default_value = "config.toml")]
	config: std::path::PathBuf,
}

#[tokio::main]
async fn main() {
	let args = Args::parse();

	let config = match read(&args.config) {
		Ok(v) => v,
		Err(e) => {
			eprintln!("ERROR: {e}");

			/////// look at the chain of errors... ///////
			for e in e.iter().skip(1) {
				eprintln!("caused by: {e}");
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

	println!("Listening on {listen}");
	warp::serve(update).run(listen).await;
}
