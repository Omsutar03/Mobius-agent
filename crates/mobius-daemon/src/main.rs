mod server;

#[tokio::main]
async fn main() {
    println!("Starting Mobius Daemon...");
    if let Err(e) = server::start_daemon().await {
        eprintln!("❌ Daemon fatal error: {}", e);
        std::process::exit(1);
    }
}
