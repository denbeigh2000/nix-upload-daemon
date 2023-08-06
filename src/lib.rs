use std::path::PathBuf;

use clap::Parser;
use tokio::task::JoinError;
use tokio_util::sync::CancellationToken;

pub mod binding;
mod daemon;
mod upload;

use binding::Binding;

#[derive(Parser)]
pub struct UploadSubcommand {
    #[arg(short, long, env)]
    bind: Binding,

    #[arg(short, long, env)]
    sign_key: Option<PathBuf>,

    #[arg(short, long, env = "OUT_PATHS")]
    paths: Vec<PathBuf>,
}

#[derive(Parser)]
pub struct ServeSubcommand {
    #[arg(short, long, env)]
    bind: Binding,

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
    let listener = args.bind.listen().await.map_err(ServeError::MakingListener)?;
    daemon::serve(cancel, dest, workers, listener).await?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("error writing to daemon: {0}")]
    WritingData(#[from] upload::Error),
    #[error("error creating connection: {0}")]
    ConnectingToDaemon(#[from] std::io::Error),
    #[error("error waiting on tasks to finish: {0}")]
    JoiningThread(#[from] JoinError),
}

pub async fn upload(cancel: CancellationToken, args: UploadSubcommand) -> Result<(), UploadError> {
    let paths: Vec<PathBuf> = args.paths.into_iter().filter(|p| p.exists()).collect();
    // TODO: Respect signals?
    let socket = args.bind.connect().await.map_err(UploadError::ConnectingToDaemon)?;
    upload::upload(cancel, socket, &paths, args.sign_key.as_deref()).await?;

    Ok(())
}
