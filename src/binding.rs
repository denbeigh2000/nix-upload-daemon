use std::fs::Permissions;
use std::net::SocketAddr;
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;

use clap::error::ErrorKind;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Clone)]
pub struct BindingParser;

#[derive(Clone)]
pub enum Binding {
    Unix(PathBuf),
    Tcp(SocketAddr),
}

impl Binding {
    pub async fn listen(self) -> Result<Listener, std::io::Error> {
        match self {
            Binding::Tcp(addr) => TcpListener::bind(addr).await.map(Listener::Tcp),
            Binding::Unix(path) => {
                if path.exists() {
                    tokio::fs::remove_file(&path).await?;
                }
                let listener = UnixListener::bind(&path).map(Listener::Unix)?;
                let perms = Permissions::from_mode(0o0666);
                tokio::fs::set_permissions(path, perms).await?;
                Ok(listener)
            },
        }
    }

    pub async fn connect(self) -> Result<Connection, std::io::Error> {
        match self {
            Binding::Tcp(addr) => TcpStream::connect(addr).await.map(Connection::Tcp),
            Binding::Unix(path) => UnixStream::connect(path).await.map(Connection::Unix),
        }
    }
}

pub enum Listener {
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl Listener {
    pub async fn get_connection(
        &mut self,
        cancel: CancellationToken,
    ) -> Result<Option<Connection>, std::io::Error> {
        let cancel_fut = cancel.cancelled();
        let conn = match self {
            Self::Unix(l) => {
                tokio::select! {
                     pair = l.accept() => Connection::Unix(pair?.0),
                    _ = cancel_fut => return Ok(None),
                }
            }
            Self::Tcp(l) => {
                tokio::select! {
                    pair = l.accept() => Connection::Tcp(pair?.0),
                    _ = cancel_fut => return Ok(None),
                }
            }
        };

        Ok(Some(conn))
    }
}

pub enum Connection {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl AsyncRead for Connection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(t) => Pin::new(t).poll_read(cx, buf),
            Self::Unix(u) => Pin::new(u).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            Self::Tcp(t) => Pin::new(t).poll_write(cx, buf),
            Self::Unix(u) => Pin::new(u).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            Self::Tcp(t) => Pin::new(t).poll_flush(cx),
            Self::Unix(u) => Pin::new(u).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            Self::Tcp(t) => Pin::new(t).poll_shutdown(cx),
            Self::Unix(u) => Pin::new(u).poll_shutdown(cx),
        }
    }
}

impl clap::builder::TypedValueParser for BindingParser {
    type Value = Binding;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = match value.to_str() {
            Some(v) => v,
            None => {
                let mut cmd = cmd.to_owned();
                return Err(cmd.error(ErrorKind::InvalidUtf8, "Binding is not parseable"));
            }
        };

        value.parse().map_err(|e: ParseBindingError| {
            let mut cmd = cmd.to_owned();
            cmd.error(
                ErrorKind::InvalidValue,
                format!("error parsing binding: {e}"),
            )
        })
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum ParseBindingError {
    #[error("not a valid url: {0}")]
    Url(#[from] url::ParseError),
    #[error("not a valid tcp address: {0}")]
    SocketAddr(#[from] std::net::AddrParseError),
    #[error("{0} is not a valid scheme, valid schemes are 'tcp', 'tcp4', 'tcp6' and 'sock'")]
    Scheme(String),
}

impl FromStr for Binding {
    type Err = ParseBindingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: Url = s.parse()?;
        match url.scheme() {
            "sock" => {
                let path = url.path().parse().expect("this error is infallible");
                Ok(Self::Unix(path))
            }
            "tcp" | "tcp4" | "tcp6" => {
                let addr = s.parse()?;
                Ok(Self::Tcp(addr))
            }
            scheme => Err(ParseBindingError::Scheme(scheme.to_string())),
        }
    }
}
