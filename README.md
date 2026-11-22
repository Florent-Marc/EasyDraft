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

## Pièces automatiques (prototype)

EasyDraft détecte maintenant les **espaces fermés** formés par les murs. Il prend en compte les intersections et les jonctions en T pour séparer plusieurs pièces partageant une cloison.

- **Surface intérieure estimée en m²** calculée à partir des faces intérieures des murs (épaisseurs comprises dans le calcul).
- **Dimensions intérieures** pour une pièce rectangulaire, ou encombrement selon X/Y pour une forme non rectangulaire.
- **Nom personnalisable** : sélectionner l'outil « Sélectionner un élément », puis cliquer **à l'intérieur d'une pièce**. Le panneau « Propriétés de la sélection » permet de saisir son nom (Salon, Cuisine, Chambre...).
- **Étiquettes sur le plan** : nom, surface et dimensions estimées ; elles apparaissent aussi dans l'export PDF du plan.
- Case **« Pièces, surfaces et dimensions »** dans les calques visibles pour masquer/afficher les étiquettes sans effacer les données.
- Les noms de pièces sont sauvegardés dans le **JSON** et restent associés à un point intérieur, même si l'ordre des murs change.

**Exemple prêt à ouvrir** : `examples/maison-deux-pieces.json` contient un Salon et une Cuisine séparés par un mur commun avec une porte, ainsi qu'un interrupteur relié à une lampe. Pour le charger, mettez ce chemin dans le champ **Fichier JSON**, puis cliquez sur **Ouvrir**. Le logiciel doit détecter automatiquement **deux pièces**.

**Limites** : il s'agit d'un calcul 2D géométrique, pas d'un métré réglementaire. Les surfaces de pièces concaves ou très complexes restent des estimations, et les cotes hors rectangles sont des dimensions d'encombrement. Les noms sont ancrés à une position dans la pièce : si un changement de murs déplace complètement la pièce, il peut être nécessaire de la renommer. Les ouvertures ne cassent pas la fermeture géométrique des murs.

## Édition précise du plan 2D

L'éditeur permet désormais de **modifier et supprimer les éléments sans recréer le plan** :

1. Cliquez sur **Sélectionner un élément**, puis choisissez un mur, un escalier, une prise, un interrupteur ou un luminaire sur le dessin.
2. Dans **Propriétés de la sélection**, modifiez les valeurs exactes en mètres. Pour un mur : coordonnées X/Y du départ et de l'arrivée, **longueur**, épaisseur et alignement horizontal/vertical.
3. Utilisez **Supprimer la sélection** ou la touche **Suppr** pour effacer l'objet.
4. Le logiciel retire automatiquement les portes/fenêtres appartenant à un mur supprimé, et les connexions électriques liées à un appareil supprimé. Il réajuste également les références internes des autres objets.
5. Utilisez **Annuler / Rétablir** ou **Ctrl+Z / Ctrl+Y** ; **Échap** annule une sélection ou un tracé en cours.

Pour dessiner avec plus de précision :

- **Aimantation aux extrémités de murs** : priorité au raccord d'un mur existant dans un rayon de 13 pixels, sinon l'aimantation à la grille prend le relais. Option désactivable.
- **Zoom à la molette**, centré sous le pointeur ; outil « Déplacer la vue » pour se déplacer sur le plan.
- **Coordonnées X et Y** du curseur affichées en haut à droite du plan.
- **Calques d'affichage Architecture / Électricité** : possibilité de masquer temporairement l'un ou l'autre, sans supprimer les données.
- Les nouvelles valeurs de longueur sont vérifiées : le logiciel refuse de raccourcir un mur si cela ferait sortir une porte ou fenêtre de son emprise.

Les calques sont des **options d'affichage du logiciel** ; l'export PDF actuel du plan contient les données du projet indépendamment de leur visibilité à l'écran. Les plans JSON existants restent compatibles.

## Dessin de la maison

- Murs à deux clics, épaisseur de 5 à 60 cm, cotations axe-à-axe
- Portes et fenêtres insérées dans les murs, avec largeur réglable
- Escaliers droits, marches et flèche du sens de la montée
- Grille, aimantation, zoom, déplacement, annulation
- Prises, interrupteurs et lampes en superposition du plan 2D

## Limitations importantes

**Ce logiciel n'est pas un générateur de documents certifiés conformes au RGIE.** La vue unifilaire est un **prototype fonctionnel**, pas un schéma d'exécution ni une étude de dimensionnement. Elle ne prend pas encore en charge les circuits de prises, les boîtes, la distribution multi-tableaux, le schéma complet des différentiels, les symboles normalisés exhaustifs, ni les calculs de protection. Les valeurs saisies par l'utilisateur ne font l'objet d'aucune validation réglementaire. Pour une installation réelle, faire vérifier le dossier par un professionnel compétent.

L'unifilaire décrit actuellement **un seul circuit d'éclairage logique**, même si plusieurs interrupteurs et lampes peuvent être reliés. Les escaliers sont uniquement droits. La suppression individuelle des appareils et l'édition avancée des circuits restent à venir.

## Vérification

Des tests Rust supplémentaires couvrent l'édition et la suppression de murs avec leurs ouvertures, la conservation des index d'éléments électriques, l'aimantation aux extrémités, l'annulation/rétablissement, la détection des pièces fermées, les pièces séparées par une cloison, les calculs de surfaces rectangulaires et la persistance des noms. Ils sont destinés à être exécutés par `cargo test` ou le workflow CI ; les commits ne garantissent pas à eux seuls que la compilation a réussi.

## Licence

Non définie à ce stade.
