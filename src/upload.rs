use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
};

use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("path does not exist: {0}")]
    MissingPath(PathBuf),
    #[error("path cannot be represented as utf-8: {0}")]
    UnrepresentablePath(PathBuf),
    #[error("key does not exist at {0}")]
    MissingKey(PathBuf),
    #[error("error running `nix store sign`: {0}")]
    ForkingSignProcess(std::io::Error),
    #[error("signing process failed with status {0}")]
    CouldNotSign(ExitStatus),
    #[error("error writing data: {0}")]
    IO(#[from] std::io::Error),
    #[error("the operation was cancelled by the user")]
    Cancelled,
}

pub async fn upload<Sink>(
    cancel: CancellationToken,
    sink: Sink,
    paths: &[PathBuf],
    key_path: Option<&Path>,
) -> Result<(), Error>
where
    Sink: AsyncWrite + Send + Sync + Unpin + 'static,
{
    let paths = paths
        .iter()
        .map(|p| {
            if !p.exists() {
                return Err(Error::MissingPath(p.to_owned()));
            }

            match p.to_str() {
                Some(s) => Ok(s),
                None => Err(Error::UnrepresentablePath(p.to_owned())),
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    if let Some(k_path) = key_path {
        if !k_path.exists() {
            return Err(Error::MissingKey(k_path.to_owned()));
        }
        let mut cmd = tokio::process::Command::new("nix");

        cmd.arg("store").arg("sign").arg("--key-file").arg(k_path);

        let status = cmd.status().await.map_err(Error::ForkingSignProcess)?;
        if !status.success() {
            return Err(Error::CouldNotSign(status));
        }
    }

    let mut buf = BufWriter::new(sink);
    for path in paths {
        buf.write_all(path.as_bytes()).await?;
        buf.write_all(&[b'\n']).await?;
    }

    tokio::select! {
        res = buf.flush() => res.map_err(Error::IO),
        _ = cancel.cancelled() => Err(Error::Cancelled),
    }
}
