use valetudo_dreameadapter_map::start_http_server;

#[tokio::main]
async fn main() {
    start_http_server().await;
}