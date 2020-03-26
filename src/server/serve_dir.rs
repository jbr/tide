use async_std::fs::File;
use async_std::io::BufReader;
use http_types::StatusCode;

use crate::{Endpoint, Request, Response};

use std::path::PathBuf;

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a + Send>>;
pub struct ServeDir {
    prefix: String,
    dir: PathBuf,
}

impl ServeDir {
    /// Create a new instance of `ServeDir`.
    pub(crate) fn new(prefix: String, dir: PathBuf) -> Self {
        Self { prefix, dir }
    }
}

impl<State> Endpoint<State> for ServeDir {
    fn call<'a>(&'a self, req: Request<State>) -> BoxFuture<'a, Response> {
        let path = req.uri().path();
        let path = path.replace(&self.prefix, "");
        let path = path.trim_start_matches('/');
        let dir = self.dir.clone();
        let dir = dir.join(&path);
        log::info!("Requested file: {:?}", dir);

        Box::pin(async move {
            let file = match async_std::fs::canonicalize(&dir).await {
                Err(_) => {
                    log::info!("File not found: {:?}", dir);
                    return Response::new(StatusCode::NotFound);
                }
                Ok(mut file_path) => {
                    // Verify this is a sub-path of the original dir.
                    let mut file_iter = (&mut file_path).iter();
                    for lhs in &dir {
                        if Some(lhs) != file_iter.next() {
                            return Response::new(StatusCode::Forbidden);
                        }
                    }

                    // Open the file and send back the contents.
                    match File::open(&file_path).await {
                        Ok(file) => file,
                        Err(_) => {
                            log::warn!("Could not open {:?}", file_path);
                            return Response::new(StatusCode::Forbidden);
                        }
                    }
                }
            };

            // TODO: fix related bug where async-h1 crashes on large files
            Response::new(StatusCode::Ok).body(BufReader::new(file))
        })
    }
}
