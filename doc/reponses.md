# HTTP Server Audit Responses

## Fonctionnalités et Justifications

### Comment fonctionne un serveur HTTP ?

Notre serveur HTTP fonctionne en suivant ces étapes :
1. Il écoute les connexions entrantes sur le(s) port(s) configuré(s).
2. Lorsqu'une requête arrive, elle est lue et parsée (voir `HttpRequest::parse` dans `http.rs`).
3. La requête est routée vers le bon gestionnaire en fonction de la méthode HTTP et du chemin (voir `Router::match_route` dans `router.rs`).
4. Le gestionnaire approprié traite la requête (GET, POST, DELETE, etc.).
5. Une réponse est générée (voir `HttpResponse` dans `http.rs`).
6. La réponse est envoyée au client.

Notre serveur HTTP fonctionne en écoutant les connexions entrantes, en lisant les requêtes HTTP, en les traitant selon la configuration, et en envoyant les réponses appropriées. Il utilise une boucle d'événements non bloquante pour gérer plusieurs connexions simultanément.

### Quelle fonction est utilisée pour le multiplexage I/O et comment fonctionne-t-elle ?

Nous utilisons la fonction `poll` de la bibliothèque `mio` pour le multiplexage I/O. Elle permet de surveiller plusieurs descripteurs de fichiers (sockets) pour des événements de lecture ou d'écriture sans bloquer.

### Le serveur utilise-t-il un seul select (ou équivalent) pour lire les requêtes des clients et écrire les réponses ?

Oui, notre serveur utilise un seul appel à `poll` dans la boucle principale pour gérer toutes les opérations I/O. Cela peut être vérifié dans la méthode `run` de la struct `Server`.

### Pourquoi est-il important d'utiliser un seul select et comment cela a-t-il été réalisé ?

L'utilisation d'un seul `poll` est importante pour l'efficacité et la scalabilité. Cela permet de gérer de nombreuses connexions simultanées sans créer un thread par connexion. Nous l'avons réalisé en utilisant une seule boucle d'événements qui traite tous les événements I/O.

### Y a-t-il seulement une lecture ou écriture par client par select ?

Oui, nous effectuons une seule lecture ou écriture par client à chaque itération de la boucle d'événements. Cela peut être vérifié dans les méthodes `handle_client` et `send_response`.

### Les valeurs de retour des fonctions I/O sont-elles correctement vérifiées ?

Oui, toutes les opérations I/O sont vérifiées pour les erreurs. Par exemple, dans la méthode `read_request`, nous vérifions explicitement les erreurs de lecture.

### Si une erreur est renvoyée par les fonctions précédentes sur un socket, le client est-il supprimé ?

Oui, si une erreur se produit lors de la lecture ou de l'écriture, nous appelons la méthode `remove_client` pour fermer la connexion et nettoyer les ressources associées.

### L'écriture et la lecture sont-elles TOUJOURS effectuées via un select (ou équivalent) ?

Oui, toutes les opérations de lecture et d'écriture sont effectuées uniquement après qu'un événement approprié a été signalé par `poll`.

## Configuration du fichier

### Configuration d'un seul serveur avec un seul port

Testez avec cette configuration :
```
server_name: localhost
host: 127.0.0.1
port: 8080
```
Vérifiez avec : `curl http://localhost:8080`

### Configuration de plusieurs serveurs avec différents ports

Testez avec :
```
server_name: server1
host: 127.0.0.1
port: 8080
server_name: server2
host: 127.0.0.1
port: 8081
```
Vérifiez avec : `curl http://localhost:8080` et `curl http://localhost:8081`

### Configuration de plusieurs serveurs avec différents noms d'hôte

Testez avec :
```
server_name: test1.com
host: 127.0.0.1
port: 8080
server_name: test2.com
host: 127.0.0.1
port: 8080
```
Verifiter avec: `curl --resolve test1.com:8080:127.0.0.1 http://test1.com:8080` et `curl --resolve test2.com:8080:127.0.0.1 http://test2.com:8080`

### Limitation de la taille du corps du client

Ajoutez dans la configuration :
`client_max_body_size: 1000`

Tester avec: `curl -v -X POST -H "Content-Type: plain/text" --data "$(printf 'A%.0s' {1..2000})" http://localhost:8080`

## 5. Configuration du serveur

Notre système de configuration (voir `config.rs`) prend en charge toutes les configurations mentionnées. Nous pouvons configurer plusieurs serveurs, ports, noms d'hôte, pages d'erreur personnalisées, limitations de taille de requête, routes, et méthodes HTTP autorisées.

## 6. Méthodes HTTP et Cookies

Nous supportons les méthodes GET, POST et DELETE (voir les méthodes correspondantes dans `Server`). Les requêtes incorrectes sont gérées sans crash du serveur. Nous avons implémenté un système de cookies et de sessions (voir `session.rs`).

## 7. Upload de fichiers

L'upload de fichiers est géré dans `file_upload.rs`. Les fichiers sont stockés dans un répertoire "uploads" et peuvent etre supprimés avec DELETE.

## 8. Tests avec un navigateur

Toutes les fonctionnalités mentionnées (connexion, en-têtes, gestion des erreurs, listing de répertoire, redirection, CGI) ont été implémentées et testées avec des navigateurs modernes.

## 9. Gestion des ports

Notre système de configuration permet de gérer plusieurs ports et sites web. Nous détectons les conflits de port lors du chargement de la configuration (voir `ServerConfig::from_file` dans `config.rs`).

## 10. Test de stress avec Siege

Nos tests avec Siege ont montré une disponibilité supérieure à 99,5% sur une page vide.

## 11. Vérification des fuites de mémoire

Nous avons utilisé Valgrind pour vérifier l'absence de fuites de mémoire. Aucune fuite n'a été détectée lors de nos tests.

## 12. Vérification des connexions persistantes

Nous utilisons un système de timeout (voir `REQUEST_TIMEOUT` dans `server.rs`) pour nous assurer qu'aucune connexion ne reste ouverte indéfiniment.

Tous ces éléments ont été implémentés et testés. Notre serveur HTTP est robuste, performant et conforme aux spécifications demandées.