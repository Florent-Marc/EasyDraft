//! Schéma unifilaire d'éclairage : représentation de travail.
//! Les liaisons (plan 2D / unifilaire) ont une seule source de vérité : Plan.
//! L'affichage ne constitue ni une validation RGIE ni un plan de câblage.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};

use crate::model::{ElectricalAppearance, ElectricalKind, Plan};
use crate::rgie_symbols;

const INK: Color32 = Color32::from_rgb(43, 56, 72);
const BLUE: Color32 = Color32::from_rgb(37, 107, 180);
const MUTED: Color32 = Color32::from_rgb(101, 111, 127);
const WARNING: Color32 = Color32::from_rgb(177, 93, 21);

/// Actions remontées au niveau de l'application, pour enregistrer une annulation.
pub enum Action {
    SelectCircuit(usize),
    SelectSymbol(usize),
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
        &rgie_symbols::schematic_device_primitives(ElectricalKind::Switch, appearance),
        INK, 0.91
    );
}

fn light_symbol(
    painter: &egui::Painter, at: Pos2, appearance: ElectricalAppearance
) {
    rgie_symbols::draw_egui(
        painter, at,
        &rgie_symbols::schematic_device_primitives(ElectricalKind::Light, appearance),
        INK, 1.1
    );
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
                    circuit.cable_label().as_deref().unwrap_or(""), 10.0, MUTED,
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

const CIRCUIT_WIDTH: f32 = 226.0;
const POINT_STEP: f32 = 44.0;

struct CircuitBranches {
    index: usize,
    commands: Vec<(usize, Vec<usize>)>,
    outlets: Vec<usize>,
}

impl CircuitBranches {
    fn new(plan: &Plan, index: usize) -> Self {
        Self {
            index,
            commands: plan.lighting_groups_for_circuit(index),
            outlets: plan.outlets_on_circuit(index),
        }
    }

    fn height(&self) -> f32 {
        let lamps: usize = self.commands.iter().map(|(_, lamps)| lamps.len()).sum();
        (lamps + self.outlets.len()) as f32 * POINT_STEP + self.commands.len() as f32 * 10.0
    }
}

fn symbol_hit(ui: &mut egui::Ui, index: usize, at: Pos2) -> bool {
    let rect = egui::Rect::from_center_size(at, Vec2::splat(36.0));
    let response = ui.interact(rect, ui.id().with(("unifilar_symbol", index)), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Modifier le symbole");
    response.clicked()
}

fn draw_circuit(
    ui: &mut egui::Ui, painter: &egui::Painter, plan: &Plan, branches: &CircuitBranches, foot: Pos2,
) -> Option<usize> {
    let circuit = plan.circuit(branches.index)?;
    let mut selected = None;
    let protection_y = foot.y - 40.0;
    let top = foot.y - 110.0 - branches.height();
    let x = foot.x;
    let size = 12.0;
    let name = format!("Circuit {}", circuit.letter);
    painter.text(Pos2::new(x, top - 36.0), Align2::LEFT_CENTER, name,
        FontId::proportional(15.0), BLUE);

    let mark = rgie_symbols::protection_from_text(&circuit.protection_kind);
    let gap = match mark {
        rgie_symbols::ProtectionMark::Unspecified => 12.0,
        rgie_symbols::ProtectionMark::Breaker => 19.0,
        rgie_symbols::ProtectionMark::Fuse => 21.0,
    };
    segment(painter, foot, Pos2::new(x, protection_y + gap), INK, 1.6);
    segment(painter, Pos2::new(x, protection_y - gap), Pos2::new(x, top), INK, 1.6);
    rgie_symbols::draw_egui(painter, Pos2::new(x, protection_y),
        &rgie_symbols::protection_primitives(mark), INK, 1.0);
    let column = egui::Rect::from_min_max(
        Pos2::new(x - 19.0, top - 50.0),
        Pos2::new(x + CIRCUIT_WIDTH - 19.0, foot.y),
    );
    let label_painter = painter.with_clip_rect(painter.clip_rect().intersect(column));
    for (offset, text) in [(39.0, Some(circuit.protection_label())), (83.0, circuit.cable_label())] {
        let Some(text) = text else { continue };
        label_painter.text(foot + Vec2::new(18.0, -offset), Align2::LEFT_CENTER,
            text, FontId::proportional(11.0), MUTED);
    }

    let mut y = top;
    for (switch_index, lamps) in &branches.commands {
        let switch_x = x + 47.0;
        let split_x = x + 93.0;
        let lamp_x = x + 136.0;
        let appearance = plan.symbol_appearance(*switch_index);
        let radius = rgie_symbols::schematic_switch_radius(appearance) * 0.91;
        segment(painter, Pos2::new(x, y), Pos2::new(switch_x - radius, y), INK, 1.5);
        painter.circle_filled(Pos2::new(x, y), 2.8, INK);
        let position = Pos2::new(switch_x, y);
        if symbol_hit(ui, *switch_index, position) {
            selected = Some(*switch_index);
        }
        switch_symbol(painter, position, appearance);
        if let Some(number) = plan.symbol_number(*switch_index) {
            caption(painter, Pos2::new(switch_x, y + 25.0), &format!("I{number}"), size, MUTED);
        }
        segment(painter, Pos2::new(switch_x + radius, y), Pos2::new(split_x, y), INK, 1.5);
        let last_y = y + lamps.len().saturating_sub(1) as f32 * POINT_STEP;
        if lamps.len() > 1 {
            segment(painter, Pos2::new(split_x, y), Pos2::new(split_x, last_y), INK, 1.5);
        }
        for (index, lamp) in lamps.iter().enumerate() {
            let row_y = y + index as f32 * POINT_STEP;
            segment(painter, Pos2::new(split_x, row_y), Pos2::new(lamp_x, row_y), INK, 1.5);
            let position = Pos2::new(lamp_x, row_y);
            if symbol_hit(ui, *lamp, position) {
                selected = Some(*lamp);
            }
            light_symbol(painter, position, plan.symbol_appearance(*lamp));
            if let Some(reference) = plan.lighting_point_ref(*lamp) {
                caption(painter, Pos2::new(lamp_x + 34.0, row_y), &reference, size, BLUE);
            }
        }
        y += lamps.len() as f32 * POINT_STEP + 10.0;
    }
    for outlet in &branches.outlets {
        let point = Pos2::new(x + 65.0, y);
        if symbol_hit(ui, *outlet, point) {
            selected = Some(*outlet);
        }
        segment(painter, Pos2::new(x, y), point + Vec2::new(-19.0 * 0.85, 0.0), INK, 1.5);
        painter.circle_filled(Pos2::new(x, y), 2.8, INK);
        rgie_symbols::draw_egui(painter, point,
            &rgie_symbols::schematic_device_primitives(ElectricalKind::Outlet, plan.symbol_appearance(*outlet)),
            INK, 0.85);
        if let Some(reference) = plan.point_ref(*outlet) {
            caption(painter, point + Vec2::new(49.0, 0.0), &reference, size, BLUE);
        }
        y += POINT_STEP;
    }
    if branches.commands.is_empty() && branches.outlets.is_empty() {
        label_painter.text(Pos2::new(x + 16.0, top), Align2::LEFT_CENTER, "Circuit vide",
            FontId::proportional(11.0), MUTED);
    }
    selected
}

/// Circuits côte à côte et rails distincts pour les DDR enregistrés.
fn show_schematic_sheet(ui: &mut egui::Ui, plan: &Plan, active: usize) -> Option<Action> {
    let mut groups: Vec<_> = plan.differentials.iter().enumerate().map(|(index, ddr)| {
        (Some(ddr), plan.circuits_for_differential(index))
    }).collect();
    let unassigned = plan.circuits_without_differential();
    if !unassigned.is_empty() {
        groups.push((None, unassigned));
    }
    let groups: Vec<_> = groups.into_iter().map(|(ddr, indices)| {
        let circuits = indices.into_iter().map(|index| CircuitBranches::new(plan, index))
            .collect::<Vec<_>>();
        (ddr, circuits)
    }).collect();
    let max_branches = groups.iter().flat_map(|(_, circuits)| circuits)
        .map(CircuitBranches::height).fold(0.0_f32, f32::max);
    let group_width = |count: usize| 44.0 + CIRCUIT_WIDTH * count.max(1) as f32;
    let width = 20.0 + groups.iter().map(|(_, circuits)| group_width(circuits.len()) + 16.0)
        .sum::<f32>();
    let height = max_branches + 260.0;

    egui::ScrollArea::both().id_salt("central_unifilar").auto_shrink([false, false]).show(ui, |ui| {
        let size = Vec2::new(width.max(ui.available_width()), height.max(ui.available_height()));
        let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
        painter.rect_filled(response.rect, 0.0, Color32::from_rgb(245, 249, 252));
        let origin = response.rect.center() - Vec2::new(width / 2.0, height / 2.0);
        let bus_y = origin.y + height - 82.0;
        let mut left = origin.x + 18.0;
        let mut action = None;
        for (ddr, circuits) in &groups {
            let width = group_width(circuits.len());
            let group_rect = egui::Rect::from_min_size(Pos2::new(left, origin.y + 10.0),
                Vec2::new(width, height - 20.0));
            painter.rect_filled(group_rect, 6.0, Color32::WHITE);
            let group_painter = painter.with_clip_rect(painter.clip_rect().intersect(group_rect));
            let first_x = left + 62.0;
            for (slot, circuit) in circuits.iter().enumerate() {
                let x = first_x + slot as f32 * CIRCUIT_WIDTH;
                let foot = Pos2::new(x, bus_y);
                let hit_area = egui::Rect::from_min_max(
                    Pos2::new(x - 16.0, bus_y - 159.0 - circuit.height()),
                    Pos2::new(x + CIRCUIT_WIDTH - 16.0, bus_y),
                ).intersect(group_rect);
                let hit = ui.interact(hit_area, ui.id().with(("unifilar_circuit", circuit.index)),
                    egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                if hit.hovered() || active == circuit.index {
                    group_painter.rect_filled(hit_area, 5.0, Color32::from_rgb(237, 245, 255));
                }
                if hit.clicked() {
                    action = Some(Action::SelectCircuit(circuit.index));
                }
                if let Some(index) = draw_circuit(ui, &group_painter, plan, circuit, foot) {
                    action = Some(Action::SelectSymbol(index));
                }
                if ddr.is_some() {
                    group_painter.circle_filled(foot, 3.0, INK);
                }
            }
            let (title, description, color) = match ddr {
                Some(ddr) => {
                    if let Some(last) = circuits.len().checked_sub(1) {
                        let rail_start = Pos2::new(left + 22.0, bus_y);
                        segment(&group_painter, rail_start,
                            Pos2::new(first_x + last as f32 * CIRCUIT_WIDTH, bus_y), INK, 2.1);
                        segment(&group_painter, rail_start, rail_start + Vec2::new(0.0, 12.0),
                            INK, 2.1);
                    }
                    (format!("DDR {}", ddr.reference), ddr.description(), INK)
                }
                None => (
                    "Circuits sans DDR attribué".to_owned(),
                    "Aucune liaison amont enregistrée".to_owned(), WARNING,
                ),
            };
            group_painter.text(Pos2::new(left + 22.0, bus_y + 25.0), Align2::LEFT_CENTER,
                title, FontId::proportional(14.0), color);
            group_painter.text(Pos2::new(left + 22.0, bus_y + 47.0), Align2::LEFT_CENTER,
                description, FontId::proportional(11.0), MUTED);
            left += width + 16.0;
        }
        action
    }).inner
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SchematicView {
    #[default]
    Diagram,
    Board,
    Connections,
}

pub fn show(ui: &mut egui::Ui, plan: &Plan, active: usize) -> Option<Action> {
    let view_id = ui.id().with("unifilar_view");
    let mut view = ui.data_mut(|data| data.get_temp::<SchematicView>(view_id)).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut view, SchematicView::Diagram, "Schéma");
        ui.selectable_value(&mut view, SchematicView::Board, "Topologie du tableau");
        ui.selectable_value(&mut view, SchematicView::Connections, "Circuits et liaisons");
    });
    ui.data_mut(|data| data.insert_temp(view_id, view));
    ui.separator();
    match view {
        SchematicView::Diagram => {
            if !plan.installation_info.address.trim().is_empty() {
                ui.label(&plan.installation_info.address);
            }
            show_schematic_sheet(ui, plan, active)
        }
        SchematicView::Board => {
            egui::ScrollArea::vertical().show(ui, |ui| show_board_overview(ui, plan));
            None
        }
        SchematicView::Connections => show_connections(ui, plan),
    }
}

fn show_connections(ui: &mut egui::Ui, plan: &Plan) -> Option<Action> {
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
    egui::ScrollArea::vertical().show(ui, |ui| {
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
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn central_view_draws_circuits_and_their_points_side_by_side() {
        let plan: Plan = serde_json::from_str(include_str!(
            "../examples/tableau-deux-ddr-trois-circuits.json"
        )).unwrap();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0))),
            ..Default::default()
        };
        let output = ctx.run(input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(show(ui, &plan, 0).is_none());
            });
        });
        let labels: Vec<_> = output.shapes.iter().filter_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                Some((text.galley.text(), text.pos))
            } else {
                None
            }
        }).collect();
        let position = |label| labels.iter().find(|(text, _)| *text == label).unwrap().1;
        let a = position("Circuit A");
        let b = position("Circuit B");
        let c = position("Circuit C");
        assert!(a.x < b.x && b.x < c.x);
        let (ddr_1, ddr_2) = (position("DDR D1"), position("DDR D2"));
        assert_eq!(ddr_1.y, ddr_2.y);
        assert!(position("A1").y < ddr_1.y);
        assert!(position("C1").x > c.x);

        let click = |pos| {
            let mut action = None;
            for pressed in [true, false] {
                let mut input = input.clone();
                input.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos, button: egui::PointerButton::Primary, pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                let _ = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(ui, &plan, 0);
                    });
                });
            }
            action
        };
        assert!(matches!(click(b + Vec2::new(16.0, 8.0)), Some(Action::SelectCircuit(1))));
        let light = plan.active_lighting_connections_for_circuit(0)[0].light_index;
        let light_pos = Pos2::new(a.x + 136.0, a.y + 44.0);
        assert!(matches!(click(light_pos), Some(Action::SelectSymbol(index)) if index == light));
    }

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
