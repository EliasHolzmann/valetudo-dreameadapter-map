use valetudo_dreameadapter_map::{start_http_server, start_telegram_bot};

#[tokio::main]
async fn main() {
    tokio::join!(start_http_server(), start_telegram_bot());
}
