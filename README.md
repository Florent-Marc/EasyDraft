# EasyDraft 🦀

**EasyDraft** est un prototype en **Rust + egui/eframe** pour dessiner des plans de maison en 2D et construire des schémas électriques de travail.

## Joindre et dissoudre les sommets de murs (sans raccordement automatique)

En mode **Selection**, un clic sur un point de mur le selectionne (poignee
ambre). **Maj + clic** ajoute un autre sommet. **Maj + rectangle** capture
plusieurs extremites. Les murs superposes ne sont **plus lies implicitement** :
deux points qui ont les memes coordonnees peuvent etre deplaces independamment.

- **Glisser** les sommets choisis deplace leur groupe en conservant leur
  decalage initial ; cela ne les joint pas automatiquement.
- **Joindre** dans la barre superieure ou dans **Parametres > Objet** :
  avec **au moins deux sommets selectionnes**, tous prennent exactement la
  position du premier point choisi. Une **liaison persistante** les fait
  ensuite bouger ensemble si tu glisses un de leurs sommets.
- **Joindre avec un seul sommet selectionne** : le sommet est rapproche de
  l'axe du mur le plus proche dans un rayon de **20 cm** (fermeture manuelle
  de petits manques en T pour la detection des pieces). S'il s'agit de
  l'extremite d'un autre mur, la liaison devient persistante.
- **Dissoudre** enlève une liaison explicite de la selection **sans bouger**
  les murs : un nouveau glissement d'un sommet le deplace seul. Sur deux
  sommets encore superposes, un clic suivant peut alterner le sommet actif.
- **Ctrl+Z / Ctrl+Y** annule ou retablit en une operation un Joindre, un
  Dissoudre ou un deplacement.
- Les groupes joints sont conserves dans le **JSON** (champ optionnel
  `wall_joints`), sans casser la lecture des anciens fichiers. La suppression
  d'un mur ajuste automatiquement les references des groupes survivants.

Les jonctions impossibles (mur trop court, ouverture qui sortirait de son
mur) sont refusees en bloc, sans deformation partielle.


## Navigation souris et placement de murs parfaitement droits

- **Clic droit maintenu + glisser** sur le plan : deplacement de la vue, quel que soit l'outil actif (y compris **Selection** et durant le trace d'un mur). Aucun element n'est modifie.
- **Molette de la souris** : zoom autour du pointeur. L'outil separe « Deplacer le plan » a ete supprime.
- **Clic gauche** : creation ou selection normale de l'element. En mode Selection, glisser depuis un espace vide encadre plusieurs objets ; glisser un objet selectionne le deplace.
- **Tracer un mur** : un clic pour le depart, puis deplacer la souris pour l'apercu, et un second clic pour valider. Une ligne **verte avec guide horizontal/vertical** apparait quand l'axe est proche de 0 ou 90 degres (tolerance approximative de 5 degres) ; le mur est alors pose exactement sur cet axe.
- **Mur oblique** : la previsualisation est **bleue** et affiche l'angle courant. L'orientation arbitraire reste autorisee.
- **Maintenir Maj (Shift)** pendant la previsualisation et le deuxieme clic pour forcer exactement l'axe horizontal ou vertical le plus proche.
- L'indication **longueur en metres + angle / axe** se met a jour sous la souris, meme si les cotes permanentes sont masquees.

## Nouvelle interface CAO - espace de travail en trois colonnes

La fenetre est organisee comme un outil de dessin technique :

- **En haut** : bascule **Schema unifilaire / Plan de situation / Dossier RGIE**, menus Fichier, Edition et Export, puis commandes rapides de l'espace courant.
- **A gauche** : bibliotheque de composants **recherchable** et organisee par categories (dessin, construction, electricite). Choisir un outil l'active reellement sur le canvas.
- **Au centre** : feuille de travail ; le plan est dessinable a la souris, le schema unifilaire affiche une vue d'ensemble des departements par differentiel.
- **A droite** : **Parametres** contextuels (selection, outil courant, calques, circuits, dossier et export). Les formulaires electriques lourds sont separes du dessin.

### Plan de situation avec fond de plan image

1. Ouvrir **Plan de situation**, puis **Parametres > Plan > Importer une image...**.
2. Choisir un **PNG, JPG, JPEG ou BMP** (la lecture directe d'un PDF comme fond n'est pas encore prise en charge).
3. Ajuster **Largeur reelle de l'image**, **Opacite**, **Position X/Y** et la rotation par pas de 90 degres.
4. Superposer les murs, les prises, les interrupteurs et les points lumineux. Les calques, la grille, les cotes et les filtres de circuits peuvent etre affiches ou masques.
5. Exporter le plan en PDF : **le fond raster est inclus sous les objets vectoriels** et est precompose selon son opacite.

La sauvegarde JSON garde le **chemin vers le fichier image externe**, pas les pixels : si le JSON est ouvert sur un autre ordinateur ou si l'image est deplacee, il faut remettre l'image a son ancien emplacement ou la reimporter. Le PDF exporte **contient** l'image, lui, et reste portable.

### Schema unifilaire

L'onglet **Schéma** affiche les circuits côte à côte dans la zone centrale, avec leurs protections, commandes, luminaires et prises. Les circuits rattachés à un même différentiel partagent un rail. Les circuits sans différentiel attribué sont affichés à part, sans liaison à l’amont. Les onglets **Topologie du tableau** et **Circuits et liaisons** donnent accès aux détails et aux actions de modification.

> La disposition graphique n'est pas une simulation physique de conducteurs, ni une preuve de conformite RGIE. Les protections et leurs valeurs doivent etre declarees d'apres l'installation reelle.

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

## Selection a la souris et fichiers Windows

Le mode **Mur** est un outil de pose unique : un clic pour le point de depart,
un second pour la fin. Le mur est immediatement selectionne et EasyDraft revient
automatiquement en mode **Selection** pour le deplacer ou modifier ses proprietes.

En mode **Selection** :

- **Clic** sur un mur, une porte/fenetre/passage, un escalier ou un symbole
  electrique pour le selectionner.
- **Glisser** directement un symbole electrique ou un escalier pour le deplacer ;
  **glisser** un mur entier ou ses poignees pour deplacer ou redimensionner le mur.
  Une ouverture glissee suit uniquement son mur.
- **Dessiner un rectangle** depuis une zone vide pour selectionner plusieurs
  objets : de **gauche a droite**, seuls les elements entierement contenus
  sont pris ; de **droite a gauche**, les murs qui traversent le rectangle
  sont egalement captures.
- **Glisser un objet du groupe selectionne** pour deplacer tous les elements
  du groupe. Les portes, fenetres et passages restent attaches a leurs murs.
  **Supprimer la selection** supprime le groupe. **Ctrl+Z** annule un
  deplacement complet en une seule fois.

Les **fenetres natives d'ouverture et de sauvegarde** sont disponibles via
le menu **Fichier**. La premiere utilisation de **Enregistrer** ouvre une
fenetre pour choisir le fichier JSON ; les sauvegardes suivantes reprennent
cet emplacement. **Enregistrer sous...** demande toujours un nouveau fichier.
**Ouvrir...** permet de choisir le JSON a charger. Dans **Exporter**,
**Plan en PDF...** et **Schema unifilaire en PDF...** demandent le dossier
de sortie et le nom du PDF. Les annulations des fenetres ne remplacent pas
le projet courant.

Les boites de dialogue sont fournies par `rfd` (natives sous Windows).
Les tests automatises verifient egalement l'ecriture/relecture du JSON
et la production d'un PDF. Un programme de bureau necessite toujours un
essai manuel des fenetres de fichier sur le poste cible.

## Interface de CAO et passages libres

EasyDraft affiche désormais une **barre d'outils par métier** en haut de la fenêtre
(**Architecture**, **Électricité**) et des onglets latéraux qui évitent les longs
formulaires mélangés :

- **Dessin** : création de murs, portes, fenêtres, passages libres et escaliers, avec réglages des nouveaux éléments.
- **Inspecteur** : propriétés de l'élément cliqué, édition des cotes et de l'épaisseur.
- **Calques** : visibilité, grille, aimantation, zoom et recentrage.
- **Élec.** : tableau électrique, circuits et symboles RGIE.
- **RGIE** : identification du dossier.
- **Fichiers** : chemins JSON et PDF. La barre supérieure contient également des menus **Fichier**, **Édition**, **Exporter**.

### Créer un passage entre deux pièces sans porte

**Important :** un passage libre compte comme une **limite invisible de pièce**,
même si le passage ne contient pas de porte. Le dessin conserve le vide, tandis
que la détection automatique et les surfaces utilisent une frontière fermée.

Deux géométries sont maintenant reconnues en cliquant avec l'outil **Passage libre** :
- **Un trou entre deux morceaux d'une même cloison** (tronçons alignés ou presque),
  y compris lorsque le clic est proche de l'extrémité d'un mur ;
- **Un trou en T entre la fin d'une cloison et un mur perpendiculaire** (par
  exemple un passage entre la cloison et le mur extérieur).

Dans les deux cas, une fermeture virtuelle de l'axe est créée, recouverte par
une ouverture de la même largeur. Elle ne se dessine pas comme un mur plein
et reste enregistrée dans le fichier JSON. La barre d'état annonce le nombre
de pièces reconnues après insertion. Si d'autres murs ne sont pas raccordés,
le plan peut rester ouvert : il faut alors joindre aussi ces extrémités
avec l'outil **Sélection**.



1. **Méthode normale :** tracez une cloison continue, même là où le passage doit être.
   Sélectionnez **Architecture → Passage libre**, réglez sa largeur, puis cliquez
   sur la cloison. Une ouverture apparaît **sans battant**. Le calcul automatique
   reconnaît toujours les deux pièces de part et d'autre de la cloison.
2. **Pour réparer un ancien plan :** si la cloison est déjà dessinée en deux
   segments avec un vide, sélectionnez **Passage libre** puis cliquez **dans ce vide**.
   EasyDraft recherche deux extrémités de murs alignées et face à face, séparées
   de **35 cm à 4 m**. Il relie leurs axes **uniquement pour le calcul des pièces**,
   tandis que le passage reste visuellement ouvert. Une cloison vraiment absente
   ne doit pas être reliée : il suffit de ne pas utiliser cet outil sur ce vide.
3. Pour modifier la largeur ou l'emplacement, cliquez sur **Sélection** puis sur
   l'ouverture : ses paramètres apparaissent dans **Inspecteur**. Les passages
   et leurs dimensions sont enregistrés dans le JSON et affichés dans le PDF du plan.

**Important :** les passages libres modélisent ici une séparation logique entre
deux pièces, même si l'air circule librement. Les surfaces sont estimées en 2D
par les faces intérieures des cloisons ; ce ne sont pas des volumes ni une
vérification réglementaire des surfaces habitables.

## Symboles graphiques du RGIE (Livre 1, tableau 2.23)

Les pictogrammes du plan de position et du schéma unifilaire utilisent désormais une **bibliothèque graphique commune** : `src/rgie_symbols.rs`, également utilisée par les deux exports PDF vectoriels.

Les formes de base disponibles sont :

- **Interrupteurs** : symbole général, bipolaire, va-et-vient unipolaire, bouton-poussoir.
- **Prises** : symbole général, prise avec contact pour conducteur de protection, prise avec protection enfant, et combinaison des deux.
- **Éclairage** : point lumineux (croix) et point d'attente en applique (croix avec trait de mur).
- **Protections** : symbole de disjoncteur ou de coupe-circuit à fusible **selon le type renseigné**. Un type absent ou inconnu est représenté comme **non spécifié**, pas comme un disjoncteur inventé.

Pour configurer un appareil déjà placé : outil **Sélectionner un élément** → cliquer sur la prise, l'interrupteur ou la lampe → **Inspecteur** → **Symbole (RGIE tableau 2.23)**. La variante choisie est sauvegardée dans le JSON, et les anciens plans sans variantes conservent un symbole général. La liste est limitée aux variantes compatibles avec le type d'appareil.

L'interface contient une **Légende RGIE** permettant de consulter les formes actuellement prises en charge. Les variantes **ne sont pas détectées** automatiquement : l'utilisateur doit relever le type réel du matériel et sélectionner le pictogramme adéquat. En particulier, **aucune terre ni protection enfant n'est supposée sur une prise**. Un pictogramme va-et-vient ou bouton-poussoir ne constitue **pas** une simulation des conducteurs et commandes spécifiques.

**Références officielles** :
- [RGIE Livre 1 2026, tableau 2.23, chapitre 2.13](https://economie.fgov.be/sites/default/files/Files/Publications/files/RGIE-Annexe-Livre-1-Installations-a-basse-tenstion-et-a-tres-basse-tension-2026.pdf)
- [Liste illustrée des symboles SPF Économie](https://economie.fgov.be/sites/default/files/Files/Energy/Symboles-fr.pdf)

L'ensemble du tableau 2.23 n'est **pas encore implémenté** (sont notamment absents : socles multiples, prises étanches, commandes complexes, boîtes, sources, appareils fixes, schémas complets de tableau). Les symboles et l'ordre de câblage doivent être revus avant toute présentation comme dossier réglementaire.

## Tableau électrique et liaisons amont (prototype)

Le tableau n'est plus une simple étiquette. Le modèle de `Plan` contient désormais des **protections différentielles identifiées** et une affectation explicite de chaque circuit à son DDR amont.

Dans le panneau gauche :

1. Dérouler **Tableau électrique - protections différentielles**, puis cliquer sur **Ajouter un différentiel (D1, D2...)**. Pour chaque DDR, renseigner son **repère, type, courant nominal, sensibilité en mA et nombre de pôles** d'après l'installation réelle. Toutes les valeurs restent inconnues par défaut.
2. Dérouler les propriétés d'un circuit A/B/C : dans **Différentiel EN AMONT du circuit**, sélectionner D1 ou D2. Un DDR peut alimenter plusieurs circuits.
3. Ouvrir **Schéma unifilaire** : le premier dessin est un **synoptique du tableau**, montrant `Alimentation → DDR → protection divisionnaire → circuit`. Les circuits sans DDR apparaissent séparément comme **non attribués**, sans fausse liaison à l'alimentation.
4. Décrire la **canalisation d'entrée du tableau** sous **Identification du dossier RGIE** et conserver les caractéristiques de chaque départ.
5. Cliquer sur **Exporter PDF unifilaire** : la première page présente le tableau, les suivantes montrent les circuits et leurs points d'utilisation. Les pages sont vectorielles.
6. Cliquer sur **Dossier RGIE (vérification)** pour afficher les champs et liaisons manquants. Le tableau refuse la suppression d'un DDR tant que des circuits lui sont associés.

**Exemple immédiatement utilisable** : cliquer sur **Charger l'exemple tableau D1 / D2**. La démonstration contient les circuits **A et B en aval de D1**, et le circuit **C en aval de D2**. Elle remplace le plan courant, ce qui est annulable. Les calibres, sensibilités et types sont **volontairement vides** : ils ne représentent aucun schéma d'installation à reproduire.

**Compatibilité** : les anciens fichiers JSON restent chargeables. L'ancien champ `upstream_rdc` est conservé comme note historique, **mais n'établit jamais une liaison de tableau**. Pour tracer la topologie, il faut affecter explicitement un DDR.

**Limite fondamentale :** le synoptique décrit les relations saisies, **pas les conducteurs réels**. Il ne modélise pas encore les barres de répartition, les protections en cascade, les chemins de câble, la terre, les bornes ou les calculs d'admissibilité et de sélectivité. Les boîtes des DDR sont des représentations de travail identifiées, pas une couverture complète de la symbolique réglementaire.

## Préparation du dossier RGIE belge (prototype)

Les documents de référence sont le **Livre 1 du RGIE belge (version 2026)** et les indications du [SPF Economie pour le contrôle d'installations domestiques](https://economie.fgov.be/fr/themes/energie/sources-et-vecteurs-denergie/electricite/securite-et-controle-des/controle-des-installations).

EasyDraft ne prétend **pas** certifier une installation. Le bouton **« Dossier RGIE (vérification) »** ouvre une **liste d'informations manquantes ou à contrôler** ; il ne remplace pas le contrôle d'un organisme agréé.

- **Identification du dossier** dans le panneau latéral : adresse, responsable de l'exécution, qualité, n° TVA si applicable, tension nominale, nature du courant, alimentation, **câble d'entrée**, tableau, date et révision. Les valeurs doivent être saisies, pas inventées.
- **Circuits et protections** : un DDR défini et associé aux circuits, puis des caractéristiques propres par départ (protection, canalisation, conducteurs, section, pose).
- **Prises de courant** : quand une prise est placée, elle est affectée au circuit actif. Une prise ancienne sans affectation peut être sélectionnée pour choisir son circuit dans **Inspecteur**.
- **Concordance des repères** : chaque lampe et prise rattachée possède un repère A1, A2, B1… repris sur le plan de position 2D, dans l'unifilaire et les deux PDF. Les interrupteurs rappellent les repères des lampes qu'ils commandent.
- **Détection des incohérences** : prises sans circuit, lampes sans commande, interrupteurs sans charge, liaisons invalides ou dupliquées, caractéristiques de départ et du dossier non renseignées.
- **PDF** : page de tableau, DDR attribués, adresse, tension, nature du courant, responsable, date et révision sur l'unifilaire ; repères des prises et points lumineux sur le plan et l'unifilaire.

L'ordre des numéros est généré **selon l'ordre des appareils dans le projet**, et **non déterminé en traçant le véritable circuit électrique** : l'ordre physique depuis chaque protection doit être confirmé avant l'établissement d'un document réglementaire.

**Limites non résolues :** symboles de travail à rapprocher du tableau 2.23 du RGIE, différences entre prises simples et multiples, boîtes de connexion et de dérivation, machines/appareils fixes, sources (PV, onduleurs, batteries), tableaux multiples, distribution sur jeux de barres, vraies liaisons des conducteurs, terre, protections et dimensionnement en fonction des conditions réelles, mise à jour et signatures officielles. Aucun de ces points n'est déclaré conforme par le logiciel.

La description exhaustive des écarts et des priorités est dans [`docs/RGIE-CHECKLIST.md`](docs/RGIE-CHECKLIST.md).

## Schéma unifilaire : plusieurs circuits d'éclairage

Le prototype permet maintenant de gérer **plusieurs circuits distincts** (16 au maximum pour le moment, prises et éclairage associés). Dans le panneau gauche :

1. Cliquez sur **« Charger l'exemple unifilaire A + B »** pour charger un projet de démonstration avec deux circuits, trois lampes, deux interrupteurs, une prise et une petite maison. **L'exemple remplace le plan courant** (Annuler permet de revenir en arrière).
2. Ouvrez **« Schéma unifilaire »** : les circuits apparaissent côte à côte avec leurs points d'utilisation. A commande deux lampes (**A1**, **A2**) et B comprend une lampe (**B1**) et une prise (**B2**).
3. Cliquez sur **« Ajouter un circuit (prototype) »** pour créer B, C, D, etc. Choisissez **« Circuit actif pour lampes et prises »** avant de relier un interrupteur et une lampe sur le plan 2D. Le bouton de démonstration ajoute également une paire sur ce circuit actif.
4. Déployez **« Circuit X - protection / conducteurs »** pour renseigner, séparément pour chaque circuit, sa protection, son différentiel en amont, sa canalisation, la section et le mode de pose. **Les champs ne sont pas remplis automatiquement avec des valeurs supposées**.
5. Dans le schéma, utilisez **« Déplacer vers un autre circuit »** à côté d'un interrupteur pour transférer **toutes ses lampes** au circuit choisi sans refaire les liaisons. Une lampe ne peut appartenir qu'à une seule liaison, et un interrupteur ne peut commander des lampes de plusieurs circuits simultanément dans cette version.
6. **« Exporter PDF unifilaire »** génère un **synoptique de tableau en première page**, puis des feuilles A4 paysage **séparées par circuit**, paginées par groupes de six liaisons. La même référence (A1, B1...) est reprise dans le plan 2D et le PDF.

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
- **Zoom à la molette**, centré sous le pointeur ; **clic droit maintenu + glisser** pour déplacer la vue (sans changer d'outil).
- **Coordonnées X et Y** du curseur affichées en haut à droite du plan.
- **Calques d'affichage Architecture / Électricité** : possibilité de masquer temporairement l'un ou l'autre, sans supprimer les données.
- Les nouvelles valeurs de longueur sont vérifiées : le logiciel refuse de raccourcir un mur si cela ferait sortir une porte ou fenêtre de son emprise.

La case **Électricité** contrôle aussi l’affichage des symboles, repères et liaisons électriques sur toutes les feuilles du PDF du plan de situation.

## Niveaux et parties du bâtiment

Dans la barre du haut, ouvrez **Gérer les feuilles** pour ajouter un étage, un grenier ou une cave. Pour dessiner une annexe ou une autre partie du bâtiment, cliquez sur **Ajouter une partie**, puis saisissez son nom dans la fenêtre. Le sélecteur **Feuille** permet de reprendre le dessin d'un autre niveau. Les anciens plans s'ouvrent sur la feuille **Rez-de-chaussée**, que vous pouvez renommer.

L'enregistrement JSON conserve toutes les feuilles et les réglages du projet, dont les calques visibles, la grille, les cotes, l'aimantation, le cadrage du dessin et les réglages des outils. Dans **Exporter → Plan de situation en PDF**, EasyDraft crée un seul PDF A4 paysage avec une page titrée par feuille, dans l'ordre de la liste. Le schéma unifilaire et le dossier RGIE concernent la feuille active.

### Préparer et réutiliser les feuilles

- **Exporter → Aperçu du plan de situation** permet de parcourir les pages avant de générer le PDF.
- Dans **Paramètres → Plan**, choisissez une **Feuille en fond léger** pour aligner les murs et les escaliers avec ceux d’un autre niveau.
- Sélectionnez un appareil électrique et réglez sa **Rotation** dans **Objet**.
- Sélectionnez une pièce, puis utilisez **Déplacer le nom** ou **Déplacer la surface**. Cliquez sur le plan pour choisir la nouvelle position.
- Sélectionnez des murs ou escaliers, copiez avec **Ctrl+C**, changez de feuille et collez avec **Ctrl+V**. Les ouvertures suivent leurs murs et les éléments gardent leurs coordonnées.

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
