//! Main entry point for the crate checker application

use crate_checker::cli::run;
use crate_checker::error::Result;
use std::process;
use tracing::error;

#[tokio::main]
async fn main() {
    // Initialize error handling
    if let Err(e) = run_app().await {
        error!("Application error: {}", e);
        eprintln!("Error: {}", e.user_message());
        process::exit(1);
    }
}

async fn run_app() -> Result<()> {
    // Set up panic handler
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Application panicked: {}", panic_info);
        process::exit(1);
    }));

    // Run the CLI application
    run().await
}
