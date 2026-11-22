# EasyDraft 🦀

Éditeur de plans de maison en 2D et prototype de schéma unifilaire d'éclairage, développé en **Rust** avec **eframe/egui**.

## Démarrer

Installer [Rust](https://rustup.rs/), cloner le dépôt puis :

```bash
cargo run
```

Pour lancer les tests unitaires :

```bash
cargo test
```

## Premier scénario à tester : un interrupteur et une lampe

1. Ouvrir **Plan 2D**. Sélectionner **Interrupteur** et cliquer sur le plan pour placer `I1`.
2. Sélectionner **Point lumineux** et cliquer ailleurs pour placer `L1` (la lampe est représentée par un X).
3. Sélectionner **Lier I -> L** dans la barre latérale.
4. Cliquer d'abord sur `I1` puis sur `L1`. Une **liaison pointillée** apparaît sur le plan. Elle représente le lien logique, **pas le trajet physique du câble**.
5. Cliquer sur **Schema unifilaire** en haut de l'application. La vue est **calculée à partir des mêmes liaisons enregistrées** et affiche le départ symbolique A, l'interrupteur I1 et la lampe L1.
6. Cliquer sur **Enregistrer** pour sauvegarder le plan en JSON. Réouvrir le fichier avec **Ouvrir** : la liaison est conservée, ainsi que les murs et les autres éléments.

**Raccourci de test** : **Ajouter demo I -> L** crée un interrupteur, une lampe et leur liaison sans supprimer le reste du projet. Un exemple JSON de pièce équipée est aussi disponible dans [`examples/eclairage-simple.json`](examples/eclairage-simple.json) ; entrer son chemin dans **Fichier JSON**, puis cliquer sur **Ouvrir**.

Pour chaque nouvelle lampe, on peut associer un seul interrupteur dans cette première version ; un interrupteur peut commander plusieurs lampes. Les doublons et les types erronés sont rejetés. Le schéma avertit aussi lorsqu'une lampe n'est reliée à rien.

## Dessin architectural

- **Murs** : tracé en deux clics, épaisseur 5 à 60 cm et longueur cotée.
- **Portes** : clic sur un mur, largeur réglable et sens d'ouverture configurable.
- **Fenêtres** : clic sur un mur et largeur réglable.
- **Escaliers droits** : deux clics, largeur, nombre de marches et flèche de montée.
- **Grille**, aimantation réglable, zoom et déplacement de vue.
- **Prises, interrupteurs, lampes** placés directement sur le plan.
- **Annuler**, enregistrer et charger un fichier JSON. Les anciens JSON sans liaisons continuent à se charger.

## Export en PDF

Le bouton **Exporter PDF** produit un plan **A4 paysage** vectoriel dans `plan-easydraft.pdf` (chemin modifiable). Le dessin est ajusté à la page et **n'est pas encore à l'échelle exacte 1:50 ou 1:100**.

L'export PDF actuel porte sur le **plan 2D architectural**, pas encore sur la vue unifilaire.

## Important : prototype non réglementaire

Le **schéma unifilaire est pour l'instant une démonstration fonctionnelle**, pas un dossier électrique belge complet. Il **ne représente pas** le type de protection, le calibre des disjoncteurs, les sections et types de conducteurs, ni toutes les conventions de symboles exigées. Il **ne garantit aucune conformité au RGIE** et ne doit pas être utilisé comme document d'exécution ou de contrôle. Aucune simulation de mise sous tension n'est implémentée.

## Prochaines étapes

- Circuit(s) personnalisables, protection et section de câble
- Références stables pour les appareils et édition/suppression des liaisons
- Symboles normalisés, vrais schémas unifilaires avec plusieurs types de circuits
- Export PDF dédié au schéma unifilaire et contrôles de cohérence
- Tests de compilation CI confirmés sur les plateformes cibles

## Technique

Rust 2024, eframe/egui 0.33, serde et serde_json. Les vues du plan et du schéma se basent sur une seule structure `Plan` dans `src/model.rs` ; la logique de dessin unifilaire se trouve dans `src/unifilar.rs`.

## Licence

Non définie à ce stade.
