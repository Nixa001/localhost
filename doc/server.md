# Documentation de server.rs

## Vue d'ensemble

`server.rs` est le cœur du serveur HTTP. Il gère la boucle d'événements principale, les connexions entrantes, le traitement des requêtes et l'envoi des réponses.

## Structures principales

### `Server`

La structure `Server` contient l'état global du serveur, y compris :
- `configs`: Les configurations des serveurs virtuels
- `poll`: L'instance de `Poll` pour la gestion des événements I/O
- `listeners`: Les sockets d'écoute pour chaque port configuré
- `clients`: Les connexions client actives
- `routers`: Les routeurs pour chaque serveur virtuel
- `error_handler`: Le gestionnaire d'erreurs personnalisé
- `cgi_handler`: Le gestionnaire de CGI
- `session_manager`: Le gestionnaire de sessions
- `logger`: Le logger pour enregistrer les événements du serveur

## Méthodes principales

### `Server::new`

Initialise une nouvelle instance de `Server` avec les configurations données.

### `Server::run`

La boucle principale du serveur. Utilise `epoll` pour gérer les événements I/O de manière efficace.

### `Server::accept_connection`

Accepte une nouvelle connexion cliente et l'enregistre pour les événements futurs.

### `Server::handle_client`

Gère une connexion cliente, lit la requête et envoie la réponse.

### `Server::handle_request`

Traite une requête HTTP, route vers le bon gestionnaire et génère une réponse.

### `Server::handle_get`, `Server::handle_post`, `Server::handle_delete`

Gèrent respectivement les requêtes GET, POST et DELETE.

### `Server::handle_file_upload`

Gère l'upload de fichiers via des requêtes POST multipart.

## Gestion des erreurs

Le serveur utilise un système de gestion d'erreurs robuste, avec des types d'erreurs personnalisés définis dans `error.rs`. Les erreurs sont propagées de manière appropriée et loggées pour le débogage.

## Performance et scalabilité

Le serveur utilise un modèle d'I/O non bloquant avec `epoll`, ce qui lui permet de gérer efficacement de nombreuses connexions simultanées avec une seule thread. La lecture et l'écriture sont effectuées de manière asynchrone pour éviter le blocage.

## Sécurité

Le serveur implémente plusieurs vérifications de sécurité, notamment :
- Validation des chemins de fichiers pour éviter les attaques par traversée de répertoire
- Limitation de la taille des requêtes pour prévenir les attaques par déni de service
- Gestion sécurisée des sessions et des cookies

## Points d'attention

- La gestion des timeouts pour les connexions inactives
- La gestion correcte des erreurs I/O pour maintenir la stabilité du serveur
