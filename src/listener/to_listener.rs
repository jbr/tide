use super::UnixListener;
use super::{MultiListener, TcpListener};
use crate::http::url::Url;
use crate::Server;
use async_std::io;
use std::net::ToSocketAddrs;

/// ToListener represents any type that can be converted into a
/// [`Listener`](crate::listener::Listener).  Any type that implements
/// ToListener can be passed to [`Server::listen`](crate::Server::listen) or
/// added to a [`MultiListener`](crate::listener::MultiListener)
///
/// # Example strings on all platforms include:
/// * `tcp://localhost:8000`
/// * `tcp://0.0.0.0` (binds to port 80 by default)
/// * `http://localhost:8000` (http is an alias for tcp)
/// * `http://127.0.0.1:8000` (or `0.0.0.0`, or some specific bindable ip)
/// * `127.0.0.1:3000` (or any string that can be parsed as a [SocketAddr](std::net::SocketAddr))
/// * `[::1]:1213` (an ipv6 [SocketAddr](std::net::SocketAddr))
///
/// # Strings supported only on `cfg(unix)` platforms:
/// * `unix:///var/run/tide/socket` (absolute path)
/// * `unix://socket` (relative path)
/// * `unix://./socket.file` (also relative path)
/// * `unix://../socket` (relative path)
/// * any of the above with the alternate schemes of `file://` or `http+unix://`
///
/// # String supported only on windows:
/// * `:3000` (binds to port 3000)
///
/// # Specifying multiple listeners:
/// To bind to any number of listeners concurrently:
/// ```rust,no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// # let app = tide::new();
/// app.listen(vec!["tcp://localhost:8000", "tcp://localhost:8001"]).await?;
/// # Ok(()) }) }
/// ```
///
/// # Multiple socket resolution
/// If a TCP listener resolves to multiple socket addresses, tide will
/// bind to the first successful one. For example, on ipv4+ipv6
/// systems, `tcp://localhost:1234` resolves both to `127.0.0.1:1234`
/// (v4) as well as `[::1]:1234` (v6). The order that these are
/// attempted is platform-determined. To listen on all of the addresses, use
/// ```rust,no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// # let app = tide::new();
/// use std::net::ToSocketAddrs;
/// app.listen("localhost:8000".to_socket_addrs()?.collect::<Vec<_>>()).await?;
/// # Ok(()) }) }
/// ```
/// # Other implementations
/// See below for additional provided implementations of ToListener.

pub(crate) type ResultFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>>;

pub trait Listener<State: Send + Sync + 'static> {
    fn listen(self, app: Server<State>) -> ResultFuture;
}

impl<State: Send + Sync + 'static> Listener<State> for Url {
    fn listen(self, app: Server<State>) -> ResultFuture {
        Box::pin(async move {
            match self.scheme() {
                "unix" | "file" | "http+unix" => {
                    #[cfg(unix)]
                    {
                        let path = std::path::PathBuf::from(format!(
                            "{}{}",
                            self.domain().unwrap_or_default(),
                            self.path()
                        ));

                        UnixListener::from_path(path).listen(app).await
                    }

                    #[cfg(not(unix))]
                    {
                        Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Unix sockets not supported on this platform",
                        ))
                    }
                }

                "tcp" | "http" => {
                    TcpListener::from_addrs(self.socket_addrs(|| Some(80))?)
                        .listen(app)
                        .await
                }

                "tls" | "ssl" | "https" => Err(io::Error::new(
                    io::ErrorKind::Other,
                    "parsing TLS listeners not supported yet",
                )),

                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unrecognized url scheme",
                )),
            }
        })
    }
}

impl<State: Send + Sync + 'static> Listener<State> for String {
    fn listen(self, app: Server<State>) -> ResultFuture {
        Box::pin(async move {
            if let Ok(socket_addrs) = self.to_socket_addrs() {
                return TcpListener::from_addrs(socket_addrs.collect())
                    .listen(app)
                    .await;
            }

            match Url::parse(&self) {
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unable to parse listener from `{}`", self),
                )),
                Ok(url) => Listener::<State>::listen(url, app).await,
            }
        })
    }
}

impl<State: Send + Sync + 'static> Listener<State> for &'static str {
    fn listen(self, app: Server<State>) -> ResultFuture {
        Listener::<State>::listen(self.to_owned(), app)
    }
}

#[cfg(unix)]
impl<State: Send + Sync + 'static> Listener<State> for async_std::path::PathBuf {
    fn listen(self, app: Server<State>) -> ResultFuture {
        UnixListener::from_path(self.clone()).listen(app)
    }
}

#[cfg(unix)]
impl<State: Send + Sync + 'static> Listener<State> for std::path::PathBuf {
    fn listen(self, app: Server<State>) -> ResultFuture {
        UnixListener::from_path(self.clone()).listen(app)
    }
}

impl<State: Send + Sync + 'static> Listener<State> for async_std::net::TcpListener {
    fn listen(self, app: Server<State>) -> ResultFuture {
        TcpListener::from_listener(self).listen(app)
    }
}

impl<State: Send + Sync + 'static> Listener<State> for std::net::TcpListener {
    fn listen(self, app: Server<State>) -> ResultFuture {
        TcpListener::from_listener(self).listen(app)
    }
}

impl<State: Send + Sync + 'static> Listener<State> for (&'static str, u16) {
    fn listen(self, app: Server<State>) -> ResultFuture {
        Box::pin(async move {
            TcpListener::from_addrs(self.to_socket_addrs()?.collect())
                .listen(app)
                .await
        })
    }
}

#[cfg(unix)]
impl<State: Send + Sync + 'static> Listener<State> for async_std::os::unix::net::UnixListener {
    fn listen(self, app: Server<State>) -> ResultFuture {
        UnixListener::from_listener(self).listen(app)
    }
}

#[cfg(unix)]
impl<State: Send + Sync + 'static> Listener<State> for std::os::unix::net::UnixListener {
    fn listen(self, app: Server<State>) -> ResultFuture {
        UnixListener::from_listener(self).listen(app)
    }
}

impl<State: Send + Sync + 'static> Listener<State> for std::net::SocketAddr {
    fn listen(self, app: Server<State>) -> ResultFuture {
        TcpListener::from_addrs(vec![self]).listen(app)
    }
}

impl<TL: Listener<State> + Send + Sync + 'static, State: Send + Sync + 'static> Listener<State>
    for Vec<TL>
{
    fn listen(mut self, app: Server<State>) -> ResultFuture {
        Box::pin(async move {
            let mut multi = MultiListener::new(app.clone());
            for listener in self.drain(..) {
                multi.bind(listener);
            }
            multi.listen(app).await
        })
    }
}

// #[cfg(test)]
// mod parse_tests {
//     use super::*;

//     fn listen<TL: Listener<()>>(listener: TL) -> io::Result<TL::Listener> {
//         listener.listen()
//     }

//     #[test]
//     fn url_to_tcp_listener() {
//         let listener = listen(Url::parse("http://localhost:8000").unwrap()).unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert!(listener.to_string().contains("http://127.0.0.1:8000"));

//         let listener = listen(Url::parse("tcp://localhost:8000").unwrap()).unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert!(listener.to_string().contains("http://127.0.0.1:8000"));

//         let listener = listen(Url::parse("http://127.0.0.1").unwrap()).unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert_eq!(listener.to_string(), "http://127.0.0.1:80");
//     }

//     #[test]
//     fn str_url_to_tcp_listener() {
//         let listener = listen("tcp://localhost:8000").unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert!(listener.to_string().contains("http://127.0.0.1:8000"));

//         let listener = listen("tcp://localhost:8000").unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert!(listener.to_string().contains("http://127.0.0.1:8000"));

//         let listener = listen("tcp://127.0.0.1").unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert_eq!(listener.to_string(), "http://127.0.0.1:80");
//     }

//     #[cfg(unix)]
//     mod unix {
//         use super::*;

//         #[test]
//         fn str_url_to_unix_listener() {
//             let listener = listen("unix:///var/run/tide/socket").unwrap();
//             assert!(matches!(
//                 listener,
//                 ParsedListener::Unix(UnixListener::FromPath(_, None))
//             ));
//             assert_eq!("unix:///var/run/tide/socket", listener.to_string());

//             let listener = listen("unix://./socket").unwrap();
//             assert!(matches!(
//                 listener,
//                 ParsedListener::Unix(UnixListener::FromPath(_, None))
//             ));
//             assert_eq!("unix://./socket", listener.to_string());

//             let listener = listen("unix://socket").unwrap();
//             assert!(matches!(
//                 listener,
//                 ParsedListener::Unix(UnixListener::FromPath(_, None))
//             ));
//             assert_eq!("unix://socket", listener.to_string());
//         }

//         #[test]
//         fn colon_port_does_not_work() {
//             let err = listen(":3000").unwrap_err().to_string();
//             assert_eq!(err, "unable to parse listener from `:3000`");
//         }
//     }

//     #[cfg(not(unix))]
//     mod not_unix {
//         use super::*;
//         #[test]
//         fn str_url_to_unix_listener() {
//             let err = listen("unix:///var/run/tide/socket").unwrap_err();
//             assert_eq!(
//                 err.to_string(),
//                 "Unix sockets not supported on this platform"
//             );
//         }

//         #[test]
//         fn colon_port_works() {
//             let listener = listen(":3000").unwrap();
//             assert!(listener.to_string().ends_with(":3000"));
//             assert!(listener.to_string().starts_with("http://"));
//         }
//     }

//     #[test]
//     fn str_tls_parse_and_url() {
//         let err = listen("tls://localhost:443").unwrap_err();
//         assert_eq!(err.to_string(), "parsing TLS listeners not supported yet");

//         let err = listen(Url::parse("https://localhost:443").unwrap()).unwrap_err();
//         assert_eq!(err.to_string(), "parsing TLS listeners not supported yet");
//     }

//     #[test]
//     fn str_unknown_scheme() {
//         let err = listen("pigeon://localhost:443").unwrap_err();
//         assert_eq!(err.to_string(), "unrecognized url scheme");

//         let err = listen(Url::parse("pigeon:///localhost:443").unwrap()).unwrap_err();
//         assert_eq!(err.to_string(), "unrecognized url scheme");
//     }

//     #[test]
//     fn str_to_socket_addr() {
//         let listener = listen("127.0.0.1:1312").unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert_eq!("http://127.0.0.1:1312", listener.to_string());

//         let listener = listen("[::1]:1312").unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert_eq!("http://[::1]:1312", listener.to_string());

//         let listener = listen("localhost:3000").unwrap();
//         assert!(matches!(
//             listener,
//             ParsedListener::Tcp(TcpListener::FromAddrs(_, None))
//         ));
//         assert!(listener.to_string().contains(":3000"));
//     }

//     #[test]
//     fn invalid_str_input() {
//         let err = listen("hello world").unwrap_err();
//         assert_eq!(
//             err.to_string(),
//             "unable to parse listener from `hello world`"
//         );

//         let err = listen("🌊").unwrap_err();
//         assert_eq!(err.to_string(), "unable to parse listener from `🌊`");
//     }
// }
