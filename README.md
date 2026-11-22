# EasyDraft 🦀

EasyDraft est un prototype de logiciel de bureau en **Rust** pour dessiner des plans de maison en 2D et placer des éléments électriques.

## Fonctionnalités v0.1

- Tracer des murs avec deux clics et afficher leur longueur
- Grille aimantée (pas de 0,5 m), zoom et déplacement de la vue
- Placer des prises, interrupteurs et points lumineux
- Annuler la dernière modification
- Enregistrer et ouvrir les plans au format JSON

## Démarrage

Installez [Rust](https://rustup.rs/), puis lancez :

```bash
cargo run
```

Tests :

```bash
cargo test
```

## Utilisation

Sélectionnez « Mur » puis cliquez deux fois sur la zone de dessin. Les outils « Prise », « Interrupteur » et « Point lumineux » placent des symboles à chaque clic. Utilisez « Déplacer la vue » pour naviguer. Le champ « Fichier » définit le chemin du projet JSON.

## Évolutions envisagées

Portes, fenêtres, cotations, calques, circuits, schéma unifilaire, symboles normalisés et export PDF/SVG.

> **Attention :** les symboles de cette première version sont indicatifs. Le logiciel ne produit pas encore de plans certifiés conformes au RGIE.

## Technologies

Rust 2024, eframe/egui 0.37, serde et serde_json.

## Licence

Non définie à ce stade.
