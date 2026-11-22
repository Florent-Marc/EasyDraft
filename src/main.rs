mod model;
mod pdf_export;
mod unifilar;

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Sense, Stroke, Vec2};
use model::{
    ElectricalKind, ElectricalSymbol, Opening, OpeningKind, Plan, Point, Stair, Wall,
    DEFAULT_WALL_THICKNESS_M,
};

const GRID_METERS: f32 = 0.5;
const PIXELS_PER_METER: f32 = 80.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Plan2D,
    Unifilar,
}

/// Pointilles indiquant une relation logique, pas le passage reel des cables.
fn draw_logical_link(painter: &Painter, a: Pos2, b: Pos2) {
    let delta = b - a;
    let length = delta.length();
    if length < 0.1 { return; }
    let direction = delta / length;
    let mut along = 0.0;
    while along < length {
        painter.line_segment(
            [a + direction * along, a + direction * (along + 8.0_f32).min(length)],
            Stroke::new(1.6, Color32::from_rgb(31, 135, 192)),
        );
        along += 14.0;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Wall,
    SelectWall,
    Door,
    Window,
    Stair,
    Outlet,
    Switch,
    Light,
    LinkLighting,
    Pan,
}

struct EasyDraft {
    plan: Plan,
    undo: Vec<Plan>,
    tool: Tool,
    view: View,
    pending_switch: Option<usize>,
    pending_wall: Option<Point>,
    pending_stair: Option<Point>,
    selected_wall: Option<usize>,
    new_wall_thickness_cm: f32,
    new_door_width_cm: f32,
    new_window_width_cm: f32,
    new_stair_width_cm: f32,
    new_stair_steps: u16,
    door_hinge_right: bool,
    door_swing_flipped: bool,
    show_dimensions: bool,
    snap_m: f32,
    zoom: f32,
    pan: Vec2,
    grid: bool,
    file_path: String,
    pdf_path: String,
    unifilar_pdf_path: String,
    status: String,
}

impl Default for EasyDraft {
    fn default() -> Self {
        Self {
            plan: Plan::default(),
            undo: Vec::new(),
            tool: Tool::Wall,
            view: View::Plan2D,
            pending_switch: None,
            pending_wall: None,
            pending_stair: None,
            selected_wall: None,
            new_wall_thickness_cm: DEFAULT_WALL_THICKNESS_M * 100.0,
            new_door_width_cm: 90.0,
            new_window_width_cm: 120.0,
            new_stair_width_cm: 100.0,
            new_stair_steps: 12,
            door_hinge_right: false,
            door_swing_flipped: false,
            show_dimensions: true,
            snap_m: 0.05,
            zoom: 1.0,
            pan: Vec2::ZERO,
            grid: true,
            file_path: "mon-plan.json".into(),
            pdf_path: "plan-easydraft.pdf".into(),
            unifilar_pdf_path: "unifilaire-easydraft.pdf".into(),
            status: "Clique une fois pour commencer un mur, puis une seconde fois pour le terminer.".into(),
        }
    }
}

/// Cotation de l'axe du mur : trait de cote, lignes d'attache et valeur en metres.
fn draw_dimension(painter: &Painter, a: Pos2, b: Pos2, wall_width_px: f32, length_m: f32) {
    let segment = b - a;
    let len = segment.length();
    if len < 0.001 {
        return;
    }
    let direction = segment / len;
    let normal = Vec2::new(-direction.y, direction.x);
    let offset = wall_width_px * 0.5 + 26.0;
    let from = a + normal * offset;
    let to = b + normal * offset;
    let stroke = Stroke::new(1.1, Color32::from_rgb(71, 108, 148));
    for anchor in [a, b] {
        painter.line_segment(
            [anchor + normal * (wall_width_px * 0.5 + 4.0), anchor + normal * (offset + 7.0)],
            stroke,
        );
    }
    painter.line_segment([from, to], stroke);
    // Petites barres obliques a chaque extremite.
    for p in [from, to] {
        let tick = (direction + normal) * 4.0;
        painter.line_segment([p - tick, p + tick], Stroke::new(1.5, stroke.color));
    }
    let label_pos = from + (to - from) * 0.5 + normal * 12.0;
    painter.text(
        label_pos,
        Align2::CENTER_CENTER,
        format!("{length_m:.2} m"),
        FontId::proportional(13.0),
        Color32::from_rgb(45, 79, 118),
    );
}

/// Trace les parties pleines du mur en respectant l'epaisseur saisie.
fn draw_wall_segment(painter: &Painter, a: Pos2, b: Pos2, width_px: f32, selected: bool) {
    if (b - a).length_sq() < 0.001 {
        return;
    }
    if selected {
        painter.line_segment(
            [a, b],
            Stroke::new(width_px + 5.0, Color32::from_rgb(40, 135, 230)),
        );
    }
    painter.line_segment([a, b], Stroke::new(width_px, Color32::from_rgb(45, 53, 65)));
}

/// Dessine les tableaux de l'ouverture, la vitre ou le battant et son arc d'ouverture.
fn draw_opening(
    painter: &Painter,
    wall_start: Pos2,
    direction: Vec2,
    width_px: f32,
    ppm: f32,
    opening: &Opening,
) {
    let normal = Vec2::new(-direction.y, direction.x);
    let left = wall_start + direction * (opening.start_m() * ppm);
    let right = wall_start + direction * (opening.end_m() * ppm);
    let half_thickness = width_px * 0.5;
    let frame = Stroke::new(1.5, Color32::from_rgb(45, 53, 65));

    // Deux traits perpendiculaires ferment les extremites coupees du mur.
    for pos in [left, right] {
        painter.line_segment(
            [pos - normal * half_thickness, pos + normal * half_thickness],
            frame,
        );
    }

    match opening.kind {
        OpeningKind::Window => {
            let glass = Stroke::new(1.8, Color32::from_rgb(35, 136, 188));
            for offset in [-0.3_f32, 0.0, 0.3] {
                let shift = normal * (width_px * offset);
                painter.line_segment([left + shift, right + shift], glass);
            }
        }
        OpeningKind::Door => {
            let hinge = if opening.hinge_right { right } else { left };
            let along_sign = if opening.hinge_right { -1.0 } else { 1.0 };
            let swing_sign = if opening.swing_flipped { -1.0 } else { 1.0 };
            let radius = opening.width_m * ppm;
            let leaf_end = hinge + normal * (swing_sign * radius);
            let stroke = Stroke::new(1.6, Color32::from_rgb(45, 92, 139));
            painter.line_segment([hinge, leaf_end], stroke);

            let mut arc = Vec::with_capacity(17);
            for i in 0..=16 {
                let angle = (i as f32 / 16.0) * std::f32::consts::FRAC_PI_2;
                arc.push(
                    hinge
                        + direction * (along_sign * radius * angle.cos())
                        + normal * (swing_sign * radius * angle.sin()),
                );
            }
            painter.add(egui::Shape::line(
                arc,
                Stroke::new(1.0, Color32::from_rgb(98, 130, 164)),
            ));
        }
    }
}

/// Dessine un escalier droit vu de dessus, avec marches et fleche de montee.
fn draw_stair(painter: &Painter, stair: &Stair, origin: Pos2, ppm: f32) {
    if !stair.is_valid() { return; }
    let to_screen = |p: Point| Pos2::new(origin.x + p.x * ppm, origin.y + p.y * ppm);
    let corners = stair.corners().map(to_screen);
    let dark = Stroke::new(1.8, Color32::from_rgb(49, 59, 73));
    for i in 0..4 {
        painter.line_segment([corners[i], corners[(i + 1) % 4]], dark);
    }
    for i in 1..stair.steps {
        let t = i as f32 / stair.steps as f32;
        let p = corners[0] + (corners[1] - corners[0]) * t;
        let q = corners[3] + (corners[2] - corners[3]) * t;
        painter.line_segment([p, q], Stroke::new(1.0, Color32::from_rgb(105, 116, 131)));
    }
    let a = to_screen(stair.start);
    let b = to_screen(stair.end);
    let direction = (b - a).normalized();
    let normal = Vec2::new(-direction.y, direction.x);
    let from = a + (b - a) * 0.2;
    let to = a + (b - a) * 0.83;
    let arrow = Stroke::new(2.0, Color32::from_rgb(27, 123, 204));
    painter.line_segment([from, to], arrow);
    painter.line_segment([to, to - direction * 13.0 + normal * 6.0], arrow);
    painter.line_segment([to, to - direction * 13.0 - normal * 6.0], arrow);
}

impl EasyDraft {
    fn snapshot(&mut self) {
        self.undo.push(self.plan.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
    }

    fn undo(&mut self) {
        if let Some(plan) = self.undo.pop() {
            self.plan = plan;
            self.pending_wall = None;
            self.pending_stair = None;
            self.pending_switch = None;
            self.selected_wall = None;
            self.status = "Derniere modification annulee".into();
        }
    }

    fn save(&mut self) {
        let result = serde_json::to_string_pretty(&self.plan)
            .map_err(|e| e.to_string())
            .and_then(|data| std::fs::write(&self.file_path, data).map_err(|e| e.to_string()));
        self.status = match result {
            Ok(()) => format!("Enregistre : {}", self.file_path),
            Err(e) => format!("Erreur : {e}"),
        };
    }

    fn load(&mut self) {
        let result = std::fs::read_to_string(&self.file_path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Plan>(&s).map_err(|e| e.to_string()));
        match result {
            Ok(plan) => {
                self.snapshot();
                self.plan = plan;
                self.pending_wall = None;
                self.pending_stair = None;
                self.pending_switch = None;
                self.selected_wall = None;
                self.status = format!("Ouvert : {}", self.file_path);
            }
            Err(e) => self.status = format!("Erreur : {e}"),
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.pending_wall = None;
        self.pending_stair = None;
        self.pending_switch = None;
    }

    fn find_wall(&self, point: Point, ppm: f32) -> Option<usize> {
        let mut closest = None;
        let mut closest_distance = f32::INFINITY;
        for (index, wall) in self.plan.walls.iter().enumerate() {
            let dx = wall.end.x - wall.start.x;
            let dy = wall.end.y - wall.start.y;
            let len_squared = dx * dx + dy * dy;
            let t = if len_squared > 0.000001 {
                (((point.x - wall.start.x) * dx + (point.y - wall.start.y) * dy) / len_squared)
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            let closest_point = Point {
                x: wall.start.x + dx * t,
                y: wall.start.y + dy * t,
            };
            let distance = point.distance_to(closest_point);
            let tolerance_m = wall.thickness_m.max(0.0) * 0.5 + 9.0 / ppm;
            if distance <= tolerance_m && distance < closest_distance {
                closest = Some(index);
                closest_distance = distance;
            }
        }
        closest
    }

    fn find_electrical_symbol(&self, point: Point, ppm: f32, kind: ElectricalKind) -> Option<usize> {
        self.plan.symbols.iter().enumerate()
            .filter(|(_, symbol)| symbol.kind == kind)
            .map(|(index, symbol)| (index, point.distance_to(symbol.position)))
            .filter(|(_, distance)| *distance <= 18.0 / ppm)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    fn remove_connection(&mut self, index: usize) {
        if index < self.plan.lighting_connections.len() {
            self.snapshot();
            self.plan.lighting_connections.remove(index);
            self.pending_switch = None;
            self.status = "Liaison supprimee du plan et du schema unifilaire".into();
        }
    }

    fn add_lighting_demo(&mut self) {
        self.snapshot();
        self.plan.add_demo_lighting();
        self.pending_switch = None;
        self.view = View::Plan2D;
        self.status = "Exemple ajoute : I et L sont lies. Clique 'Schema unifilaire' pour voir le resultat.".into();
    }

    fn export_unifilar_pdf(&mut self) {
        self.status = match pdf_export::export_unifilar_pdf(&self.unifilar_pdf_path, &self.plan) {
            Ok(()) => format!("PDF unifilaire exporte : {}", self.unifilar_pdf_path),
            Err(error) => error,
        };
    }

    fn export_pdf(&mut self) {
        self.status = match pdf_export::export_pdf(&self.pdf_path, &self.plan, self.show_dimensions) {
            Ok(()) => format!("PDF exporte : {}", self.pdf_path),
            Err(error) => error,
        };
    }

    fn place(&mut self, position: Point, ppm: f32) {
        match self.tool {
            Tool::Wall => {
                if let Some(start) = self.pending_wall.take() {
                    let distance = start.distance_to(position);
                    if distance > 0.001 {
                        self.snapshot();
                        self.plan.walls.push(Wall {
                            start,
                            end: position,
                            thickness_m: self.new_wall_thickness_cm / 100.0,
                        });
                        self.status = format!("Mur de {distance:.2} m ajoute");
                    }
                } else {
                    self.pending_wall = Some(position);
                    self.status = "Choisis le deuxieme point du mur".into();
                }
            }
            Tool::Stair => {
                if let Some(start) = self.pending_stair.take() {
                    let stair = Stair {
                        start,
                        end: position,
                        width_m: self.new_stair_width_cm / 100.0,
                        steps: self.new_stair_steps,
                    };
                    if stair.is_valid() {
                        self.snapshot();
                        self.plan.stairs.push(stair);
                        self.status = "Escalier ajoute (fleche dans le sens de la montee)".into();
                    } else {
                        self.status = "L'escalier doit mesurer au moins 5 cm".into();
                    }
                } else {
                    self.pending_stair = Some(position);
                    self.status = "Clique le deuxieme point pour definir la montee".into();
                }
            }
            Tool::Door | Tool::Window => {
                if let Some(wall_index) = self.find_wall(position, ppm) {
                    let kind = if self.tool == Tool::Door {
                        OpeningKind::Door
                    } else {
                        OpeningKind::Window
                    };
                    let width_m = if self.tool == Tool::Door {
                        self.new_door_width_cm / 100.0
                    } else {
                        self.new_window_width_cm / 100.0
                    };
                    // On valide avant de creer une entree dans l'historique d'annulation.
                    let mut updated = self.plan.clone();
                    match updated.add_opening(
                        wall_index,
                        position,
                        width_m,
                        kind,
                        self.door_hinge_right,
                        self.door_swing_flipped,
                        self.snap_m,
                    ) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.status = match kind {
                                OpeningKind::Door => format!("Porte de {:.0} cm ajoutee", width_m * 100.0),
                                OpeningKind::Window => format!("Fenetre de {:.0} cm ajoutee", width_m * 100.0),
                            };
                        }
                        Err(err) => self.status = err.into(),
                    }
                } else {
                    self.status = "Clique sur un mur pour y placer l'ouverture".into();
                }
            }
            Tool::LinkLighting => {
                if let Some(switch_index) = self.pending_switch {
                    if let Some(light_index) = self.find_electrical_symbol(position, ppm, ElectricalKind::Light) {
                        let mut updated = self.plan.clone();
                        match updated.connect_switch_to_light(switch_index, light_index) {
                            Ok(()) => {
                                self.snapshot();
                                self.plan = updated;
                                self.pending_switch = None;
                                self.status = "Liaison enregistree : visible dans le schema unifilaire.".into();
                            }
                            Err(error) => self.status = error.into(),
                        }
                    } else {
                        self.status = "Clique sur une lampe (X) pour terminer la liaison.".into();
                    }
                } else if let Some(index) = self.find_electrical_symbol(position, ppm, ElectricalKind::Switch) {
                    self.pending_switch = Some(index);
                    self.status = "Interrupteur selectionne. Clique maintenant sur une lampe (X).".into();
                } else {
                    self.status = "Clique d'abord sur un interrupteur (I).".into();
                }
            }
            Tool::Outlet | Tool::Switch | Tool::Light => {
                let kind = match self.tool {
                    Tool::Outlet => ElectricalKind::Outlet,
                    Tool::Switch => ElectricalKind::Switch,
                    _ => ElectricalKind::Light,
                };
                self.snapshot();
                self.plan.symbols.push(ElectricalSymbol { position, kind });
                self.status = "Symbole ajoute".into();
            }
            Tool::SelectWall => {
                self.selected_wall = self.find_wall(position, ppm);
                self.status = match self.selected_wall {
                    Some(i) => format!("Mur n°{} selectionne", i + 1),
                    None => "Aucun mur a cet endroit".into(),
                };
            }
            Tool::Pan => {}
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(249, 250, 252));

        if self.tool == Tool::Pan && response.dragged() {
            self.pan += ui.ctx().input(|i| i.pointer.delta());
        }

        let ppm = PIXELS_PER_METER * self.zoom;
        let origin = rect.center() + self.pan;
        let to_screen = |p: Point| -> Pos2 {
            Pos2::new(origin.x + p.x * ppm, origin.y + p.y * ppm)
        };
        let to_world = |p: Pos2| -> Point {
            Point { x: (p.x - origin.x) / ppm, y: (p.y - origin.y) / ppm }
        };

        if self.grid {
            let step = GRID_METERS * ppm;
            let left = ((rect.left() - origin.x) / step).floor() as i32;
            let right = ((rect.right() - origin.x) / step).ceil() as i32;
            let top = ((rect.top() - origin.y) / step).floor() as i32;
            let bottom = ((rect.bottom() - origin.y) / step).ceil() as i32;
            for i in left.max(-200)..=right.min(200) {
                let x = origin.x + i as f32 * step;
                let color = if i == 0 { Color32::GRAY } else { Color32::from_gray(225) };
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(1.0, color));
            }
            for i in top.max(-200)..=bottom.min(200) {
                let y = origin.y + i as f32 * step;
                let color = if i == 0 { Color32::GRAY } else { Color32::from_gray(225) };
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(1.0, color));
            }
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let world = to_world(pos);
                let point = if matches!(self.tool, Tool::SelectWall | Tool::Door | Tool::Window | Tool::LinkLighting) {
                    world
                } else {
                    world.snapped(self.snap_m)
                };
                self.place(point, ppm);
            }
        }

        for (index, wall) in self.plan.walls.iter().enumerate() {
            let a = to_screen(wall.start);
            let b = to_screen(wall.end);
            let width_px = wall.thickness_m.max(0.01) * ppm;
            let screen_length = (b - a).length();
            if screen_length > 0.001 {
                let direction = (b - a) / screen_length;
                let mut openings: Vec<&Opening> = self.plan.openings.iter()
                    .filter(|o| o.wall_index == index)
                    .collect();
                openings.sort_by(|first, second| first.start_m().total_cmp(&second.start_m()));
                let mut cursor_m = 0.0_f32;
                for opening in openings {
                    let from_m = opening.start_m().clamp(0.0, wall.length_m());
                    let to_m = opening.end_m().clamp(0.0, wall.length_m());
                    if from_m > cursor_m {
                        draw_wall_segment(
                            &painter,
                            a + direction * (cursor_m * ppm),
                            a + direction * (from_m * ppm),
                            width_px,
                            self.selected_wall == Some(index),
                        );
                    }
                    draw_opening(&painter, a, direction, width_px, ppm, opening);
                    cursor_m = cursor_m.max(to_m);
                }
                if cursor_m < wall.length_m() {
                    draw_wall_segment(
                        &painter,
                        a + direction * (cursor_m * ppm),
                        b,
                        width_px,
                        self.selected_wall == Some(index),
                    );
                }
            }
            if self.show_dimensions {
                draw_dimension(&painter, a, b, width_px, wall.length_m());
            }
        }

        for stair in &self.plan.stairs {
            draw_stair(&painter, stair, origin, ppm);
        }

        for link in &self.plan.lighting_connections {
            if !self.plan.lighting_connection_is_valid(link) { continue; }
            let a = to_screen(self.plan.symbols[link.switch_index].position);
            let b = to_screen(self.plan.symbols[link.light_index].position);
            draw_logical_link(&painter, a, b);
        }
        if let Some(index) = self.pending_switch {
            if let (Some(symbol), Some(cursor)) = (self.plan.symbols.get(index), response.hover_pos()) {
                draw_logical_link(&painter, to_screen(symbol.position), cursor);
            }
        }

        for (index, symbol) in self.plan.symbols.iter().enumerate() {
            let pos = to_screen(symbol.position);
            painter.circle_filled(pos, 13.0, Color32::WHITE);
            painter.circle_stroke(pos, 13.0, Stroke::new(2.0, Color32::from_rgb(20, 106, 190)));
            let label = match symbol.kind {
                ElectricalKind::Outlet => "P",
                ElectricalKind::Switch => "I",
                ElectricalKind::Light => "X",
            };
            painter.text(pos, Align2::CENTER_CENTER, label, FontId::proportional(15.0), Color32::from_rgb(20, 106, 190));
            let number = self.plan.symbol_number(index).unwrap_or(index + 1);
            let ref_suffix = if symbol.kind == ElectricalKind::Light {
                self.plan.lighting_point_number(index).map(|n| format!(
                    " ({}{})",
                    self.plan.lighting_circuit.letter,
                    n
                )).unwrap_or_default()
            } else {
                String::new()
            };
            painter.text(
                pos + Vec2::new(0.0, 23.0),
                Align2::CENTER_CENTER,
                format!("{label}{number}{ref_suffix}"),
                FontId::proportional(12.0),
                Color32::from_rgb(35, 77, 123),
            );
            if self.pending_switch == Some(index) {
                painter.circle_stroke(pos, 17.0, Stroke::new(2.0, Color32::from_rgb(233, 139, 40)));
            }
        }

        if let Some(start) = self.pending_stair {
            if let Some(cursor) = response.hover_pos() {
                let end = to_world(cursor).snapped(self.snap_m);
                let preview = Stair {
                    start, end,
                    width_m: self.new_stair_width_cm / 100.0,
                    steps: self.new_stair_steps,
                };
                draw_stair(&painter, &preview, origin, ppm);
            }
        }

        if let Some(start) = self.pending_wall {
            if let Some(cursor) = response.hover_pos() {
                let end = to_world(cursor).snapped(self.snap_m);
                let width_px = self.new_wall_thickness_cm / 100.0 * ppm;
                painter.line_segment(
                    [to_screen(start), to_screen(end)],
                    Stroke::new(width_px.max(1.0), Color32::from_rgb(40, 130, 215)),
                );
                if self.show_dimensions {
                    draw_dimension(&painter, to_screen(start), to_screen(end), width_px, start.distance_to(end));
                }
            }
        }

        painter.text(
            rect.left_top() + Vec2::new(12.0, 12.0),
            Align2::LEFT_TOP,
            format!(
                "Grille : 0,5 m | Pas : {:.0} cm | Zoom : {:.0} %",
                self.snap_m * 100.0,
                self.zoom * 100.0
            ),
            FontId::proportional(13.0),
            Color32::DARK_GRAY,
        );
    }
}

impl eframe::App for EasyDraft {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("EasyDraft");
                ui.separator();
                if ui.selectable_label(self.view == View::Plan2D, "Plan 2D").clicked() {
                    self.view = View::Plan2D;
                }
                if ui.selectable_label(self.view == View::Unifilar, "Schema unifilaire").clicked() {
                    self.view = View::Unifilar;
                }
                ui.separator();
                if ui.button("Ajouter demo I -> L").clicked() { self.add_lighting_demo(); }
                ui.separator();
                if ui.button("Nouveau").clicked() {
                    self.snapshot();
                    self.plan = Plan::default();
                    self.pending_wall = None;
                    self.pending_stair = None;
                    self.pending_switch = None;
                    self.selected_wall = None;
                    self.view = View::Plan2D;
                    self.status = "Nouveau plan".into();
                }
                if ui.button("Annuler").clicked() { self.undo(); }
                if ui.button("Enregistrer").clicked() { self.save(); }
                if ui.button("Ouvrir").clicked() { self.load(); }
                if ui.button("Exporter PDF plan").clicked() { self.export_pdf(); }
                if self.view == View::Unifilar && ui.button("Exporter PDF unifilaire").clicked() {
                    self.export_unifilar_pdf();
                }
            });
        });

        egui::SidePanel::left("tools").show(ctx, |ui| {
            ui.heading("Outils");
            ui.separator();
            if ui.selectable_label(self.tool == Tool::Wall, "Mur").clicked() { self.select_tool(Tool::Wall); }
            if ui.selectable_label(self.tool == Tool::SelectWall, "Selectionner un mur").clicked() {
                self.select_tool(Tool::SelectWall);
            }
            if ui.selectable_label(self.tool == Tool::Door, "Porte").clicked() {
                self.select_tool(Tool::Door);
            }
            if ui.selectable_label(self.tool == Tool::Window, "Fenetre").clicked() {
                self.select_tool(Tool::Window);
            }
            if ui.selectable_label(self.tool == Tool::Stair, "Escalier").clicked() {
                self.select_tool(Tool::Stair);
            }
            if ui.selectable_label(self.tool == Tool::Outlet, "Prise").clicked() { self.select_tool(Tool::Outlet); }
            if ui.selectable_label(self.tool == Tool::Switch, "Interrupteur").clicked() { self.select_tool(Tool::Switch); }
            if ui.selectable_label(self.tool == Tool::Light, "Point lumineux").clicked() { self.select_tool(Tool::Light); }
            if ui.selectable_label(self.tool == Tool::LinkLighting, "Lier I -> L").clicked() {
                self.select_tool(Tool::LinkLighting);
                self.view = View::Plan2D;
                self.status = "Clique sur un interrupteur I, puis sur une lampe X.".into();
            }
            if ui.selectable_label(self.tool == Tool::Pan, "Deplacer la vue").clicked() { self.select_tool(Tool::Pan); }
            ui.separator();
            if self.tool == Tool::Door {
                ui.label("Largeur de la porte");
                ui.add(egui::Slider::new(&mut self.new_door_width_cm, 50.0..=200.0)
                    .step_by(5.0).suffix(" cm"));
                ui.checkbox(&mut self.door_hinge_right, "Charniere a la fin du mur");
                ui.checkbox(&mut self.door_swing_flipped, "Inverser le sens d'ouverture");
                ui.small("Clique sur un mur pour inserer la porte.");
                ui.separator();
            }
            if self.tool == Tool::Stair {
                ui.label("Largeur de l'escalier");
                ui.add(egui::Slider::new(&mut self.new_stair_width_cm, 60.0..=250.0)
                    .step_by(5.0).suffix(" cm"));
                ui.label("Nombre de marches");
                ui.add(egui::Slider::new(&mut self.new_stair_steps, 3..=30));
                ui.small("Deux clics : depart puis arrivee (sens de montee).");
                ui.separator();
            }
            if self.tool == Tool::Window {
                ui.label("Largeur de la fenetre");
                ui.add(egui::Slider::new(&mut self.new_window_width_cm, 40.0..=400.0)
                    .step_by(5.0).suffix(" cm"));
                ui.small("Clique sur un mur pour inserer la fenetre.");
                ui.separator();
            }
            ui.label("Epaisseur des nouveaux murs");
            ui.add(egui::Slider::new(&mut self.new_wall_thickness_cm, 5.0..=60.0).text("cm").step_by(1.0));
            if let Some(i) = self.selected_wall {
                if let Some(wall) = self.plan.walls.get(i) {
                    let mut thickness_cm = wall.thickness_m * 100.0;
                    ui.label(format!("Epaisseur du mur n°{}", i + 1));
                    if ui.add(egui::DragValue::new(&mut thickness_cm).speed(1.0).range(5.0..=60.0).suffix(" cm")).changed() {
                        self.snapshot();
                        if let Some(selected) = self.plan.walls.get_mut(i) {
                            selected.thickness_m = thickness_cm / 100.0;
                        }
                    }
                }
            }
            ui.checkbox(&mut self.show_dimensions, "Afficher les cotes (axe des murs)");
            ui.label("Pas d'aimantation");
            ui.add(egui::Slider::new(&mut self.snap_m, 0.05..=0.5).step_by(0.05).suffix(" m"));
            ui.separator();
            ui.label("Zoom");
            ui.add(egui::Slider::new(&mut self.zoom, 0.4..=2.5));
            ui.checkbox(&mut self.grid, "Afficher la grille");
            if ui.button("Recentrer").clicked() {
                self.pan = Vec2::ZERO;
                self.zoom = 1.0;
            }
            ui.separator();
            ui.label("Fichier JSON");
            ui.text_edit_singleline(&mut self.file_path);
            ui.separator();
            ui.label("Fichier PDF du plan");
            ui.text_edit_singleline(&mut self.pdf_path);
            if ui.button("Exporter le plan PDF").clicked() { self.export_pdf(); }
            ui.separator();
            ui.label("Fichier PDF unifilaire");
            ui.text_edit_singleline(&mut self.unifilar_pdf_path);
            if ui.button("Exporter l'unifilaire PDF").clicked() {
                self.export_unifilar_pdf();
            }
            ui.separator();
            ui.label(format!("Murs : {}", self.plan.walls.len()));
            ui.label(format!(
                "Portes : {}",
                self.plan.openings.iter().filter(|o| o.kind == OpeningKind::Door).count()
            ));
            ui.label(format!(
                "Fenetres : {}",
                self.plan.openings.iter().filter(|o| o.kind == OpeningKind::Window).count()
            ));
            ui.label(format!("Escaliers : {}", self.plan.stairs.len()));
            ui.label(format!("Symboles : {}", self.plan.symbols.len()));
            ui.label(format!("Liaisons I -> L : {}", self.plan.lighting_connections.len()));
            ui.label(format!("Lampes non liees : {}", self.plan.unlinked_lights()));
            ui.label(format!("Interrupteurs non lies : {}", self.plan.unlinked_switches()));
            ui.separator();
            ui.collapsing(format!("Circuit {} - parametres", self.plan.lighting_circuit.letter), |ui| {
                ui.small("Valeurs a renseigner d'apres l'installation reelle. 0 = inconnu.");
                let mut circuit = self.plan.lighting_circuit.clone();
                ui.label("Repere du circuit");
                ui.text_edit_singleline(&mut circuit.letter);
                ui.label("Disjoncteur - calibre");
                ui.add(egui::DragValue::new(&mut circuit.breaker_amp).range(0..=63).suffix(" A"));
                ui.label("Nombre de poles");
                ui.add(egui::DragValue::new(&mut circuit.breaker_poles).range(0..=4));
                ui.label("Courbe / type (ex. C)");
                ui.text_edit_singleline(&mut circuit.breaker_curve);
                ui.label("Differentiel en amont (ex. type / mA / A)");
                ui.text_edit_singleline(&mut circuit.upstream_rdc);
                ui.separator();
                ui.label("Type de canalisation (ex. XVB)");
                ui.text_edit_singleline(&mut circuit.cable_type);
                ui.label("Nombre de conducteurs");
                ui.add(egui::DragValue::new(&mut circuit.conductor_count).range(0..=10));
                ui.label("Section des conducteurs");
                ui.add(egui::DragValue::new(&mut circuit.section_mm2).speed(0.1)
                    .range(0.0..=35.0).suffix(" mm²"));
                ui.label("Mode de pose");
                ui.text_edit_singleline(&mut circuit.installation_method);
                if circuit != self.plan.lighting_circuit {
                    self.snapshot();
                    self.plan.lighting_circuit = circuit;
                    self.status = "Caracteristiques du circuit mises a jour".into();
                }
                let missing = self.plan.lighting_circuit.missing_fields();
                if !missing.is_empty() {
                    ui.colored_label(
                        Color32::from_rgb(176, 91, 13),
                        format!("{} champs non renseignes", missing.len()),
                    );
                } else {
                    ui.small("Champs remplis ; conformite RGIE non verifiee.");
                }
            });
            ui.separator();
            if self.view == View::Unifilar {
                ui.small("Le schema est automatiquement tire des liaisons du plan 2D.");
                if ui.button("Retour au plan 2D").clicked() {
                    self.view = View::Plan2D;
                }
            } else {
                ui.small("La liaison pointillee est logique, pas un chemin de cable.");
            }
            ui.small("Prototype uniquement : pas de validation RGIE.");
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view {
                View::Plan2D => self.canvas(ui),
                View::Unifilar => {
                    if let Some(unifilar::Action::RemoveConnection(index)) =
                        unifilar::show(ui, &self.plan)
                    {
                        self.remove_connection(index);
                    }
                },
            }
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "EasyDraft",
        options,
        Box::new(|_cc| Ok(Box::new(EasyDraft::default()))),
    )
}
