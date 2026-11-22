# EasyDraft 🦀

EasyDraft est un prototype de logiciel de bureau en **Rust** pour dessiner des plans de maison en 2D et placer des éléments électriques.

## Fonctionnalités v0.2

- Tracer des murs avec deux clics et définir leur épaisseur (5 à 60 cm, 15 cm par défaut)
- Sélectionner un mur pour modifier son épaisseur
- Afficher ou masquer les cotes de longueur des murs (mesurées sur leur axe)
- Grille de 0,5 m, aimantation réglable de 5 à 50 cm, zoom et déplacement de la vue
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

Sélectionnez « Mur », réglez l'épaisseur puis cliquez deux fois sur la zone de dessin. Pour modifier l'épaisseur d'un mur existant, choisissez « Sélectionner un mur », cliquez sur le mur, puis modifiez le champ correspondant dans la barre latérale. La case « Afficher les cotes » contrôle les lignes de cote indiquant la longueur axe-à-axe des murs. Les outils « Prise », « Interrupteur » et « Point lumineux » placent des symboles à chaque clic. Utilisez « Déplacer la vue » pour naviguer. Le champ « Fichier » définit le chemin du projet JSON.

## Évolutions envisagées

Portes, fenêtres, cotations intérieures et extérieures, calques, circuits, schéma unifilaire, symboles normalisés et export PDF/SVG.

> **Attention :** les symboles de cette première version sont indicatifs. Le logiciel ne produit pas encore de plans certifiés conformes au RGIE.

## Technologies

Rust 2024, eframe/egui 0.33, serde et serde_json.

## Licence

Non définie à ce stade.
