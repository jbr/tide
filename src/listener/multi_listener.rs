use crate::listener::{Listener, ResultFuture};
use crate::Server;
use async_std::{io, task};

use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};

/// MultiListener allows tide to listen on any number of transports
/// simultaneously (such as tcp ports, unix sockets, or tls).
///
/// # Example:
/// ```rust
/// fn main() -> Result<(), std::io::Error> {
///    async_std::task::block_on(async {
///        tide::log::start();
///        let mut app = tide::new();
///        app.at("/").get(|_| async { Ok("Hello, world!") });
///
///        let mut multi = tide::listener::MultiListener::new();
///        multi.bind("127.0.0.1:8000")?;
///        multi.bind(async_std::net::TcpListener::bind("127.0.0.1:8001").await?)?;
/// # if cfg!(unix) {
///        multi.bind("unix://unix.socket")?;
/// # }
///    
/// # if false {
///        app.listen(multi).await?;
/// # }
///        Ok(())
///    })
///}
///```

pub struct MultiListener<State>(
    Server<State>,
    FuturesUnordered<task::JoinHandle<io::Result<()>>>,
);

impl<State> std::fmt::Debug for MultiListener<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiListener").finish()
    }
}

impl<State: Send + Sync + 'static> MultiListener<State> {
    pub fn new(app: Server<State>) -> Self {
        Self(app, FuturesUnordered::new())
    }

    pub fn bind<TL: Listener<State> + Send + Sync + 'static>(&mut self, listener: TL) {
        self.1.push(task::spawn(listener.listen(self.0.clone())));
    }
}

impl<State: Send + Sync + 'static> Listener<State> for MultiListener<State> {
    fn listen(mut self, _: Server<State>) -> ResultFuture {
        Box::pin(async move {
            while let Some(result) = self.1.next().await {
                result?;
            }
            Ok(())
        })
    }
}
