use valetudo_dreameadapter_map::{start_http_server, start_telegram_bot, Database};

#[tokio::main]
async fn main() {
    let db = Database::new().await;
    tokio::join!(start_http_server(db.clone()), start_telegram_bot(db));
}
