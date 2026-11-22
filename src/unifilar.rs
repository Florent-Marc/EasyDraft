//! Schéma unifilaire d'éclairage : représentation de travail.
//! Les liaisons (plan 2D / unifilaire) ont une seule source de vérité : Plan.
//! L'affichage ne constitue ni une validation RGIE ni un plan de câblage.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};

use crate::model::{LightingCircuit, Plan, Point};

const INK: Color32 = Color32::from_rgb(43, 56, 72);
const BLUE: Color32 = Color32::from_rgb(37, 107, 180);
const MUTED: Color32 = Color32::from_rgb(101, 111, 127);
const WARNING: Color32 = Color32::from_rgb(177, 93, 21);

/// Actions remontées au niveau de l'application, pour enregistrer une annulation.
pub enum Action {
    RemoveConnection(usize),
}

fn stroke(color: Color32, width: f32) -> Stroke {
    Stroke::new(width, color)
}

fn segment(p: &egui::Painter, a: Pos2, b: Pos2, color: Color32, width: f32) {
    p.line_segment([a, b], stroke(color, width));
}

fn caption(p: &egui::Painter, at: Pos2, label: &str, size: f32, color: Color32) {
    p.text(at, Align2::CENTER_CENTER, label, FontId::proportional(size), color);
}

fn box_outline(p: &egui::Painter, top_left: Pos2, size: Vec2) {
    let a = top_left;
    let b = top_left + Vec2::new(size.x, 0.0);
    let c = top_left + size;
    let d = top_left + Vec2::new(0.0, size.y);
    for (start, end) in [(a,b), (b,c), (c,d), (d,a)] {
        segment(p, start, end, INK, 1.3);
    }
}

fn switch_symbol(p: &egui::Painter, at: Pos2) {
    let left = at + Vec2::new(-18.0, 0.0);
    let right = at + Vec2::new(18.0, 0.0);
    segment(p, left + Vec2::new(-4.0, 0.0), left, INK, 2.0);
    segment(p, right, right + Vec2::new(4.0, 0.0), INK, 2.0);
    // Contact normalement ouvert, representation intentionnellement simplifiee.
    p.circle_filled(left, 2.8, INK);
    p.circle_filled(right, 2.8, INK);
    segment(p, left, at + Vec2::new(12.0, -14.0), BLUE, 2.0);
}

fn light_symbol(p: &egui::Painter, at: Pos2) {
    p.circle_stroke(at, 17.0, stroke(INK, 1.7));
    segment(p, at + Vec2::new(-10.0, -10.0), at + Vec2::new(10.0, 10.0), INK, 1.7);
    segment(p, at + Vec2::new(-10.0, 10.0), at + Vec2::new(10.0, -10.0), INK, 1.7);
}

fn breaker_symbol(p: &egui::Painter, left: Pos2) {
    // Repère et contact de disjoncteur simplifié, sans allégation de symbolique normative.
    let middle = left + Vec2::new(27.0, 0.0);
    let right = left + Vec2::new(64.0, 0.0);
    segment(p, left, middle, INK, 2.0);
    segment(p, middle, middle + Vec2::new(18.0, -18.0), INK, 2.0);
    let block = middle + Vec2::new(18.0, -18.0);
    let top_left = block + Vec2::new(-4.0, -4.0);
    box_outline(p, top_left, Vec2::new(9.0, 9.0));
    segment(p, right + Vec2::new(-11.0, 0.0), right, INK, 2.0);
}

fn label_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() { "a renseigner" } else { value.trim() }
}

fn draw_header(
    painter: &egui::Painter,
    origin: Pos2,
    circuit: &LightingCircuit,
) {
    let y = origin.y + 100.0;
    caption(painter, origin + Vec2::new(110.0, 17.0), "TABLEAU / DEPART", 16.0, INK);
    box_outline(painter, origin + Vec2::new(25.0, 57.0), Vec2::new(160.0, 65.0));
    caption(painter, origin + Vec2::new(105.0, 78.0), "Tableau electrique", 14.0, INK);
    caption(painter, origin + Vec2::new(105.0, 100.0), "amont non dessine", 11.0, MUTED);

    let breaker_start = origin + Vec2::new(205.0, 100.0);
    segment(
        painter,
        Pos2::new(origin.x + 185.0, y),
        breaker_start,
        INK,
        2.0,
    );
    breaker_symbol(painter, breaker_start);
    caption(
        painter,
        origin + Vec2::new(242.0, 62.0),
        &format!("Protection : {}", if circuit.breaker_amp == 0 {
            "a renseigner".to_owned()
        } else {
            circuit.protection_label()
        }),
        13.0, INK,
    );
    caption(
        painter,
        origin + Vec2::new(245.0, 142.0),
        &format!("DDR amont : {}", label_or_unknown(&circuit.upstream_rdc)),
        12.0, MUTED,
    );

    let cable_x = origin.x + 382.0;
    segment(
        painter,
        origin + Vec2::new(269.0, 100.0),
        Pos2::new(cable_x, y),
        INK, 2.0,
    );
    caption(
        painter,
        origin + Vec2::new(490.0, 63.0),
        &format!("Circuit {}", label_or_unknown(&circuit.letter)),
        16.0, BLUE,
    );
    caption(
        painter,
        origin + Vec2::new(540.0, 87.0),
        &circuit.cable_label(),
        13.0, INK,
    );
    caption(
        painter,
        origin + Vec2::new(540.0, 110.0),
        &format!("Pose : {}", label_or_unknown(&circuit.installation_method)),
        12.0, MUTED,
    );

}

fn draw_groups(
    painter: &egui::Painter,
    plan: &Plan,
    origin: Pos2,
    groups: &[(usize, Vec<usize>)],
) {
    let spine_x = origin.x + 382.0;
    let switch_x = origin.x + 565.0;
    let right_x = origin.x + 820.0;
    let branch_x = origin.x + 675.0;

    let mut top_y = origin.y + 190.0;
    let first_y = top_y + 32.0;
    let mut last_group_y = first_y;
    for (group_index, (switch_index, lamps)) in groups.iter().enumerate() {
        let base_y = top_y + 32.0;
        let switch_position = Pos2::new(switch_x, base_y);
        segment(
            painter,
            Pos2::new(spine_x, base_y),
            Pos2::new(switch_x - 22.0, base_y),
            INK, 2.0,
        );
        painter.circle_filled(Pos2::new(spine_x, base_y), 4.0, BLUE);
        switch_symbol(painter, switch_position);

        let switch_number = plan.symbol_number(*switch_index).unwrap_or(switch_index + 1);
        caption(
            painter,
            Pos2::new(switch_x, base_y + 29.0),
            &format!("I{switch_number}"),
            14.0, INK,
        );
        caption(
            painter,
            Pos2::new((spine_x + switch_x) / 2.0, base_y - 23.0),
            &format!("Commande {}", group_index + 1),
            12.0, BLUE,
        );

        segment(
            painter,
            Pos2::new(switch_x + 22.0, base_y),
            Pos2::new(branch_x, base_y),
            INK, 2.0,
        );
        let last_lamp_y = base_y + ((lamps.len() - 1) as f32 * 64.0);
        if lamps.len() > 1 {
            segment(
                painter,
                Pos2::new(branch_x, base_y),
                Pos2::new(branch_x, last_lamp_y),
                INK, 2.0,
            );
        }
        for (i, light_index) in lamps.iter().enumerate() {
            let y = base_y + i as f32 * 64.0;
            if i > 0 {
                painter.circle_filled(Pos2::new(branch_x, y), 3.5, INK);
            }
            segment(
                painter,
                Pos2::new(branch_x, y),
                Pos2::new(right_x - 20.0, y),
                INK, 2.0,
            );
            light_symbol(painter, Pos2::new(right_x, y));
            let lamp_number = plan.symbol_number(*light_index).unwrap_or(light_index + 1);
            caption(
                painter,
                Pos2::new(right_x, y + 29.0),
                &format!("L{lamp_number}"),
                14.0, INK,
            );
            if let Some(link_index) = plan.lighting_connections.iter().position(|link| {
                link.light_index == *light_index && plan.lighting_connection_is_valid(link)
            }) {
                caption(
                    painter,
                    Pos2::new(right_x + 65.0, y),
                    &format!(
                        "{}{}",
                        label_or_unknown(&plan.lighting_circuit.letter),
                        link_index + 1
                    ),
                    13.0, BLUE,
                );
            }
        }
        let row_height = 99.0 + (lamps.len() - 1) as f32 * 64.0;
        last_group_y = base_y;
        top_y += row_height.max(112.0);
    }

    // Le tronc vertical part du tableau et alimente les differents groupes.
    segment(
        painter,
        origin + Vec2::new(382.0, 100.0),
        Pos2::new(spine_x, last_group_y.max(first_y)),
        INK, 2.4,
    );
}

/// Affiche un schéma derivé du modèle, et renvoie une demande de suppression éventuelle.
pub fn show(ui: &mut egui::Ui, plan: &Plan) -> Option<Action> {
    ui.heading("Schéma unifilaire - éclairage");
    ui.label("Vue de travail du circuit de commande : protections, canalisation et points commandés.");
    ui.colored_label(
        WARNING,
        "Prototype : ce document n'est ni un schema de controle RGIE ni un plan de cablage.",
    );

    let missing = plan.lighting_circuit.missing_fields();
    if !missing.is_empty() {
        ui.colored_label(
            WARNING,
            format!(
                "Informations manquantes ({}): {}.",
                missing.len(),
                missing.join(", ")
            ),
        );
        ui.small("Renseigne ces caracteristiques dans le panneau de gauche. Aucun calibre ni type de cable n'est suppose.");
    } else {
        ui.label("Champs descriptifs remplis (coherence et conformite non verifiees).");
    }

    let groups = plan.lighting_groups();
    let invalid = plan.lighting_connections.iter()
        .filter(|link| !plan.lighting_connection_is_valid(link))
        .count();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "{} commande(s) | {} liaison(s) | {} lampe(s) sans commande | {} interrupteur(s) sans charge",
            groups.len(),
            plan.lighting_connections.len().saturating_sub(invalid),
            plan.unlinked_lights(),
            plan.unlinked_switches(),
        ));
    });
    if invalid > 0 {
        ui.colored_label(
            Color32::RED,
            format!("{invalid} liaison(s) invalide(s) dans le fichier JSON : ignoree(s)."),
        );
    }

    let mut action = None;
    ui.separator();

    if groups.is_empty() {
        ui.label("Aucune commande reliee. Place un interrupteur et un point lumineux sur le plan 2D, puis utilise « Lier I -> L ».");
        ui.label("Le bouton « Ajouter demo I -> L » permet d'obtenir un premier exemple immediatement.");
        return None;
    }

    // La vue est de largeur fixe, pour rester lisible en petit et en grand ecran.
    // Le défilement vertical/horizontal est gere par egui.
    let rows_height: f32 = groups.iter().map(|(_, lamps)| {
        (99.0 + lamps.len().saturating_sub(1) as f32 * 64.0).max(112.0)
    }).sum();
    let canvas_h = 270.0 + rows_height;

    egui::ScrollArea::both().max_height(ui.available_height().max(220.0) * 0.75)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                Vec2::new(970.0, canvas_h),
                egui::Sense::hover(),
            );
            painter.rect_filled(response.rect, 0.0, Color32::from_rgb(250, 251, 253));
            let origin = response.rect.left_top() + Vec2::new(14.0, 8.0);
            draw_header(&painter, origin, &plan.lighting_circuit);
            draw_groups(&painter, plan, origin, &groups);
            painter.text(
                origin + Vec2::new(13.0, canvas_h - 35.0),
                Align2::LEFT_CENTER,
                "Lignes = relations fonctionnelles ; ne pas utiliser pour un raccordement reel",
                FontId::proportional(12.0),
                MUTED,
            );
        });

    ui.separator();
    ui.label("Connexions du projet (supprimer ici actualise aussi le plan 2D) :");
    for (index, link) in plan.lighting_connections.iter().enumerate() {
        if !plan.lighting_connection_is_valid(link) {
            continue;
        }
        let switch_number = plan.symbol_number(link.switch_index).unwrap_or(link.switch_index + 1);
        let lamp_number = plan.symbol_number(link.light_index).unwrap_or(link.light_index + 1);
        ui.horizontal(|ui| {
            ui.label(format!("Interrupteur I{switch_number}  ->  luminaire L{lamp_number}"));
            if ui.small_button("Supprimer la liaison").clicked() {
                action = Some(Action::RemoveConnection(index));
            }
        });
    }
    ui.small("Symboles provisoires : les references et conventions RGIE restent a implementer.");
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagram_uses_same_grouping_as_plan() {
        let mut p = Plan::default();
        p.add_demo_lighting();
        assert_eq!(p.lighting_groups().len(), 1);
        assert_eq!(p.lighting_groups()[0].1.len(), 1);
    }
}
