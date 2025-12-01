use color_eyre::{
    Result, Section,
    eyre::{Context, OptionExt},
};
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio::task::JoinHandle;
use tracing::info;

pub async fn handle_clients(port: u16, addr: &str) -> Result<()> {
    let addr: Arc<str> = addr.into();
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    loop {
        let (stream, _) = listener.accept().await.wrap_err("Could not accept connection")?;
        let (reader, writer) = tokio::io::split(stream);
        let reader = BufReader::new(reader).lines();
        let addr = addr.clone();
        task::spawn(async move {
            if let Err(e) = handle(reader, writer, addr).await {
                info!("error handling client: {e:?}");
            }
        });
    }
}

async fn handle(
    mut client_reader: tokio::io::Lines<impl AsyncBufRead + Unpin + 'static>,
    mut client_writer: impl AsyncWrite + Send + 'static + Unpin,
    addr: Arc<str>,
) -> Result<()> {
    let stream = TcpStream::connect(&*addr)
        .await
        .wrap_err("Failed to connect to mpd_server")
        .with_note(|| format!("address: {addr}"))?;
    let (server_reader, mut server_writer) = tokio::io::split(stream);
    let mut server_reader = BufReader::new(server_reader).lines();

    let t1: JoinHandle<Result<()>> = task::spawn_local(async move {
        loop {
            let response_line = server_reader
                .next_line()
                .await
                .wrap_err("Error reading reply from mpd server")?
                .ok_or_eyre("server closed the connection")?;

            // here to experiment if this is allowed by most clients
            if response_line.contains("lastloadedplaylist") {
                info!("skipping: {response_line}");
                continue;
            }
            println!("server: {response_line}");
            let response = format!("{response_line}\n");
            client_writer
                .write_all(response.as_bytes())
                .await
                .wrap_err("Failed to forward server reply")?;
        }
    });

    let t2: JoinHandle<Result<()>> = task::spawn_local(async move {
        loop {
            let request_line = client_reader
                .next_line()
                .await
                .wrap_err("Error reading request from mpd client")?
                .ok_or_eyre("client closed the connection")?;
            println!("(***************************** (for readablity not part of proto)");
            println!("client: {request_line}");
            println!("(***************************** (for readablity not part of proto)");
            let request = format!("{request_line}\n");
            server_writer
                .write_all(request.as_bytes())
                .await
                .wrap_err("Could not forward line to mpd_server")?;
        }
    });

    t1.await.unwrap().unwrap();
    t2.await.unwrap().unwrap();
    Ok(())
}
