# HTTP Server Audit Tests

## 1. Compréhension du fonctionnement du serveur HTTP

**Test:** Demandez à l'étudiant d'expliquer le fonctionnement général d'un serveur HTTP.

**Résultat attendu:** L'étudiant doit pouvoir expliquer le cycle de vie d'une requête HTTP, de la réception à l'envoi de la réponse.

**Explication:** Ce test évalue la compréhension fondamentale du protocole HTTP.

## 2. I/O Multiplexing

**Test:** Vérifiez quelle fonction est utilisée pour l'I/O Multiplexing (par exemple, epoll, select, poll).

**Résultat attendu:** Le code doit utiliser une seule fonction de multiplexing pour gérer toutes les connexions.

**Explication:** L'utilisation d'une seule fonction de multiplexing est cruciale pour l'efficacité du serveur.

## 3. Lecture et écriture par client

**Test:** Examinez le code autour de la fonction de multiplexing.

**Résultat attendu:** Il ne doit y avoir qu'une seule lecture et une seule écriture par client par cycle de la fonction de multiplexing.

**Explication:** Cela garantit une utilisation efficace des ressources système.

## 4. Gestion des erreurs I/O

**Test:** Vérifiez comment sont gérées les valeurs de retour des fonctions I/O.

**Résultat attendu:** Toutes les erreurs doivent être vérifiées et gérées correctement, avec déconnexion du client si nécessaire.

**Explication:** Une bonne gestion des erreurs est essentielle pour la stabilité du serveur.

## 5. Configuration du serveur

**Test:** Modifiez le fichier de configuration pour tester différentes configurations :
- Un seul serveur sur un seul port
- Plusieurs serveurs sur différents ports
- Plusieurs serveurs avec différents noms d'hôte
- Pages d'erreur personnalisées
- Limitation de la taille du corps des requêtes
- Configuration des routes
- Fichier par défaut pour les répertoires
- Méthodes HTTP autorisées par route

**Résultat attendu:** Le serveur doit fonctionner correctement avec chaque configuration.

**Explication:** Ces tests vérifient la flexibilité et la robustesse du système de configuration.

## 6. Méthodes HTTP et Cookies

**Test:** Testez les méthodes GET, POST et DELETE. Vérifiez également le système de cookies et de sessions.

**Résultat attendu:**
- Les requêtes GET, POST et DELETE doivent fonctionner correctement.
- Les requêtes incorrectes doivent être gérées sans crash du serveur.
- Le système de cookies et de sessions doit fonctionner.

**Explication:** Ces tests vérifient la conformité du serveur avec le protocole HTTP et sa capacité à gérer l'état des sessions.

## 7. Upload de fichiers

**Test:** Uploadez des fichiers et vérifiez qu'ils peuvent être récupérés sans corruption.

**Résultat attendu:** Les fichiers doivent être correctement stockés et récupérables.

**Explication:** Ce test vérifie la capacité du serveur à gérer le transfert de fichiers.

## 8. Tests avec un navigateur

**Test:** Utilisez un navigateur pour tester diverses fonctionnalités :
- Connexion au serveur
- Vérification des en-têtes de requête et de réponse
- Test d'URL incorrecte
- Listing de répertoire
- Redirection d'URL
- Fonctionnement du CGI

**Résultat attendu:** Toutes ces fonctionnalités doivent fonctionner correctement dans un vrai navigateur.

**Explication:** Ces tests simulent l'utilisation réelle du serveur par des clients.

## 9. Gestion des ports

**Test:**
- Configurez plusieurs ports et sites web
- Essayez de configurer le même port plusieurs fois

**Résultat attendu:**
- Les configurations multiples doivent fonctionner correctement
- Le serveur doit détecter et signaler les erreurs de configuration de port

**Explication:** Ces tests vérifient la robustesse de la configuration du serveur.

## 10. Test de stress avec Siege

**Test:** Utilisez la commande `siege -b [IP]:[PORT]` sur une page vide.

**Résultat attendu:** La disponibilité doit être d'au moins 99,5%.

**Explication:** Ce test évalue la stabilité et les performances du serveur sous charge.

## 11. Vérification des fuites de mémoire

**Test:** Utilisez des outils comme `top` ou Valgrind pour vérifier l'utilisation de la mémoire pendant le fonctionnement du serveur.

**Résultat attendu:** Aucune fuite de mémoire ne doit être détectée.

**Explication:** Ce test garantit que le serveur gère correctement la mémoire sur le long terme.

## 12. Vérification des connexions persistantes

**Test:** Vérifiez qu'il n'y a pas de connexions bloquées ou persistantes indéfiniment.

**Résultat attendu:** Toutes les connexions doivent être correctement fermées après utilisation.

**Explication:** Ce test assure que le serveur ne gaspille pas de ressources en maintenant des connexions inutiles.