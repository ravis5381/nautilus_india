use std::path::PathBuf;

use india_desk::router;

#[tokio::main]
async fn main() {
    let port = std::env::var("DESK_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787);
    let database = std::env::var("DESK_DB").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("positions.db")
            .display()
            .to_string()
    });

    let app = router(&database).unwrap_or_else(|error| {
        eprintln!("Could not open the position book at {database}: {error}");
        std::process::exit(1);
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap_or_else(|error| {
            eprintln!("Could not listen on 127.0.0.1:{port}: {error}");
            std::process::exit(1);
        });
    println!("Desk is at http://127.0.0.1:{port}");
    axum::serve(listener, app).await.unwrap_or_else(|error| {
        eprintln!("Desk stopped: {error}");
        std::process::exit(1);
    });
}
