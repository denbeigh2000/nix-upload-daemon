use std::path::PathBuf;
use std::process::ExitStatus;

use async_channel::{Receiver, Sender};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use tokio_util::sync::CancellationToken;

use crate::binding::Listener;

#[derive(thiserror::Error, Debug)]
pub enum ServeError {
    #[error("error listening for new connection: {0}")]
    GettingConnection(std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum WorkerError {
    #[error("error reading connection: {0}")]
    ReadingConnection(std::io::Error),
    #[error("error running `nix copy`: {0}")]
    ForkingUploadProcess(std::io::Error),
    #[error("uploading process failed with status {0}")]
    CouldNotUpload(ExitStatus),
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

        sender
            .send(path)
            .await
            .expect("no receivers available => deadlock");
    }
}

async fn work(copy_dest: &str, r: &Receiver<PathBuf>) -> Result<(), WorkerError> {
    loop {
        let item_path = match r.recv().await {
            Ok(p) => p,
            // Closed
            Err(_) => return Ok(()),
        };

        let mut cmd = tokio::process::Command::new("nix");
        cmd.arg("--experimental-features")
            .arg("flakes")
            .arg("--experimental-features")
            .arg("nix-command")
            .arg("copy")
            .arg(&item_path)
            .arg("--to")
            .arg(copy_dest)
            .env("NIX_SSHOPTS", "-oStrictHostKeyChecking=no -oUpdateHostKeys=no");
        let status = cmd
            .status()
            .await
            .map_err(WorkerError::ForkingUploadProcess)?;

        if !status.success() {
            return Err(WorkerError::CouldNotUpload(status));
        }
    }
}

pub async fn serve(
    cancel: CancellationToken,
    copy_dest: String,
    workers: u8,
    mut source: Listener,
) -> Result<(), ServeError> {
    let (s, r) = async_channel::unbounded::<PathBuf>();

    for _ in 0..workers {
        let copy_dest = copy_dest.clone();
        let r = r.clone();
        tokio::spawn(async move {
            while let Err(e) = work(&copy_dest, &r).await {
                eprintln!("worker raised error: {e}");
            }
        });
    }

    loop {
        let conn = match source.get_connection(cancel.clone()).await {
            Ok(Some(conn)) => conn,
            Ok(None) => return Ok(()),
            Err(e) => return Err(ServeError::GettingConnection(e)),
        };

        let s = s.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(conn, s).await {
                eprintln!("error handling connection: {e}");
            }
        });
    }
}
