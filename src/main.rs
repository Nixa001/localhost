mod config;
mod http;
mod server;
mod cgi;

use config::ServerConfig;
use server::Server;

fn main() -> std::io::Result<()> {
    let config = ServerConfig::new();
    let mut server = Server::new(&config)?;

    println!("Serveur HTTP démarré sur:");
    for addr in &config.addresses {
        println!("  - http://{}", addr);
    }

    server.run()
}