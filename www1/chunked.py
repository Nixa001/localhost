#!/usr/bin/env python3
import sys
import time

def generate_chunks():
    # Envoyer les en-têtes HTTP
    print("Content-Type: text/plain")
    print("Transfer-Encoding: chunked")
    print()  # Ligne vide après les en-têtes

    # Définir les morceaux de données
    chunks = [
        b"4\r\nWiki\r\n",
        b"5\r\npedia\r\n",
        b"0\r\n\r\n"
    ]

    for chunk in chunks:
        sys.stdout.buffer.write(chunk)
        sys.stdout.buffer.flush()
 
# Appeler la fonction pour générer les morceaux
generate_chunks()
