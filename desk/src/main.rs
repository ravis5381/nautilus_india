use std::net::IpAddr;
use std::path::PathBuf;

use india_desk::router;

#[tokio::main]
async fn main() {
    load_env_file();
    let host = std::env::var("DESK_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let address = host.parse::<IpAddr>().unwrap_or_else(|_| {
        eprintln!("DESK_HOST {host:?} is not an IP address.");
        std::process::exit(1);
    });
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

    let password = std::env::var("DESK_PASSWORD").unwrap_or_else(|_| {
        eprintln!("Set DESK_PASSWORD in .env or the environment before starting the desk.");
        std::process::exit(1);
    });
    if password.is_empty() {
        eprintln!("DESK_PASSWORD cannot be empty.");
        std::process::exit(1);
    }

    let app = router(&database, password).unwrap_or_else(|error| {
        eprintln!("Could not open the position book at {database}: {error}");
        std::process::exit(1);
    });
    let listener = tokio::net::TcpListener::bind((address, port))
        .await
        .unwrap_or_else(|error| {
            eprintln!("Could not listen on {address}:{port}: {error}");
            std::process::exit(1);
        });
    println!("Desk is at {}", listen_url(address, port));
    if address.is_unspecified() {
        println!("Other machines on the LAN use this computer's own address and port {port}.");
    }
    axum::serve(listener, app).await.unwrap_or_else(|error| {
        eprintln!("Desk stopped: {error}");
        std::process::exit(1);
    });
}

fn listen_url(address: IpAddr, port: u16) -> String {
    match address {
        IpAddr::V6(ip) => format!("http://[{ip}]:{port}"),
        IpAddr::V4(ip) => format!("http://{ip}:{port}"),
    }
}

fn load_env_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(".env");
    if !path.is_file() {
        return;
    }
    dotenvy::from_path(&path).unwrap_or_else(|error| {
        eprintln!("Could not read {}: {error}", path.display());
        std::process::exit(1);
    });
}
