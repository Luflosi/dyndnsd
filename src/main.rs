// SPDX-FileCopyrightText: 2024 Luflosi <dyndnsd@luflosi.de>
// SPDX-License-Identifier: AGPL-3.0-only

mod config;
mod logging;
mod process;

use crate::config::Config;
use crate::process::{update, QueryParameters};
use clap::Parser;
use color_eyre::eyre::{eyre, Result, WrapErr};
use listenfd::ListenFd;
use log::info;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
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

	let mut listenfd = ListenFd::from_env();

	let listen = config.listen;
	let update = warp::get()
		.and(warp::path("update"))
		.and(warp::path::end())
		.and(warp::query::<QueryParameters>())
		.map(move |q: QueryParameters| update(&config, &q));

	let server = warp::serve(update);
	let listener_count = listenfd.len();
	if let Some(listen) = listen {
		if listener_count != 0 {
			return Err(eyre!("According to the config file, we should listen on a TCP socket. But we were also passed an already opened socket as a file descriptor. Either remove the relevant section in the config file or don't let e.g. systemd pass a socket."));
		}
		info!("Listening on {listen}");
		server.run(listen).await;
	} else {
		if listener_count == 0 {
			return Err(eyre!("Don't know where to listen. The config file does not specify where to listen and nobody gave us an already file descriptor."));
		}
		if listener_count > 1 {
			return Err(eyre!(
				"We were given multiple file descriptors but only know how to handle one"
			));
		}
		info!("Using already opened Unix domain socket");
		let std_listener = match listenfd.take_unix_listener(0) {
			Ok(Some(v)) => v,
			Ok(None) => return Err(eyre!("No Unix domain socket was passed to dyndnsd")),
			Err(v) => return Err(v).wrap_err("The file descriptor handed to us is not a UNIX stream socket. Maybe it is a TCP socket, which is not supported (yet)"),
		};
		let listener = UnixListener::from_std(std_listener)
			.wrap_err("Cannot convert std::os::unix::net::UnixListener to UnixListener")?;
		let incoming = UnixListenerStream::new(listener);
		server.run_incoming(incoming).await;
	}

	Ok(())
}
