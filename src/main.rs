use std::env;
use std::net::SocketAddr;
use std::process;
use std::str::FromStr;

use http_server::{config::Config, server::Server};

#[tokio::main]
async fn main() {
    let addr = parse_bind_addr().unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(2);
    });

    let config = Config::default();
    let server = match Server::bind(config, addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind: {}", e);
            process::exit(1);
        }
    };

    eprintln!("Listening on {}", server.local_addr());

    let shutdown = server.shutdown_handle();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        shutdown.shutdown();
    });

    if let Err(e) = server.run().await {
        eprintln!("Server error: {}", e);
        process::exit(1);
    }
}

fn parse_bind_addr() -> Result<SocketAddr, String> {
    let args: Vec<String> = env::args().collect();
    let addr_str = if args.len() > 1 {
        args[1].clone()
    } else if let Ok(env_addr) = env::var("BIND_ADDR") {
        env_addr
    } else {
        "127.0.0.1:8080".to_string()
    };

    SocketAddr::from_str(&addr_str).map_err(|_| format!("Invalid bind address: {}", addr_str))
}
