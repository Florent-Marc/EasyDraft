# EasyDraft — état de préparation d'un dossier RGIE

**Référence consultée le 22 novembre 2026** : Livre 1 RGIE belge (édition 2026), sous-sections 3.1.2.1 à 3.1.2.3 et tableau 2.23 ; page officielle SPF Economie consacrée au contrôle des installations domestiques.

Sources officielles :
- https://economie.fgov.be/fr/publications/reglement-general-sur-les
- https://economie.fgov.be/fr/themes/energie/sources-et-vecteurs-denergie/electricite/securite-et-controle-des/controle-des-installations

> Ce document évalue **l'état du logiciel**, pas celui d'une installation. Le logiciel ne délivre jamais d'attestation de conformité et n'approuve ni un choix de câble ni un calibre.

| Élément du dossier | Statut EasyDraft | Limite actuelle |
|---|---|---|
| Adresse du lieu | Champ ajouté | Saisie libre, sans vérification |
| Nom, qualité du responsable de l'exécution | Champs ajoutés | Identification et signature réelles restent à établir |
| Numéro de TVA (si applicable) | Champ ajouté | Non contrôlé |
| Date et révision | Champs ajoutés | Pas de signature électronique |
| Tension et nature du courant | Champs ajoutés | Pas de simulation électrique |
| Repérage des circuits | A, B, C… configurables | Seize départs max. dans le prototype |
| Caractéristiques de la protection contre les surintensités | Champs calibre, pôles, courbe/type | Vérification des conditions réelles et dimensionnement absents |
| DDR amont | Texte descriptif par circuit | Pas de modèle de distribution/protection complet |
| Canalisations | Type, nombre de conducteurs, section, mode de pose | Pas de calcul du courant admissible, de la longueur ni de la chute de tension |
| Points lumineux et commandes | Liens et repères partagés | Les commandes complexes ne sont pas modélisées |
| Socles de prises | Affectation explicite au circuit, repères partagés | Pas de type de prise, socle multiple, commande ou PE modélisés |
| Concordance plan 2D / unifilaire | Même modèle JSON, mêmes références | Ordre des éléments à confronter à l'installation réelle |
| Export PDF | Plan 2D et unifilaire séparés | Symboles illustratifs, cartouche incomplet |
| Boîtes de connexion et dérivation | Non modélisées | À ajouter |
| Appareils fixes / machines | Non modélisés | À ajouter |
| Sources, PV, onduleurs, batteries | Non modélisées | À ajouter |
| Tableaux multiples, protection d'origine, terre | Non modélisés complètement | À ajouter |
| Symboles RGIE (tableau 2.23) | Illustrations provisoires | Légende et symboles normalisés à faire |
| Vérification complète RGIE | **Non prise en charge** | Contrôle par professionnel/organisme agréé selon obligations applicables |

## Contrôles implémentés

L'onglet **Dossier RGIE (vérification)** distingue :

- **Incomplet** : un champ documentaire important est vide, circuit sans récepteur affecté, prise non affectée, luminaire ou interrupteur sans lien, référence de liaison dupliquée ou invalide.
- **À vérifier** : valeurs de protection et de section non dimensionnées par EasyDraft, conventions graphiques, ordre réel des points, appareils et sources non modélisés, contrôle externe.

Même lorsque **aucun champ n'est vide**, il reste des avertissements **À vérifier** : le programme ne devient pas « conforme ».

## Plan des améliorations futures

1. Modèle de tableau, différentiels, barres/distribution, alimentation et protection de chaque départ.
2. Catalogue RGIE de symboles et légende du schéma, propriétés des socles de prises (simples/multiples).
3. Circuits et charges : boîtes, récepteurs fixes, appareils spéciaux, PV, onduleurs, alimentation.
4. Repérage ordonné selon le chemin électrique réel, avec correspondance bidirectionnelle plan/unifilaire.
5. Cartouche d'édition avec signature, contrôle documentaire plus détaillé, PDF à l'échelle et revue par praticien du RGIE 2026.

Les installations existantes et les dérogations réglementaires peuvent relever de règles différentes selon la date et les travaux. **Aucun contrôle logiciel générique ne peut se substituer à cette analyse.**
