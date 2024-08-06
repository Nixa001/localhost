mod cgi;
mod config;
mod error;
mod error_handler;
mod http;
mod logger;
mod router;
mod server;
mod session;
mod static_file;

use config::ServerConfig;
use logger::Logger;
use server::Server;
use std::sync::Arc;
use std::{fs, process};

fn main() {
    // Initialize the logger
    let logger = match Logger::new("server.log") {
        Ok(l) => Arc::new(l),
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            process::exit(1);
        }
    };

    // Load the configuration
    let configs = match ServerConfig::from_file("config.toml") {
        Ok(c) => c,
        Err(e) => {
            logger.error(&format!("Failed to load configuration: {}", e));
            process::exit(1);
        }
    };

    // Create upload repository if it doesn't exist
    if let Err(e) = fs::create_dir_all("uploads") {
        eprintln!("Failed to create upload directory: {}", e);
    }

    // Create and run the server
    match Server::new(configs, logger.clone()) {
        Ok(mut server) => {
            if let Err(e) = server.run() {
                logger.error(&format!("Server error: {}", e));
                process::exit(1);
            }
        }
        Err(e) => {
            logger.error(&format!("Failed to create server: {}", e));
            process::exit(1);
        }
    }
}
