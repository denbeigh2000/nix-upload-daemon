use std::error::Error;
use std::path::PathBuf;

use async_channel::{Sender, Receiver};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio_util::sync::CancellationToken;


#[derive(thiserror::Error, Debug)]
pub enum ServeError {}

#[derive(thiserror::Error, Debug)]
pub enum WorkerError {
    #[error("error reading connection: {0}")]
    ReadingConnection(std::io::Error),
}

async fn handle_conn<C>(conn: C, sender: Sender<PathBuf>) -> Result<(), WorkerError>
where
    C: AsyncRead + Send + Unpin + 'static,
{
    let mut buf_read = BufReader::new(conn).lines();

    loop {
        let line = match buf_read
            .next_line()
            .await
            .map_err(WorkerError::ReadingConnection)?
        {
            Some(l) => l,
            None => return Ok(()),
        };

        let path: PathBuf = line.parse().expect("this error is infallible");
        if !path.exists() {
            eprintln!("path does not exist: {{path.to_string_lossy()}}");
            continue;
        }

        sender.send(path).await.expect("no receivers available => deadlock");
    }
}

async fn work(copy_dest: String, r: Receiver<PathBuf>)
{
    loop {
        let path = match r.recv().await{
            Ok(p) => p,
            // Closed
            Err(_) => return,
        };

        let mut cmd = tokio::process::Command::new("nix");
        cmd.arg("copy").arg(&path).arg("--to").arg(&copy_dest);
        let status = match cmd.status().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error forking: {e}");
                return;
            },
        };

        if !status.success() {
            eprintln!("error sending to store, exited with status {status}");
        }
    }
}

pub async fn serve<S>(cancel: CancellationToken, copy_dest: String, workers: u8, mut source: S) -> Result<(), ServeError>
where
    S: ConnectionSource + 'static,
{
    let (s, r) = async_channel::unbounded::<PathBuf>();

    for _ in 0..workers {
        let copy_dest = copy_dest.clone();
        let r = r.clone();
        tokio::spawn(async move {
            work(copy_dest, r).await;
        });
    }

    loop {
        let conn = tokio::select! {
            result = source.get_connection() => {
                match result {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("error accepting connection: {e}");
                        return Ok(());
                    },
                }
            },
            _ = cancel.cancelled() => {
                // We are shutting down
                return Ok(());
            }
        };
        // let conn = match source.get_connection().await {
        // };

        let s = s.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(conn, s).await {
                eprintln!("error handling connection: {e}");
            }
        });
    }
}

#[async_trait]
pub trait ConnectionSource: Send {
    type Connection: AsyncRead + Send + Unpin + 'static;
    type Error: Error + Sized + Send + 'static;

    async fn get_connection(&mut self) -> Result<Self::Connection, Self::Error>;
}

#[async_trait]
impl ConnectionSource for UnixListener {
    type Connection = UnixStream;
    type Error = std::io::Error;

    async fn get_connection(&mut self) -> Result<Self::Connection, Self::Error> {
        let (socket, _) = self.accept().await?;
        Ok(socket)
    }
}

#[async_trait]
impl ConnectionSource for TcpListener {
    type Connection = TcpStream;
    type Error = std::io::Error;

    async fn get_connection(&mut self) -> Result<Self::Connection, Self::Error> {
        let (socket, _) = self.accept().await?;
        Ok(socket)
    }
}
