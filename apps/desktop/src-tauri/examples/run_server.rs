//! Run the cocommand API server standalone for testing.
//!
//! Usage: cargo run --example run_server

use cocommand::server;

#[tokio::main]
async fn main() {
    // Load .env file
    let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));

    println!("Starting cocommand API server...");

    match server::start().await {
        Ok(addr) => {
            println!("Server listening on http://{}", addr);
            println!("\nAvailable endpoints:");
            println!("  GET  /health            - Health check");
            println!("  GET  /apps              - List all applications");
            println!("  GET  /tools             - List tools for open apps");
            println!("  GET  /window/snapshot   - Get workspace snapshot");
            println!("  POST /window/open       - Open an application");
            println!("  POST /window/close      - Close an application");
            println!("  POST /window/focus      - Focus an application");
            println!("  POST /command           - Process a user command");
            println!("  POST /execute           - Execute a tool directly");
            println!("\nPress Ctrl+C to stop");

            // Keep the server running
            tokio::signal::ctrl_c().await.unwrap();
            println!("\nShutting down...");
        }
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    }
}
