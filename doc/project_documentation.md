# Projet de Serveur HTTP en Rust

## Vue d'ensemble

Ce projet implémente un serveur HTTP en Rust, capable de gérer des requêtes HTTP, de servir des fichiers statiques, d'exécuter des scripts CGI, et de gérer des sessions et des cookies. Le serveur est configurable via un fichier de configuration et supporte plusieurs hôtes virtuels.

## Structure du projet

Le projet est composé de plusieurs modules, chacun responsable d'une fonctionnalité spécifique :

- `main.rs`: Point d'entrée du programme, initialise le logger et lance le serveur.
- `config.rs`: Gère le parsing et la validation du fichier de configuration.
- `server.rs`: Contient la logique principale du serveur, y compris la boucle d'événements et la gestion des connexions.
- `http.rs`: Définit les structures pour les requêtes et réponses HTTP.
- `router.rs`: Gère le routage des requêtes vers les bons gestionnaires.
- `static_file.rs`: S'occupe de la lecture et de l'envoi des fichiers statiques.
- `error_handler.rs`: Gère la création des pages d'erreur personnalisées.
- `cgi.rs`: Implémente l'exécution des scripts CGI.
- `session.rs`: Gère les sessions utilisateur.
- `file_upload.rs`: Traite l'upload de fichiers.
- `logger.rs`: Fournit des fonctionnalités de logging.
- `error.rs`: Définit les types d'erreurs personnalisés pour le projet.

## Fonctionnalités principales

1. **Serveur HTTP multi-thread**: Utilise `epoll` (via la crate `mio`) pour une gestion efficace des connexions.
2. **Configuration flexible**: Supporte plusieurs serveurs virtuels, ports, et routes.
3. **Gestion des requêtes HTTP**: Supporte les méthodes GET, POST, et DELETE.
4. **Fichiers statiques**: Sert les fichiers statiques avec gestion du type MIME.
5. **CGI**: Exécute des scripts CGI (comme PHP).
6. **Sessions et cookies**: Gère les sessions utilisateur et les cookies.
7. **Upload de fichiers**: Permet l'upload de fichiers via des requêtes POST multipart.
8. **Logging**: Enregistre les événements du serveur pour le débogage et la surveillance.
9. **Gestion des erreurs**: Pages d'erreur personnalisables et gestion robuste des erreurs.

## Configuration

Le serveur est configuré via un fichier texte qui spécifie les hôtes, ports, routes, et autres paramètres. Voir `config.rs` pour les détails du format de configuration.

## Utilisation

Pour lancer le serveur, exécutez :
Assurez-vous que le fichier de configuration `config.txt` est présent dans le répertoire de travail.
et lancer avec ```cargo run``` et pour tester le serveur vous pouvez utiliser ```curl```