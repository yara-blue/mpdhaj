use color_eyre::{
    Result, Section,
    eyre::{Context, OptionExt},
};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread::{self, JoinHandle},
};

pub fn handle_clients(addr: &str) -> Result<()> {
    let addr: Arc<str> = addr.into();
    let listener = TcpListener::bind("0.0.0.0:6600")?;
    for stream in listener.incoming() {
        let stream = stream.wrap_err("Could not accept connection")?;
        let writer = stream.try_clone().wrap_err("Clone failed")?;
        let reader = BufReader::new(stream).lines();
        let addr = addr.clone();
        thread::spawn(move || {
            if let Err(e) = handle(reader, writer, addr) {
                eprintln!("error handling client: {e:?}");
            }
        });
    }

    Ok(())
}

fn handle(
    mut client_reader: impl Iterator<Item = io::Result<String>> + Send + 'static,
    mut client_writer: impl io::Write + Send + 'static,
    addr: Arc<str>,
) -> Result<()> {
    let stream = TcpStream::connect(&*addr)
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

            // here to experiment if this is allowed by most clients
            if response_line.contains("lastloadedplaylist") {
                eprintln!("skipping: {response_line}");
                continue;
            }
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
