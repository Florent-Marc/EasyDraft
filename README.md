# EasyDraft 🦀

EasyDraft est un éditeur de plans de maison **2D en Rust**, avec placement d'équipements électriques.

## Fonctions de la version prototype

- **Murs** : tracé à deux clics, épaisseur réglable (5 à 60 cm), sélection et modification de l'épaisseur
- **Cotations** : longueur axe-à-axe de chaque mur, lignes de cote affichables ou masquables
- **Portes** : insertion dans un mur, largeur réglable, choix de la charnière et du sens d'ouverture, arc de débattement
- **Fenêtres** : insertion dans un mur, largeur réglable, vitrage représenté en plan
- **Escaliers droits** : deux clics définissent le début et la fin de la montée ; largeur et nombre de marches réglables ; flèche de montée visible
- **Électricité** : placement de prises, interrupteurs et points lumineux (symboles indicatifs)
- Grille 50 cm, aimantation 5 à 50 cm, zoom, déplacement et annulation
- Sauvegarde et ouverture au format JSON, compatible avec les plans créés avant l'ajout des ouvertures et escaliers
- **Export PDF** : dessin vectoriel sur une feuille **A4 paysage**, avec murs, ouvertures, escaliers, cotes (si activées) et symboles électriques

## Démarrer

Installez [Rust](https://rustup.rs/) puis, dans le dossier du dépôt :

```bash
cargo run
```

Pour exécuter les tests :

```bash
cargo test
```

## Dessiner un plan

1. Choisissez l'outil **Mur**, réglez l'épaisseur et cliquez deux fois pour définir ses extrémités.
2. Choisissez **Porte** ou **Fenêtre**, réglez la largeur et cliquez sur un mur. Une ouverture est découpée automatiquement dans le tracé du mur. Des ouvertures qui se chevauchent sont refusées.
3. Choisissez **Escalier**, réglez largeur et nombre de marches, puis cliquez deux fois pour délimiter l'escalier. La flèche montre le sens de montée.
4. Ajoutez éventuellement les symboles électriques.
5. Cliquez sur **Enregistrer** pour sauvegarder le plan au format JSON.

## Exporter un PDF

Saisissez le chemin de sortie dans **Fichier PDF (A4 paysage)**, puis cliquez sur **Exporter en PDF**. Par défaut le document est enregistré dans `plan-easydraft.pdf` (dans le répertoire de lancement du logiciel).

**Important :** le plan est automatiquement ajusté pour tenir sur une page. L'export **n'est pas encore à une échelle architecturale stricte** (1:50, 1:100...), même si les longueurs des murs sont en mètres. Une fonction d'impression à échelle exacte sera nécessaire pour des plans d'exécution.

## Limites actuelles

- Escaliers droits uniquement ; pas encore de tournants ou colimaçons
- Portes et fenêtres liées à un mur, non repositionnables après insertion via la souris (annulation disponible)
- Pas encore de vérification des normes ou de schémas électriques unifilaires certifiés conformes au RGIE
- Les symboles électriques sont provisoires et non normalisés

## Technologies

Rust 2024, eframe/egui 0.33, serde, serde_json. Le PDF est généré directement en format vectoriel PDF 1.4, sans dépendance externe supplémentaire.

## Licence

Non définie à ce stade.
