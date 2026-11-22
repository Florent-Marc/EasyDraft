# EasyDraft 🦀

**EasyDraft** est un prototype en **Rust + egui/eframe** pour dessiner des plans de maison en 2D et construire des schémas électriques de travail.

## Installation

Installez [Rust](https://rustup.rs/), puis :

```bash
cargo run
```

Pour lancer les tests (modèle, anciennes sauvegardes JSON, liaisons et PDF) :

```bash
cargo test
```

Les sources principales sont `src/model.rs`, `src/main.rs`, `src/unifilar.rs` et `src/pdf_export.rs`.

## Démonstration électrique : une lampe et un interrupteur

1. Dans **Plan 2D**, choisissez **Interrupteur** puis cliquez dans la pièce pour placer `I1`.
2. Choisissez **Point lumineux** et cliquez pour placer `L1`.
3. Choisissez **Lier I -> L**, cliquez d'abord sur **I1**, puis sur **L1**. Une ligne bleue pointillée représente **la relation fonctionnelle**, et **non le chemin réel du câble**.
4. Cliquez sur **Schéma unifilaire** en haut. Un schéma du circuit d'éclairage apparaît avec départ, protection à renseigner, canalisation, commande et lampe.
5. Dans le panneau gauche, déroulez **Circuit A - paramètres**. Saisissez les valeurs réelles de protection (calibre, pôles, courbe, différentiel en amont) et de canalisation (type, nombre de conducteurs, section, mode de pose).
6. Vérifiez les avertissements de champs manquants. Cliquez sur **Enregistrer** pour conserver la configuration et les connexions dans le JSON.

**Raccourci** : **Ajouter demo I -> L** ajoute un interrupteur et une lampe déjà reliés. Le fichier `examples/eclairage-simple.json` contient également une petite pièce avec une liaison.

### Améliorations du schéma unifilaire

- Schéma de travail plus lisible : **tableau → disjoncteur → départ de circuit → interrupteur(s) → lampe(s)**.
- Plusieurs lampes peuvent être commandées par un seul interrupteur et sont **regroupées visuellement**.
- Repérage partagé entre le **plan 2D**, le schéma et le **PDF unifilaire** : `A1`, `A2`, etc. Les libellés `I1`, `L1` identifient les symboles d'édition.
- Les informations du circuit sont éditables. **Aucun calibre, câble ni différentiel n'est inventé** : un champ vide reste indiqué comme « à renseigner ».
- Les lampes et interrupteurs sans liaison sont comptés. Les connexions invalides ou dupliquées issues d'un fichier JSON sont ignorées ou signalées.
- Chaque liaison peut être supprimée depuis la vue unifilaire ; cette suppression est visible immédiatement dans le plan 2D et peut être annulée.
- Les caractéristiques, connexions et repères survivent aux sauvegardes JSON ; les anciens projets sans ces champs restent lisibles.

### Deux exports PDF distincts

- **Exporter PDF plan** : le plan de la maison en A4 paysage vectoriel, automatiquement ajusté à la page (sans échelle d'impression 1:50 ou 1:100 garantie).
- **Exporter PDF unifilaire** : un document A4 paysage vectoriel du circuit d'éclairage, **avec pagination automatique par lots de six liaisons**, les caractéristiques renseignées et les éléments manquants. Nom par défaut : `unifilaire-easydraft.pdf`.

Un fichier de test couvrant **sept liaisons et deux pages PDF** est prévu dans les tests Rust.

## Dessin de la maison

- Murs à deux clics, épaisseur de 5 à 60 cm, cotations axe-à-axe
- Portes et fenêtres insérées dans les murs, avec largeur réglable
- Escaliers droits, marches et flèche du sens de la montée
- Grille, aimantation, zoom, déplacement, annulation
- Prises, interrupteurs et lampes en superposition du plan 2D

## Limitations importantes

**Ce logiciel n'est pas un générateur de documents certifiés conformes au RGIE.** La vue unifilaire est un **prototype fonctionnel**, pas un schéma d'exécution ni une étude de dimensionnement. Elle ne prend pas encore en charge les circuits de prises, les boîtes, la distribution multi-tableaux, le schéma complet des différentiels, les symboles normalisés exhaustifs, ni les calculs de protection. Les valeurs saisies par l'utilisateur ne font l'objet d'aucune validation réglementaire. Pour une installation réelle, faire vérifier le dossier par un professionnel compétent.

L'unifilaire décrit actuellement **un seul circuit d'éclairage logique**, même si plusieurs interrupteurs et lampes peuvent être reliés. Les escaliers sont uniquement droits. La suppression individuelle des appareils et l'édition avancée des circuits restent à venir.

## Licence

Non définie à ce stade.
