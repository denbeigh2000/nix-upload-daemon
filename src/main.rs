use clap::{Parser, Subcommand};
use futures::{FutureExt, TryFutureExt};
use nix_upload_daemon::{
    serve, upload, ServeError, ServeSubcommand, UploadError, UploadSubcommand,
};
use tokio::task::JoinError;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Upload(UploadSubcommand),
    Serve(ServeSubcommand),
}

#[derive(thiserror::Error, Debug)]
enum MainError {
    #[error("{0}")]
    ParsingArgs(#[from] clap::Error),
    #[error("error serving: {0}")]
    Serving(#[from] ServeError),
    #[error("error enqueueing for upload: {0}")]
    Uploading(#[from] UploadError),
    #[error("error joining async task: {0}")]
    JoiningTask(#[from] JoinError),
}

#[tokio::main]
async fn main() {
    let mut status = 1;
    match real_main().await {
        Err(MainError::ParsingArgs(e)) => eprintln!("{e}"),
        Err(e) => eprintln!("error: {e}"),
        Ok(_) => {
            status = 0;
        }
    }

    std::process::exit(status);
}

fn into_main_err<E: Into<MainError>>(err: E) -> MainError {
    err.into()
}

async fn real_main() -> Result<(), MainError> {
    let cancel = CancellationToken::new();
    let local_cancel = cancel.clone();
    let fut = match CliArgs::try_parse()?.action {
        Action::Serve(s) => tokio::spawn(serve(cancel, s).map_err(into_main_err)),
        Action::Upload(u) => tokio::spawn(upload(cancel, u).map_err(into_main_err)),
    };

    let (fut, handle) = tokio::spawn(fut).remote_handle();

    tokio::select! {
        _ = fut => {
            handle.await???;
        },

        _ = tokio::signal::ctrl_c() => {
            local_cancel.cancel();
            // Wait for any operations to finish
            // TODO: Timeout?
            handle.await???;
        }
    }

    Ok(())
}
