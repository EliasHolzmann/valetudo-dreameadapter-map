mod db;
pub use db::Database;

mod http;
pub use http::start_http_server;

mod bot;
pub use bot::start_telegram_bot;

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Pcb {
    user_id: u64,
    username: String,
    location: (f64, f64),
    additional_information: Option<String>,
}
