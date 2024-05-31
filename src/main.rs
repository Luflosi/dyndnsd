// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

mod config;
mod logging;
mod process;

use crate::config::Config;
use crate::process::{update, QueryParameters};
use clap::Parser;
use color_eyre::eyre::Result;
use log::info;
use warp::Filter;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
	/// Path to the config file
	#[arg(short, long, default_value = "config.toml")]
	config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	logging::setup();

	let args = Args::parse();

	let config = Config::read(&args.config)?;

	let listen = config.listen;
	let update = warp::get()
		.and(warp::path("update"))
		.and(warp::path::end())
		.and(warp::query::<QueryParameters>())
		.map(move |q: QueryParameters| update(&config, &q));

	info!("Listening on {listen}");
	warp::serve(update).run(listen).await;

	Ok(())
}
