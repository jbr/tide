use async_std::fs::File;
use async_std::io::BufReader;
use async_std::task;
use http_types::StatusCode;

use std::io;
use tide::{Endpoint, Request, Response};

use std::path::{Path, PathBuf};

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a + Send>>;

fn main() -> Result<(), std::io::Error> {
    femme::start(log::LevelFilter::Info).unwrap();
    task::block_on(async {
        let mut app = tide::new();
        app.at("/").get(|_| async move { "Hello, world!" });
        serve_dir(&mut app.at("/foo"), "src/")?;
        app.listen("127.0.0.1:8080").await?;
        Ok(())
    })
}

fn serve_dir<State: 'static>(
    route: &mut tide::Route<State>,
    dir: impl AsRef<Path>,
) -> io::Result<()> {
    // Verify path exists, return error if it doesn't.
    let dir = dir.as_ref().to_owned().canonicalize()?;
    let serve = ServeDir {
        prefix: route.path().to_string(),
        dir,
    };
    route.at("*").get(serve);
    Ok(())
}

pub struct ServeDir {
    prefix: String,
    dir: PathBuf,
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
                Ok(file) => {
                    log::info!("Serving file: {:?}", file);
                    File::open(file).await.unwrap() // TODO: remove unwrap
                }
            };

            // TODO: fix related bug where async-h1 crashes on large files
            Response::new(StatusCode::Ok).body(BufReader::new(file))
        })
    }
}
