use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use clap::Parser;
use color_eyre::{
    Result, Section,
    eyre::{Context, OptionExt},
};

use crate::{
    cli::{Cli, RunArgs},
    mpd_protocol::Command,
    system::System,
};

mod api;
mod cli;
mod playlist;
mod mpd_protocol;
mod proxy;
mod system;

fn main() -> Result<()> {
    color_eyre::install().unwrap();
    let options = cli::Cli::parse();

    match options.command {
        cli::Commands::Proxy { address } => proxy::handle_clients(&address)?,
        cli::Commands::Run(args) => {
            let system = Arc::new(Mutex::new(
                System::new(&args.playlist_dir).wrap_err("Could not start system")?,
            ));
            handle_clients(system)?;
        }
    };

    Ok(())
}

fn handle_clients(system: Arc<std::sync::Mutex<System>>) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6600")?;
    for stream in listener.incoming() {
        let stream = stream.wrap_err("Could not accept connection")?;
        let writer = stream.try_clone().wrap_err("Clone failed")?;
        let reader = BufReader::new(stream).lines();
        let system = Arc::clone(&system);
        thread::spawn(move || {
            if let Err(e) = handle_client(reader, writer, system) {
                eprintln!("error handling client: {e:?}");
            }
        });
    }
    Ok(())
}

fn handle_client(
    reader: impl Iterator<Item = io::Result<String>> + Send + 'static,
    mut writer: impl io::Write + Send + 'static,
    system: Arc<std::sync::Mutex<System>>,
) -> Result<()> {
    writer
        .write_all(format!("OK MPD {}\n", mpd_protocol::VERSION).as_bytes())
        .wrap_err("Could not send handshake to client")?;

    for line in reader {
        let request = Command::parse(&line?)?;
        eprintln!("parsed request: {request:?}");
        let response = api::perform_request(request, &system);
        writer
            .write_all(&response)
            .wrap_err("Failed to write response to client")?;
    }
    Ok(())
}
