//! Types that represent HTTP transports and binding

mod multi_listener;
mod tcp_listener;
mod to_listener;
#[cfg(unix)]
mod unix_listener;
use async_std::io;

pub use multi_listener::MultiListener;
pub use to_listener::Listener;
pub(crate) use to_listener::ResultFuture;

pub(crate) use tcp_listener::TcpListener;
#[cfg(unix)]
pub(crate) use unix_listener::UnixListener;

pub(crate) fn is_transient_error(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::ConnectionRefused
        || e.kind() == io::ErrorKind::ConnectionAborted
        || e.kind() == io::ErrorKind::ConnectionReset
}
