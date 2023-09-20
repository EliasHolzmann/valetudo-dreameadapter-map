use axum::{routing::get, Json, Router};
use tower_http::services::ServeDir;

use crate::Database;

use super::Pcb;

pub async fn start_http_server(_: Database) {
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

async fn get_pcbs() -> Json<Vec<Pcb>> {
    //TODO
    Json(vec![
        Pcb {
            user_id: 3,
            username: String::from("bartim"),
            location: (51.5, -0.09),
            additional_information: None,
        },
        Pcb {
            user_id: 3,
            username: String::from("bartim"),
            location: (55.5, -0.09),
            additional_information: Some(String::from("Currently out of country")),
        },
    ])
}
