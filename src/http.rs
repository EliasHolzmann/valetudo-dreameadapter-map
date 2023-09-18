use axum::{routing::get, Router, Json};
use serde::Serialize;
use tower_http::services::ServeDir;

pub async fn start_http_server() {
    // build our application with a single route
    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/pcbs.json", get(get_pcbs));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Pcb {
    username: String,
    location: (f64, f64),
    additional_information: Option<String>
}

async fn get_pcbs() -> Json<Vec<Pcb>> {
    Json(vec![
        Pcb {
            username: String::from("bartim"),
            location: (51.5, -0.09),
            additional_information: None
        },
        Pcb {
            username: String::from("bartim"),
            location: (55.5, -0.09),
            additional_information: Some(String::from("Currently out of country"))
        },
    ])
}