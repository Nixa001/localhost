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
    // Initialiser le logger
    let logger = match Logger::new("server.log") {
        Ok(l) => Arc::new(l),
        Err(e) => {
            eprintln!("Échec de l'initialisation du logger : {}", e);
            process::exit(1);
        }
    };

    // Charger la configuration
    let configs = match ServerConfig::from_file("config.toml") {
        Ok(c) => c,
        Err(e) => {
            logger.error(&format!("Échec du chargement de la configuration : {}", e));
            process::exit(1);
        }
    };

    // Créer le répertoire des téléchargements s'il n'existe pas
    if let Err(e) = fs::create_dir_all("uploads") {
        eprintln!(
            "Échec de la création du répertoire des téléchargements : {}",
            e
        );
    }

    // Créer et exécuter le serveur
    match Server::new(configs, logger.clone()) {
        Ok(mut server) => {
            if let Err(e) = server.run() {
                logger.error(&format!("Erreur du serveur : {}", e));
                process::exit(1);
            }
        }
        Err(e) => {
            logger.error(&format!("Échec de la création du serveur : {}", e));
            process::exit(1);
        }
    }
}
