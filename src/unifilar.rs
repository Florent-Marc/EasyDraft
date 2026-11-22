//! Apercu unifilaire derive directement des liaisons du plan 2D.
//! Ce dessin fonctionnel n'est ni une etude de cablage ni un schema RGIE certifie.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};
use crate::model::{LightingConnection, Plan};

const INK: Color32 = Color32::from_rgb(43, 56, 72);
const ACCENT: Color32 = Color32::from_rgb(31, 109, 181);
const MUTED: Color32 = Color32::from_rgb(103, 114, 127);

fn line(painter: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    painter.line_segment([a, b], stroke);
}

fn text(painter: &egui::Painter, at: Pos2, label: &str, size: f32, color: Color32) {
    painter.text(at, Align2::CENTER_CENTER, label, FontId::proportional(size), color);
}

fn draw_link(
    painter: &egui::Painter,
    link: &LightingConnection,
    plan: &Plan,
    origin: Pos2,
    index: usize,
) {
    let y = origin.y + 104.0 + index as f32 * 116.0;
    let stroke = Stroke::new(2.0, INK);
    let trunk_x = origin.x + 204.0;
    let switch_x = origin.x + 330.0;
    let lamp_x = origin.x + 548.0;

    // Troncon du depart A, derive vers chaque branche.
    line(painter, Pos2::new(trunk_x, y), Pos2::new(switch_x - 20.0, y), stroke);
    painter.circle_filled(Pos2::new(trunk_x, y), 3.0, INK);

    let switch_center = Pos2::new(switch_x, y);
    painter.circle_stroke(switch_center, 18.0, stroke);
    line(
        painter,
        Pos2::new(switch_x - 8.0, y + 7.0),
        Pos2::new(switch_x + 8.0, y - 7.0),
        Stroke::new(1.7, ACCENT),
    );

    line(
        painter,
        Pos2::new(switch_x + 20.0, y),
        Pos2::new(lamp_x - 19.0, y),
        stroke,
    );
    let lamp_center = Pos2::new(lamp_x, y);
    painter.circle_stroke(lamp_center, 18.0, stroke);
    line(
        painter,
        lamp_center + Vec2::new(-10.0, -10.0),
        lamp_center + Vec2::new(10.0, 10.0),
        stroke,
    );
    line(
        painter,
        lamp_center + Vec2::new(-10.0, 10.0),
        lamp_center + Vec2::new(10.0, -10.0),
        stroke,
    );

    let switch_number = plan.symbol_number(link.switch_index).unwrap_or(link.switch_index + 1);
    let lamp_number = plan.symbol_number(link.light_index).unwrap_or(link.light_index + 1);

    text(painter, Pos2::new(switch_x, y + 37.0), &format!("I{switch_number}"), 14.0, INK);
    text(painter, Pos2::new(lamp_x, y + 37.0), &format!("L{lamp_number}"), 14.0, INK);
    text(painter, Pos2::new((switch_x + lamp_x) / 2.0, y - 17.0), "Commande", 12.0, MUTED);
    text(painter, Pos2::new(origin.x + 130.0, y), &format!("A.{}", index + 1), 13.0, MUTED);
}

/// Dessine la vue unifilaire dans un espace defilable horizontalement et verticalement.
pub fn show(ui: &mut egui::Ui, plan: &Plan) {
    ui.heading("Schema unifilaire - prototype");
    ui.label("Vue fonctionnelle du circuit d'eclairage A, generee depuis les liaisons du plan 2D.");
    ui.small("Illustration simplifiee : ni section de cables, ni disjoncteurs, ni controles RGIE.");
    ui.separator();

    let valid_links: Vec<&LightingConnection> = plan.lighting_connections
        .iter()
        .filter(|link| plan.lighting_connection_is_valid(link))
        .collect();

    if valid_links.is_empty() {
        ui.add_space(12.0);
        ui.label("Aucune liaison interrupteur-lampe. Sur le plan 2D, ajoute un interrupteur, une lampe, puis utilise 'Lier I -> L'.");
        ui.label("Tu peux aussi cliquer sur 'Ajouter demo I -> L' pour tester immediatement.");
        return;
    }

    let invalid = plan.lighting_connections.len() - valid_links.len();
    if invalid > 0 {
        ui.colored_label(Color32::RED, format!(
            "{invalid} liaison(s) du fichier sont invalides et ont ete ignorees."
        ));
    }
    let unlinked = plan.unlinked_lights();
    ui.label(format!(
        "{} liaison(s) enregistree(s) | {} lampe(s) sans interrupteur",
        valid_links.len(),
        unlinked
    ));
    if unlinked > 0 {
        ui.colored_label(Color32::from_rgb(183, 98, 17), "Attention : certaines lampes ne sont pas reliees.");
    }

    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        // Largeur constante pour eviter un espace infini dans ScrollArea::both.
        let diagram_width = 760.0;
        let diagram_height = (180.0 + valid_links.len() as f32 * 116.0).max(350.0);
        let (response, painter) = ui.allocate_painter(
            Vec2::new(diagram_width, diagram_height),
            egui::Sense::hover(),
        );
        let origin = response.rect.left_top() + Vec2::new(18.0, 12.0);
        let stroke = Stroke::new(2.0, INK);

        // Depart logique, ne represente pas un dispositif de protection.
        let board_left = origin.x + 8.0;
        let board_right = origin.x + 150.0;
        let board_top = origin.y + 48.0;
        let board_bottom = origin.y + 88.0;
        let a = Pos2::new(board_left, board_top);
        let b = Pos2::new(board_right, board_top);
        let c = Pos2::new(board_right, board_bottom);
        let d = Pos2::new(board_left, board_bottom);
        for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
            line(&painter, p, q, stroke);
        }
        text(
            &painter,
            Pos2::new((board_left + board_right) / 2.0, (board_top + board_bottom) / 2.0),
            "Tableau (depart A)",
            13.0,
            INK,
        );

        let trunk_x = origin.x + 204.0;
        line(
            &painter,
            Pos2::new(board_right, origin.y + 68.0),
            Pos2::new(trunk_x, origin.y + 68.0),
            stroke,
        );
        let first_y = origin.y + 104.0;
        let last_y = origin.y + 104.0 + (valid_links.len() - 1) as f32 * 116.0;
        line(
            &painter,
            Pos2::new(trunk_x, origin.y + 68.0),
            Pos2::new(trunk_x, last_y.max(first_y)),
            stroke,
        );

        for (index, link) in valid_links.into_iter().enumerate() {
            draw_link(&painter, link, plan, origin, index);
        }
        painter.text(
            origin + Vec2::new(8.0, diagram_height - 25.0),
            Align2::LEFT_CENTER,
            "Liaisons fonctionnelles seulement - aucune validation reglementaire",
            FontId::proportional(12.0),
            MUTED,
        );
    });
}
