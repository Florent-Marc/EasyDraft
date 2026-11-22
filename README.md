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

## Symboles graphiques du RGIE (Livre 1, tableau 2.23)

Les pictogrammes du plan de position et du schéma unifilaire utilisent désormais une **bibliothèque graphique commune** : `src/rgie_symbols.rs`, également utilisée par les deux exports PDF vectoriels.

Les formes de base disponibles sont :

- **Interrupteurs** : symbole général, bipolaire, va-et-vient unipolaire, bouton-poussoir.
- **Prises** : symbole général, prise avec contact pour conducteur de protection, prise avec protection enfant, et combinaison des deux.
- **Éclairage** : point lumineux (croix) et point d'attente en applique (croix avec trait de mur).
- **Protections** : symbole de disjoncteur ou de coupe-circuit à fusible **selon le type renseigné**. Un type absent ou inconnu est représenté comme **non spécifié**, pas comme un disjoncteur inventé.

Pour configurer un appareil déjà placé : outil **Sélectionner un élément** → cliquer sur la prise, l'interrupteur ou la lampe → **Propriétés de la sélection** → **Symbole (RGIE tableau 2.23)**. La variante choisie est sauvegardée dans le JSON, et les anciens plans sans variantes conservent un symbole général. La liste est limitée aux variantes compatibles avec le type d'appareil.

L'interface contient une **Légende RGIE** permettant de consulter les formes actuellement prises en charge. Les variantes **ne sont pas détectées** automatiquement : l'utilisateur doit relever le type réel du matériel et sélectionner le pictogramme adéquat. En particulier, **aucune terre ni protection enfant n'est supposée sur une prise**. Un pictogramme va-et-vient ou bouton-poussoir ne constitue **pas** une simulation des conducteurs et commandes spécifiques.

**Références officielles** :
- [RGIE Livre 1 2026, tableau 2.23, chapitre 2.13](https://economie.fgov.be/sites/default/files/Files/Publications/files/RGIE-Annexe-Livre-1-Installations-a-basse-tenstion-et-a-tres-basse-tension-2026.pdf)
- [Liste illustrée des symboles SPF Économie](https://economie.fgov.be/sites/default/files/Files/Energy/Symboles-fr.pdf)

L'ensemble du tableau 2.23 n'est **pas encore implémenté** (sont notamment absents : socles multiples, prises étanches, commandes complexes, boîtes, sources, appareils fixes, schémas complets de tableau). Les symboles et l'ordre de câblage doivent être revus avant toute présentation comme dossier réglementaire.

## Préparation du dossier RGIE belge (prototype)

Les documents de référence sont le **Livre 1 du RGIE belge (version 2026)** et les indications du [SPF Economie pour le contrôle d'installations domestiques](https://economie.fgov.be/fr/themes/energie/sources-et-vecteurs-denergie/electricite/securite-et-controle-des/controle-des-installations).

EasyDraft ne prétend **pas** certifier une installation. Le bouton **« Dossier RGIE (vérification) »** ouvre une **liste d'informations manquantes ou à contrôler** ; il ne remplace pas le contrôle d'un organisme agréé.

- **Identification du dossier** dans le panneau latéral : adresse, responsable de l'exécution, qualité, n° TVA si applicable, tension nominale, nature du courant, alimentation, tableau, date et révision. Les valeurs doivent être saisies, pas inventées.
- **Circuits et protections** : caractéristiques distinctes par départ (protection, différentiel en amont, canalisation, conducteurs, section, pose).
- **Prises de courant** : quand une prise est placée, elle est affectée au circuit actif. Une prise ancienne sans affectation peut être sélectionnée pour choisir son circuit dans **Propriétés de la sélection**.
- **Concordance des repères** : chaque lampe et prise rattachée possède un repère A1, A2, B1… repris sur le plan de position 2D, dans l'unifilaire et les deux PDF. Les interrupteurs rappellent les repères des lampes qu'ils commandent.
- **Détection des incohérences** : prises sans circuit, lampes sans commande, interrupteurs sans charge, liaisons invalides ou dupliquées, caractéristiques de départ et du dossier non renseignées.
- **PDF** : adresse, tension, nature du courant, responsable, date et révision sur l'unifilaire ; repères des prises et points lumineux sur le plan et l'unifilaire.

L'ordre des numéros est généré **selon l'ordre des appareils dans le projet**, et **non déterminé en traçant le véritable circuit électrique** : l'ordre physique depuis chaque protection doit être confirmé avant l'établissement d'un document réglementaire.

**Limites non résolues :** symboles de travail à rapprocher du tableau 2.23 du RGIE, différences entre prises simples et multiples, boîtes de connexion et de dérivation, machines/appareils fixes, sources (PV, onduleurs, batteries), tableaux multiples, détail de la distribution différentielle, vraie liaison des conducteurs, terre, protections et dimensionnement en fonction des conditions réelles, mise à jour et signatures officielles. Aucun de ces points n'est déclaré conforme par le logiciel.

La description exhaustive des écarts et des priorités est dans [`docs/RGIE-CHECKLIST.md`](docs/RGIE-CHECKLIST.md).

## Schéma unifilaire : plusieurs circuits d'éclairage

Le prototype permet maintenant de gérer **plusieurs circuits distincts** (16 au maximum pour le moment, prises et éclairage associés). Dans le panneau gauche :

1. Cliquez sur **« Charger l'exemple unifilaire A + B »** pour charger un projet de démonstration avec deux circuits, trois lampes, deux interrupteurs, une prise et une petite maison. **L'exemple remplace le plan courant** (Annuler permet de revenir en arrière).
2. Ouvrez **« Schéma unifilaire »** : une **feuille indépendante par circuit** apparaît. A commande deux lampes (**A1**, **A2**) et B comprend une lampe (**B1**) et une prise (**B2**).
3. Cliquez sur **« Ajouter un circuit (prototype) »** pour créer B, C, D, etc. Choisissez **« Circuit actif pour lampes et prises »** avant de relier un interrupteur et une lampe sur le plan 2D. Le bouton de démonstration ajoute également une paire sur ce circuit actif.
4. Déployez **« Circuit X - protection / conducteurs »** pour renseigner, séparément pour chaque circuit, sa protection, son différentiel en amont, sa canalisation, la section et le mode de pose. **Les champs ne sont pas remplis automatiquement avec des valeurs supposées**.
5. Dans le schéma, utilisez **« Déplacer vers un autre circuit »** à côté d'un interrupteur pour transférer **toutes ses lampes** au circuit choisi sans refaire les liaisons. Une lampe ne peut appartenir qu'à une seule liaison, et un interrupteur ne peut commander des lampes de plusieurs circuits simultanément dans cette version.
6. **« Exporter PDF unifilaire »** génère des feuilles A4 paysage **séparées par circuit**, paginées par groupes de six liaisons. La même référence (A1, B1...) est reprise dans le plan 2D et le PDF.

Les circuits associant prises et eclairage peuvent etre soumis a des prescriptions particulieres. Le logiciel signale ce cas pour une verification humaine ; il **ne valide pas les circuits mixtes**.

Les anciens fichiers JSON contenant un seul `lighting_circuit` et des liaisons sans `circuit_index` restent pris en charge : ces liaisons sont affectées au **circuit d'index 0**. Les nouveaux circuits sont stockés dans `additional_lighting_circuits`.

**Attention :** représentation fonctionnelle indicative uniquement. Ce schéma **n'est pas un document réglementaire conforme au RGIE**. Les symboles de base ont été rapprochés du tableau 2.23 mais le catalogue et la représentation des protections restent partiels ; aucun calcul de courant, dimensionnement, section admissible ou contrôle réglementaire n'est exécuté. Des métadonnées saisies ne garantissent pas la conformité d'une installation.

## Démonstration électrique : une lampe et un interrupteur

1. Dans **Plan 2D**, choisissez **Interrupteur** puis cliquez dans la pièce pour placer `I1`.
2. Choisissez **Point lumineux** et cliquez pour placer `L1`.
3. Choisissez **Lier I -> L**, cliquez d'abord sur **I1**, puis sur **L1**. Une ligne bleue pointillée représente **la relation fonctionnelle**, et **non le chemin réel du câble**.
4. Cliquez sur **Schéma unifilaire** en haut. Un schéma du circuit d'éclairage apparaît avec départ, protection à renseigner, canalisation, commande et lampe.
5. Dans le panneau gauche, déroulez **Circuit A - protection / conducteurs**. Saisissez les valeurs réelles de protection (calibre, pôles, courbe, différentiel en amont) et de canalisation (type, nombre de conducteurs, section, mode de pose).
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

## Déplacement direct à la souris

Pour modifier un plan existant, choisissez **« Sélectionner un élément »** dans le panneau d'outils.

- **Déplacer un mur entier** : cliquez sur le corps du mur, maintenez le bouton gauche et glissez. Ses portes et fenêtres **restent attachées** au mur (leurs distances le long du mur ne changent pas).
- **Changer la longueur ou l'orientation d'un mur** : sélectionnez-le puis faites glisser l'une de ses **poignées bleues**, à ses extrémités. L'extrémité opposée reste en place. Le mur s'accroche à la grille ou aux extrémités d'autres murs si l'option d'aimantation est activée.
- **Déplacer une porte ou une fenêtre** : cliquez sur l'ouverture et glissez **dans le sens du mur**. Le logiciel limite la course à l'intérieur du mur et refuse les positions qui chevauchent une autre ouverture. Le point orange indique la sélection.
- **Éditer précisément une ouverture** : sélectionnez-la par un simple clic, puis modifiez sa distance depuis le départ du mur, sa largeur et, pour les portes, son sens d'ouverture dans **Propriétés de la sélection**.
- **Supprimer une ouverture** : sélectionnez-la et utilisez **Supprimer la sélection** ou **Suppr**.
- **Annuler et rétablir** : le glissement complet compte comme **une seule opération** dans l'historique (**Ctrl+Z**, **Ctrl+Y**). Les modifications sont incluses dans le JSON et dans le prochain PDF du plan.

**Limites actuelles :** le déplacement d'un mur ne fait **pas automatiquement suivre les murs voisins** connectés à ses extrémités. Si cela ouvre un contour, la détection des pièces s'actualise en conséquence. Raccordez les extrémités avec les poignées et l'aimantation pour fermer de nouveau la pièce. Déplacer un mur peut également modifier la surface calculée ou demander de renommer une pièce dont le point d'étiquette se trouve désormais hors du contour.

Les mouvements à la souris sont actuellement disponibles pour les **murs, portes et fenêtres**. Les escaliers et les appareils électriques se repositionnent avec leurs champs X/Y dans le panneau de propriétés.

## Pièces automatiques (prototype)

EasyDraft détecte maintenant les **espaces fermés** formés par les murs. Il prend en compte les intersections et les jonctions en T pour séparer plusieurs pièces partageant une cloison.

- **Surface intérieure estimée en m²** calculée à partir des faces intérieures des murs (épaisseurs comprises dans le calcul).
- **Dimensions intérieures** pour une pièce rectangulaire, ou encombrement selon X/Y pour une forme non rectangulaire.
- **Nom personnalisable** : sélectionner l'outil « Sélectionner un élément », puis cliquer **à l'intérieur d'une pièce**. Le panneau « Propriétés de la sélection » permet de saisir son nom (Salon, Cuisine, Chambre...).
- **Étiquettes sur le plan** : nom, surface et dimensions estimées ; elles apparaissent aussi dans l'export PDF du plan.
- Case **« Pièces, surfaces et dimensions »** dans les calques visibles pour masquer/afficher les étiquettes sans effacer les données.
- Les noms de pièces sont sauvegardés dans le **JSON** et restent associés à un point intérieur, même si l'ordre des murs change.

**Exemple prêt à ouvrir** : cliquez sur **« Ouvrir l'exemple de 2 pièces »** dans la barre latérale. Le plan actuel est remplacé (bouton **Annuler** disponible). Le fichier source `examples/maison-deux-pieces.json` contient un Salon et une Cuisine séparés par un mur commun avec une porte, ainsi qu'un interrupteur relié à une lampe. Le logiciel doit détecter automatiquement **deux pièces**, chacune d'environ **10,64 m²** avec les murs de 20 cm. Vous pouvez également saisir le chemin du JSON dans **Fichier JSON** puis cliquer sur **Ouvrir**.

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

L'unifilaire gère plusieurs **circuits d'éclairage logiques** indépendants, mais pas encore les circuits de prises ni les tableaux de distribution complets. Les escaliers restent droits. La définition de câbles réels, de boîtes de dérivation, des phases et des différentiels détaillés, ainsi que la validation des protections, restent à développer.

## Vérification

Des tests Rust supplémentaires couvrent l'édition et la suppression de murs avec leurs ouvertures, la conservation des index d'éléments électriques, l'aimantation aux extrémités, l'annulation/rétablissement, la détection des pièces fermées, les pièces séparées par une cloison, les calculs de surfaces rectangulaires et la persistance des noms. Ils sont destinés à être exécutés par `cargo test` ou le workflow CI ; les commits ne garantissent pas à eux seuls que la compilation a réussi.

## Licence

Non définie à ce stade.
