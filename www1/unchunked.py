#!/usr/bin/env python3
import sys

def generate_response():
    # Contenu de la réponse
    response = "Hello World"  # Contenu de la réponse
    print("Content-Type: text/plain")
    print(f"Content-Length: {len(response)}")
    print()  # Ligne vide après les en-têtes

    # Envoyer les données
    print(response)

# Appeler la fonction pour générer la réponse
generate_response()
