use std::path::PathBuf;

use clap::Parser;
use tokio::net::{TcpListener, UnixListener};
use tokio_util::sync::CancellationToken;

mod daemon;

#[derive(Parser)]
pub struct UploadSubcommand {
    #[arg(short, long, env)]
    sign_key: Option<PathBuf>,
}

#[derive(Parser)]
pub struct ServeSubcommand {
    #[arg(short, long, env, group = "exposure")]
    port: Option<u16>,
    #[arg(short, long, env, group = "exposure")]
    unix_socket: Option<PathBuf>,

    #[arg(short, long, env, default_value = "2", value_parser = clap::value_parser!(u8).range(1..64))]
    workers: Option<u8>,

    #[arg(short, long, env)]
    copy_destination: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ServeError {
    #[error("neither a TCP port nor a unix socket path given")]
    NoBindingSpecified,
    #[error("error creating a listener: {0}")]
    MakingListener(std::io::Error),
    #[error("error accepting a connection: {0}")]
    AcceptingConnection(std::io::Error),
    #[error("error serving daemon: {0}")]
    Serving(#[from] daemon::ServeError),
}

pub async fn serve(cancel: CancellationToken, args: ServeSubcommand) -> Result<(), ServeError> {
    let dest = args.copy_destination;
    let workers = args.workers.unwrap_or(4);
    match &(args.port, args.unix_socket) {
        (Some(p), None) => {
            let listener = TcpListener::bind(format!("127.0.0.1:{p}"))
                .await
                .map_err(ServeError::MakingListener)?;

            daemon::serve(cancel, dest, workers, listener).await?;
        }

        (None, Some(p)) => {
            let listener = UnixListener::bind(p)
                .map_err(ServeError::MakingListener)?;

            daemon::serve(cancel, dest, workers, listener).await?;
        }

        (None, None) => return Err(ServeError::NoBindingSpecified),
        (Some(_), Some(_)) => unreachable!("enforced by clap"),
    };

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum UploadError {}

pub async fn upload(cancel: CancellationToken, args: UploadSubcommand) -> Result<(), UploadError> {
    todo!()
}
