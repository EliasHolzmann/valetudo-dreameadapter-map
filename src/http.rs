use std::{
    convert::Infallible,
    future::{ready, Ready},
    task::Poll,
};

use axum::{
    http::{HeaderMap, HeaderValue, Request, StatusCode},
    routing::get,
    Json, Router,
};
use rust_embed::RustEmbed;
use tower_service::Service;

use crate::Database;

use super::Pcb;

#[derive(RustEmbed, Clone)]
#[folder = "static/"]
struct StaticFiles;

impl<T> Service<Request<T>> for StaticFiles {
    type Response = (StatusCode, HeaderMap<HeaderValue>, Vec<u8>);
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<T>) -> Self::Future {
        let mut path = req.uri().path();
        if path.chars().nth(0) == Some('/') {
            dbg!();
            let mut path_iter = path.chars();
            path_iter.next();
            path = path_iter.as_str();
        }
        if path.is_empty() {
            path = "index.html";
        }
        let file = StaticFiles::get(path);
        let file_list: Vec<_> = <StaticFiles as RustEmbed>::iter().collect();
        dbg!(file_list);
        dbg!(path);
        dbg!(&file.is_some());
        if let Some(file) = file {
            let mut headers = HeaderMap::new();
            let mime = mime_guess::from_path(path).first();
            if let Some(mime) = mime {
                headers.insert("Content-Type", HeaderValue::from_str(&mime.to_string()).unwrap_or_else(|error| {
                    eprintln!("Could not put MIME type {mime} into Content-Type header: {error}/{error:?}");
                    HeaderValue::from_str(&mime.to_string()).unwrap()
                }));
            }
            ready(Ok((StatusCode::OK, headers, Vec::from(file.data))))
        } else {
            ready(Ok((
                StatusCode::NOT_FOUND,
                HeaderMap::new(),
                vec![b'N', b'o', b't', b' ', b'f', b'o', b'u', b'n', b'd'],
            )))
        }
    }
}

pub async fn start_http_server(database: Database) {
    // build our application with a single route

    let app = Router::new()
        .nest_service("/", StaticFiles)
        .route("/pcbs.json", get(move || get_pcbs(database.clone())));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_pcbs(database: Database) -> Result<Json<Vec<Pcb>>, StatusCode> {
    let pcbs = database.get_all_entries().await.map_err(|error| {
        eprintln!("Fetching all pcbs: {error}/{error:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(pcbs))
}
