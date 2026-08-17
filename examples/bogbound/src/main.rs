mod fold_store;
mod game;
mod model;
mod semantic;
mod server;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("BOGBOUND_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let host = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "localhost".into());
    let join_url = format!("http://{host}:{port}");
    let db_path = std::env::temp_dir().join(format!("bogbound-{}.db", std::process::id()));
    let engine = game::spawn(join_url, db_path);
    server::serve(port, engine).await;
}
