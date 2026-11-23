//! Schéma unifilaire d'éclairage : représentation de travail.
//! Les liaisons (plan 2D / unifilaire) ont une seule source de vérité : Plan.
//! L'affichage ne constitue ni une validation RGIE ni un plan de câblage.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};

use crate::model::{ElectricalAppearance, ElectricalKind, LightingCircuit, Plan};
use crate::rgie_symbols;

const INK: Color32 = Color32::from_rgb(43, 56, 72);
const BLUE: Color32 = Color32::from_rgb(37, 107, 180);
const MUTED: Color32 = Color32::from_rgb(101, 111, 127);
const WARNING: Color32 = Color32::from_rgb(177, 93, 21);

/// Actions remontées au niveau de l'application, pour enregistrer une annulation.
pub enum Action {
    RemoveConnection(usize),
    MoveGroup { switch_index: usize, target_circuit: usize },
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

fn switch_symbol(
    painter: &egui::Painter, at: Pos2, appearance: ElectricalAppearance
) {
    rgie_symbols::draw_egui(
        painter, at,
        &rgie_symbols::device_primitives(ElectricalKind::Switch, appearance),
        INK, 0.91
    );
}

fn light_symbol(
    painter: &egui::Painter, at: Pos2, appearance: ElectricalAppearance
) {
    rgie_symbols::draw_egui(
        painter, at,
        &rgie_symbols::device_primitives(ElectricalKind::Light, appearance),
        INK, 1.1
    );
}

fn protection_symbol(
    painter: &egui::Painter, at: Pos2, type_name: &str
) {
    let variant = rgie_symbols::protection_from_text(type_name);
    rgie_symbols::draw_egui(
        painter, at,
        &rgie_symbols::horizontal_protection_primitives(variant),
        INK, 0.8
    );
    if variant == rgie_symbols::ProtectionMark::Unspecified {
        caption(painter, at, "?", 16.0, WARNING);
    }
}

fn label_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() { "a renseigner" } else { value.trim() }
}

fn draw_header(
    painter: &egui::Painter,
    origin: Pos2,
    circuit: &LightingCircuit,
    upstream_description: &str,
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
    protection_symbol(
        painter, breaker_start + Vec2::new(32.0, 0.0),
        &circuit.protection_kind,
    );
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
        &format!("DDR amont : {}", upstream_description),
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
    circuit: &LightingCircuit,
) {
    let _ = circuit;
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
        switch_symbol(painter, switch_position, plan.symbol_appearance(*switch_index));

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
            light_symbol(
                painter, Pos2::new(right_x, y),
                plan.symbol_appearance(*light_index),
            );
            let lamp_number = plan.symbol_number(*light_index).unwrap_or(light_index + 1);
            caption(
                painter,
                Pos2::new(right_x, y + 29.0),
                &format!("L{lamp_number}"),
                14.0, INK,
            );
            if let Some(reference) = plan.lighting_point_ref(*light_index) {
                caption(painter, Pos2::new(right_x + 65.0, y), &reference, 13.0, BLUE);
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

/// Represente les prises attribuees a ce depart sur des branches supplementaires.
/// Symbolique de travail a comparer aux tableaux RGIE avant toute utilisation.
fn draw_outlets(
    painter: &egui::Painter,
    plan: &Plan,
    origin: Pos2,
    circuit_index: usize,
    previous_rows_height: f32,
) {
    let outlets = plan.outlets_on_circuit(circuit_index);
    let spine_x = origin.x + 382.0;
    let x = origin.x + 820.0;
    let from_y = origin.y + 190.0 + previous_rows_height + 32.0;
    for (index, symbol_index) in outlets.iter().enumerate() {
        let y = from_y + index as f32 * 64.0;
        segment(painter, Pos2::new(spine_x, y), Pos2::new(x - 21.0, y), INK, 2.0);
        painter.circle_filled(Pos2::new(spine_x, y), 3.5, BLUE);
        let pos = Pos2::new(x, y);
        rgie_symbols::draw_egui(
            painter, pos,
            &rgie_symbols::device_primitives(
                ElectricalKind::Outlet, plan.symbol_appearance(*symbol_index)
            ),
            INK, 0.94,
        );
        let local_number = plan.symbol_number(*symbol_index).unwrap_or(*symbol_index + 1);
        caption(painter, pos + Vec2::new(0.0, 30.0), &format!("P{local_number}"), 13.0, INK);
        if let Some(reference) = plan.point_ref(*symbol_index) {
            caption(painter, pos + Vec2::new(64.0, 0.0), &reference, 13.0, BLUE);
        }
    }
    if !outlets.is_empty() {
        let last = from_y + outlets.len().saturating_sub(1) as f32 * 64.0;
        segment(
            painter,
            origin + Vec2::new(382.0, 100.0),
            Pos2::new(spine_x, last),
            INK, 2.1,
        );
    }
}

/// Vue d'ensemble du tableau : seules les liaisons DDR effectivement
/// attribuees au modele sont dessinees comme alimentees.
fn show_board_overview(ui: &mut egui::Ui, plan: &Plan) {
    ui.heading("Tableau : alimentation -> DDR -> protection -> circuits");
    ui.small("Topologie des liaisons enregistrees. Les valeurs inconnues restent a renseigner.");
    let mut groups: Vec<(Option<usize>, Vec<usize>)> = (0..plan.differentials.len())
        .map(|index| (Some(index), plan.circuits_for_differential(index)))
        .collect();
    let unassigned = plan.circuits_without_differential();
    if !unassigned.is_empty() {
        groups.push((None, unassigned));
    }
    if groups.is_empty() {
        ui.colored_label(
            WARNING,
            "Aucun DDR ni circuit avec liaison amont. Ajoute un differentiel puis attribue un circuit.",
        );
        return;
    }
    let height = 92.0 + groups.iter().map(|(_, circuits)|
        76.0 + circuits.len().max(1) as f32 * 61.0
    ).sum::<f32>();

    egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(1020.0, height), egui::Sense::hover()
        );
        painter.rect_filled(response.rect, 0.0, Color32::from_rgb(249, 251, 253));
        let origin = response.rect.left_top() + Vec2::new(20.0, 18.0);
        let supply = &plan.installation_info.supply;
        let source = if supply.trim().is_empty() { "Alimentation ?" } else { supply.as_str() };
        box_outline(&painter, origin + Vec2::new(0.0, 16.0), Vec2::new(139.0, 56.0));
        caption(&painter, origin + Vec2::new(70.0, 35.0), "ALIMENTATION", 12.0, INK);
        caption(
            &painter, origin + Vec2::new(70.0, 56.0),
            &source.chars().take(22).collect::<String>(),
            11.0, MUTED,
        );
        segment(
            &painter, origin + Vec2::new(139.0, 44.0),
            origin + Vec2::new(175.0, 44.0), INK, 2.1,
        );

        let mut next_y = origin.y + 100.0;
        let linked_centers: Vec<f32> = groups.iter().scan(next_y, |y, (differential, circuits)| {
            let center = *y + 18.0;
            *y += 76.0 + circuits.len().max(1) as f32 * 61.0;
            Some(if differential.is_some() { Some(center) } else { None })
        }).flatten().collect();

        if let (Some(first), Some(last)) = (linked_centers.first(), linked_centers.last()) {
            segment(
                &painter,
                Pos2::new(origin.x + 175.0, origin.y + 44.0),
                Pos2::new(origin.x + 175.0, *last), INK, 2.0,
            );
            let _ = first;
        }
        for (differential_index, circuits) in &groups {
            let main_y = next_y + 18.0;
            let ddr_top = Pos2::new(origin.x + 217.0, main_y - 24.0);
            if let Some(index) = differential_index {
                let device = &plan.differentials[*index];
                segment(
                    &painter,
                    Pos2::new(origin.x + 175.0, main_y),
                    Pos2::new(origin.x + 217.0, main_y),
                    INK, 2.0,
                );
                box_outline(&painter, ddr_top, Vec2::new(169.0, 54.0));
                caption(
                    &painter, Pos2::new(origin.x + 301.0, main_y - 8.0),
                    &format!("DDR {}", device.reference), 13.0, BLUE,
                );
                let summary = if device.missing_fields().is_empty() {
                    format!("{} | {}A / {}mA | {}P",
                        device.kind, device.rated_current_amp, device.sensitivity_ma, device.poles)
                } else {
                    "Caracteristiques incompletes".to_owned()
                };
                caption(
                    &painter, Pos2::new(origin.x + 301.0, main_y + 13.0),
                    &summary, 10.5, MUTED,
                );
            } else {
                box_outline(&painter, ddr_top, Vec2::new(169.0, 54.0));
                caption(
                    &painter, Pos2::new(origin.x + 301.0, main_y),
                    "DDR NON ATTRIBUE", 13.0, WARNING,
                );
                caption(
                    &painter, Pos2::new(origin.x + 301.0, main_y + 16.0),
                    "branche non reliee", 10.0, WARNING,
                );
            }

            let bus_x = origin.x + 415.0;
            let first_branch_y = main_y + 37.0;
            let last_branch_y = first_branch_y + circuits.len().saturating_sub(1) as f32 * 61.0;
            if differential_index.is_some() && !circuits.is_empty() {
                segment(
                    &painter, Pos2::new(origin.x + 386.0, main_y),
                    Pos2::new(bus_x, main_y), INK, 2.0,
                );
                segment(
                    &painter, Pos2::new(bus_x, main_y),
                    Pos2::new(bus_x, last_branch_y), INK, 2.0,
                );
            }
            if circuits.is_empty() {
                caption(
                    &painter, Pos2::new(origin.x + 624.0, main_y + 28.0),
                    "Aucun circuit affecte", 12.0, MUTED,
                );
            }
            for (position, circuit_index) in circuits.iter().enumerate() {
                let y = first_branch_y + position as f32 * 61.0;
                let Some(circuit) = plan.circuit(*circuit_index) else { continue; };
                let branch = if differential_index.is_some() { INK } else { WARNING };
                // L'alimentation des branches sans DDR n'est PAS dessinee.
                if differential_index.is_some() {
                    segment(
                        &painter, Pos2::new(bus_x, y),
                        Pos2::new(origin.x + 486.0, y), INK, 2.0,
                    );
                }
                rgie_symbols::draw_egui(
                    &painter,
                    Pos2::new(origin.x + 515.0, y),
                    &rgie_symbols::horizontal_protection_primitives(
                        rgie_symbols::protection_from_text(&circuit.protection_kind)
                    ),
                    branch, 0.82,
                );
                segment(
                    &painter,
                    Pos2::new(origin.x + 539.0, y),
                    Pos2::new(origin.x + 613.0, y),
                    branch, 1.7,
                );
                caption(
                    &painter, Pos2::new(origin.x + 655.0, y - 10.0),
                    &format!("Circuit {} - {}", circuit.letter, circuit.protection_label()),
                    11.5, branch,
                );
                caption(
                    &painter, Pos2::new(origin.x + 655.0, y + 9.0),
                    &circuit.cable_label(), 10.0, MUTED,
                );
                let lights = plan.active_lighting_connections_for_circuit(*circuit_index).len();
                let sockets = plan.outlets_on_circuit(*circuit_index).len();
                caption(
                    &painter, Pos2::new(origin.x + 910.0, y),
                    &format!("{lights} lum. / {sockets} prises"), 11.0, BLUE,
                );
            }
            next_y += 76.0 + circuits.len().max(1) as f32 * 61.0;
        }
        painter.text(
            origin + Vec2::new(15.0, height - 18.0),
            Align2::LEFT_CENTER,
            "Vue de travail : pas une preuve de raccordement ou de protection correcte.",
            FontId::proportional(11.0), MUTED,
        );
    });
}

/// Vue "feuille" sur une seule surface, avec une barre de distribution par
/// DDR. Les circuits sans DDR restent visuellement NON raccordes.
/// Ce dessin est un synoptique du modele, pas une topologie du cablage reel.
fn show_schematic_sheet(ui: &mut egui::Ui, plan: &Plan) {
    let mut groups: Vec<(Option<usize>, Vec<usize>)> = (0..plan.differentials.len())
        .map(|i| (Some(i), plan.circuits_for_differential(i)))
        .collect();
    let unassigned = plan.circuits_without_differential();
    if !unassigned.is_empty() { groups.push((None, unassigned)); }
    let columns = groups.iter().map(|(_, circuits)| circuits.len()).max().unwrap_or(1).max(1);
    let width = (330.0 + columns as f32 * 180.0).max(850.0);
    let height = 88.0 + groups.len().max(1) as f32 * 250.0;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong("VUE D'ENSEMBLE");
            ui.separator();
            ui.small(format!("{} depart(s)  |  {} DDR", plan.circuit_count(), plan.differentials.len()));
        });
        ui.small("Une barre par differentiel. Les circuits sans DDR sont affiches a part, sans raccordement invente.");
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(480.0)
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(width, height), egui::Sense::hover()
                );
                painter.rect_filled(response.rect, 0.0, Color32::WHITE);
                let top = response.rect.left_top();
                let heading = if plan.installation_info.address.trim().is_empty() {
                    "INSTALLATION - ADRESSE NON RENSEIGNEE"
                } else {
                    plan.installation_info.address.as_str()
                };
                painter.text(
                    top + Vec2::new(24.0, 22.0),
                    Align2::LEFT_CENTER,
                    heading,
                    FontId::proportional(15.0),
                    INK,
                );
                painter.text(
                    top + Vec2::new(24.0, 45.0),
                    Align2::LEFT_CENTER,
                    "SCHEMA UNIFILAIRE - SYNOPTIQUE FONCTIONNEL",
                    FontId::proportional(11.0),
                    MUTED,
                );
                if groups.is_empty() {
                    painter.text(
                        top + Vec2::new(40.0, 140.0),
                        Align2::LEFT_CENTER,
                        "Ajoute des circuits et leurs protections depuis le panneau de droite.",
                        FontId::proportional(14.0), WARNING,
                    );
                }
                for (group_index, (ddr_index, circuits)) in groups.iter().enumerate() {
                    let gy = top.y + 88.0 + group_index as f32 * 250.0;
                    let bus_y = gy + 169.0;
                    let color = if ddr_index.is_some() { INK } else { WARNING };
                    let source = match ddr_index {
                        Some(index) => {
                            let ddr = &plan.differentials[*index];
                            format!("DDR {}", ddr.reference)
                        }
                        None => "DDR non attribue".to_owned(),
                    };
                    // Bande de groupe et separateur.
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            Pos2::new(top.x + 12.0, gy - 13.0),
                            Vec2::new(width - 24.0, 230.0),
                        ),
                        5.0,
                        Color32::from_rgb(247, 249, 252),
                    );
                    painter.text(
                        Pos2::new(top.x + 24.0, gy + 14.0),
                        Align2::LEFT_CENTER,
                        &source,
                        FontId::proportional(15.0),
                        color,
                    );
                    if let Some(index) = ddr_index {
                        let ddr = &plan.differentials[*index];
                        let details = if ddr.missing_fields().is_empty() {
                            ddr.description()
                        } else { "Caracteristiques du differentiel a renseigner".to_owned() };
                        painter.text(
                            Pos2::new(top.x + 24.0, gy + 35.0),
                            Align2::LEFT_CENTER, details,
                            FontId::proportional(11.0), MUTED,
                        );
                    } else {
                        painter.text(
                            Pos2::new(top.x + 24.0, gy + 35.0),
                            Align2::LEFT_CENTER, "Amont inconnu : aucune connexion dessinee",
                            FontId::proportional(11.0), WARNING,
                        );
                    }
                    let bus_left = top.x + 255.0;
                    let bus_right = bus_left + circuits.len().saturating_sub(1) as f32 * 180.0;
                    if ddr_index.is_some() && !circuits.is_empty() {
                        segment(
                            &painter,
                            Pos2::new(top.x + 122.0, bus_y),
                            Pos2::new(bus_left, bus_y), color, 2.5,
                        );
                    }
                    if !circuits.is_empty() {
                        segment(
                            &painter,
                            Pos2::new(bus_left, bus_y),
                            Pos2::new(bus_right, bus_y), color, 2.5,
                        );
                    }
                    painter.text(
                        Pos2::new(top.x + 24.0, bus_y - 9.0),
                        Align2::LEFT_CENTER,
                        if ddr_index.is_some() { "DISTRIBUTION" } else { "NON RELIE" },
                        FontId::proportional(11.0), color,
                    );
                    for (slot, &circuit_index) in circuits.iter().enumerate() {
                        let Some(circuit) = plan.circuit(circuit_index) else { continue; };
                        let x = bus_left + slot as f32 * 180.0;
                        let branch_top = gy + 71.0;
                        segment(
                            &painter,
                            Pos2::new(x, bus_y),
                            Pos2::new(x, branch_top + 39.0), color, 1.8,
                        );
                        rgie_symbols::draw_egui(
                            &painter,
                            Pos2::new(x, branch_top + 31.0),
                            &rgie_symbols::horizontal_protection_primitives(
                                rgie_symbols::protection_from_text(&circuit.protection_kind)
                            ),
                            color, 0.76,
                        );
                        painter.text(
                            Pos2::new(x, gy + 57.0), Align2::CENTER_CENTER,
                            format!("Circuit {}", circuit.letter),
                            FontId::proportional(14.0), BLUE,
                        );
                        painter.text(
                            Pos2::new(x, branch_top + 65.0), Align2::CENTER_CENTER,
                            circuit.protection_label(),
                            FontId::proportional(11.0), INK,
                        );
                        let lamps = plan.active_lighting_connections_for_circuit(circuit_index).len();
                        let sockets = plan.outlets_on_circuit(circuit_index).len();
                        painter.text(
                            Pos2::new(x, bus_y + 20.0), Align2::CENTER_CENTER,
                            format!("{lamps} lum.  /  {sockets} prise(s)"),
                            FontId::proportional(11.0), MUTED,
                        );
                        painter.text(
                            Pos2::new(x, bus_y + 42.0), Align2::CENTER_CENTER,
                            circuit.cable_label(), FontId::proportional(10.0), MUTED,
                        );
                    }
                }
            });
    });
}

/// Une feuille par circuit : les circuits ne sont pas branches sur le meme tronc.
/// Les appareils et leurs associations proviennent uniquement du Plan partage.
pub fn show(ui: &mut egui::Ui, plan: &Plan) -> Option<Action> {
    ui.heading("Schema unifilaire");
    ui.small("Representation du modele partage avec le plan de situation. Ne remplace pas le releve ni le controle RGIE.");
    show_schematic_sheet(ui, plan);
    egui::CollapsingHeader::new("Topologie detaillee du tableau")
        .default_open(false)
        .show(ui, |ui| show_board_overview(ui, plan));

    let active = plan.active_lighting_connections();
    let ignored = plan.lighting_connections.len().saturating_sub(active.len());
    ui.label(format!(
        "{} circuit(s) | {} liaison(s) | {} lampe(s) sans commande | {} interrupteur(s) sans charge",
        plan.circuit_count(),
        active.len(),
        plan.unlinked_lights(),
        plan.unlinked_switches(),
    ));
    if ignored > 0 {
        ui.colored_label(
            Color32::RED,
            format!("{ignored} liaison(s) ignoree(s) : reference invalide ou doublon."),
        );
    }

    let mut action = None;
    egui::CollapsingHeader::new("Details des circuits et liaisons")
        .default_open(false)
        .show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(620.0).show(ui, |ui| {
        for circuit_index in 0..plan.circuit_count() {
            let Some(circuit) = plan.circuit(circuit_index) else { continue; };
            let links = plan.active_lighting_connections_for_circuit(circuit_index);
            let groups = plan.lighting_groups_for_circuit(circuit_index);
            let outlets = plan.outlets_on_circuit(circuit_index);
            ui.separator();
            ui.heading(format!(
                "Circuit {} - {} commande(s), {} luminaire(s), {} prise(s)",
                circuit.letter,
                groups.len(),
                links.len(),
                outlets.len(),
            ));
            let missing = plan.circuit_missing_fields(circuit_index);
            if !missing.is_empty() {
                ui.colored_label(
                    WARNING,
                    format!(
                        "Informations manquantes ({}): {}",
                        missing.len(),
                        missing.join(", "),
                    ),
                );
            } else {
                ui.label("Informations descriptives saisies (conformite non controlee).");
            }
            if groups.is_empty() && outlets.is_empty() {
                ui.label("Circuit vide. Sur le plan 2D, ajoute une prise ou relie un interrupteur et une lampe.");
                continue;
            }
            let rows_height: f32 = groups.iter().map(|(_, lamps)| {
                (99.0 + lamps.len().saturating_sub(1) as f32 * 64.0).max(112.0)
            }).sum();
            let canvas_h = 270.0 + rows_height + outlets.len() as f32 * 64.0;
            egui::ScrollArea::horizontal()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let (response, painter) = ui.allocate_painter(
                        Vec2::new(970.0, canvas_h), egui::Sense::hover()
                    );
                    painter.rect_filled(
                        response.rect, 0.0, Color32::from_rgb(250, 251, 253)
                    );
                    let origin = response.rect.left_top() + Vec2::new(14.0, 8.0);
                    draw_header(
                        &painter, origin, circuit,
                        &plan.differential_description_for_circuit(circuit_index),
                    );
                    if !groups.is_empty() {
                        draw_groups(&painter, plan, origin, &groups, circuit);
                    }
                    draw_outlets(&painter, plan, origin, circuit_index, rows_height);
                    painter.text(
                        origin + Vec2::new(13.0, canvas_h - 35.0),
                        Align2::LEFT_CENTER,
                        "Branches issues du modele. Les fils, la terre, les calculs et la topologie amont detaillee ne sont pas verifies.",
                        FontId::proportional(11.0),
                        MUTED,
                    );
                });
            ui.label("Commandes :");
            for (switch_index, lamps) in &groups {
                let switch_number = plan.symbol_number(*switch_index)
                    .unwrap_or(switch_index + 1);
                ui.horizontal(|ui| {
                    ui.label(format!("I{switch_number} - {} lampe(s)", lamps.len()));
                    if plan.circuit_count() > 1 {
                        ui.menu_button("Deplacer vers un autre circuit", |ui| {
                            for target in 0..plan.circuit_count() {
                                if target == circuit_index { continue; }
                                let code = plan.circuit(target)
                                    .map(|c| c.letter.as_str()).unwrap_or("?");
                                if ui.button(format!("Circuit {code}")).clicked() {
                                    action = Some(Action::MoveGroup {
                                        switch_index: *switch_index,
                                        target_circuit: target,
                                    });
                                }
                            }
                        });
                    }
                });
            }
            if !outlets.is_empty() {
                ui.label("Prises du circuit :");
                for symbol_index in &outlets {
                    let ref_text = plan.point_ref(*symbol_index)
                        .unwrap_or_else(|| "?".to_owned());
                    ui.label(format!("Prise {ref_text} (repositionner et reaffecter sur le plan 2D)"));
                }
            }
            ui.label("Liaisons d'eclairage :");
            for (index, link) in plan.lighting_connections.iter().enumerate() {
                if link.circuit_index != circuit_index
                    || !plan.is_active_lighting_connection(index) {
                    continue;
                }
                let switch_number = plan.symbol_number(link.switch_index)
                    .unwrap_or(link.switch_index + 1);
                let lamp_number = plan.symbol_number(link.light_index)
                    .unwrap_or(link.light_index + 1);
                let reference = plan.lighting_point_ref(link.light_index)
                    .unwrap_or_else(|| "?".to_owned());
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "I{switch_number} -> L{lamp_number} (point {reference})"
                    ));
                    if ui.small_button(format!("Supprimer##link{index}")).clicked() {
                        action = Some(Action::RemoveConnection(index));
                    }
                });
            }
        }
        });
        });
    ui.small("Symboles de base du tableau 2.23 (RGIE Livre 1). Les branchements restent fonctionnels et les protections doivent etre identifies.");
    ui.small("Les variantes doivent correspondre au materiel pose. Ne pas utiliser ce prototype pour realiser un raccordement.");
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
        let new_index = p.add_lighting_circuit().unwrap();
        assert_eq!(p.lighting_groups_for_circuit(new_index).len(), 0);
    }
}
