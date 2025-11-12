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

use crate::{cli::Cli, protocol::Command, system::System};

mod api;
mod cli;
mod protocol;
mod system;

fn main() -> Result<()> {
    color_eyre::install().unwrap();
    let options = Arc::new(cli::Cli::parse());
    let system = Arc::new(Mutex::new(System {}));

    let listener = TcpListener::bind("0.0.0.0:6600")?;
    for stream in listener.incoming() {
        let stream = stream.wrap_err("Could not accept connection")?;
        dbg!(stream.peer_addr());
        let writer = stream.try_clone().wrap_err("Clone failed")?;
        let reader = BufReader::new(stream).lines();
        let options = Arc::clone(&options);
        let system = Arc::clone(&system);
        thread::spawn(move || {
            if let Err(e) = handle_client(reader, writer, system, options) {
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
    options: Arc<Cli>,
) -> Result<()> {
    if let Some(addr) = &options.proxy {
        return handle_by_proxying(reader, writer, addr);
    }

    writer
        .write_all(format!("OK MPD {}\n", protocol::VERSION).as_bytes())
        .wrap_err("Could not send handshake to client")?;

    for line in reader {
        let request = Command::parse(&line?)?;
        let response = api::perform_request(request, &system);
        writer
            .write_all(&response)
            .wrap_err("Failed to write response to client")?;
    }
    Ok(())
}

fn handle_by_proxying(
    mut client_reader: impl Iterator<Item = io::Result<String>> + Send + 'static,
    mut client_writer: impl io::Write + Send + 'static,
    addr: &str,
) -> Result<()> {
    let stream = TcpStream::connect(addr)
        .wrap_err("Failed to connect to mpd_server")
        .with_note(|| format!("address: {addr}"))?;
    let mut server_writer = stream.try_clone().wrap_err("Clone failed")?;
    let mut server_reader = BufReader::new(stream).lines();

    let t1: JoinHandle<Result<()>> = thread::spawn(move || {
        loop {
            let response_line = server_reader
                .next()
                .ok_or_eyre("server closed the connection")?
                .wrap_err("Error reading reply from mpd server")?;
            eprintln!("server: {response_line}");
            client_writer
                .write_fmt(format_args!("{response_line}\n"))
                .wrap_err("Failed to forward server reply")?;
        }
    });

    let t2: JoinHandle<Result<()>> = thread::spawn(move || {
        loop {
            let request_line = client_reader
                .next()
                .ok_or_eyre("client closed the connection")?
                .wrap_err("Error reading request from mpd client")?;
            eprintln!("(***************************** (for readablity not part of proto)");
            eprintln!("client: {request_line}");
            eprintln!("(***************************** (for readablity not part of proto)");
            server_writer
                .write_fmt(format_args!("{request_line}\n"))
                .wrap_err("Could not forward line to mpd_server")?;
        }
    });

    t1.join().unwrap().unwrap();
    t2.join().unwrap().unwrap();
    Ok(())
}
