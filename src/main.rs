mod model;
mod pdf_export;
mod unifilar;
mod rooms;
mod rgie_review;

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
    RgieReview,
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

#[derive(Clone, Copy, Debug)]
enum DragTarget {
    WallStart(usize),
    WallEnd(usize),
    WallBody(usize),
    Opening(usize),
}

struct ActiveDrag {
    target: DragTarget,
    /// Position de la souris au debut du glissement (en metres).
    from: Point,
    /// Etat du plan avant le glissement : une seule entree d'annulation par glissement.
    original: Plan,
    changed: bool,
}

struct EasyDraft {
    plan: Plan,
    undo: Vec<Plan>,
    redo: Vec<Plan>,
    tool: Tool,
    view: View,
    pending_switch: Option<usize>,
    active_circuit_index: usize,
    pending_wall: Option<Point>,
    pending_stair: Option<Point>,
    selected_wall: Option<usize>,
    selected_symbol: Option<usize>,
    selected_stair: Option<usize>,
    selected_room: Option<Point>,
    selected_opening: Option<usize>,
    drag: Option<ActiveDrag>,
    new_wall_thickness_cm: f32,
    new_door_width_cm: f32,
    new_window_width_cm: f32,
    new_stair_width_cm: f32,
    new_stair_steps: u16,
    door_hinge_right: bool,
    door_swing_flipped: bool,
    show_dimensions: bool,
    show_architecture: bool,
    show_rooms: bool,
    show_electrical: bool,
    snap_endpoints: bool,
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
            redo: Vec::new(),
            tool: Tool::Wall,
            view: View::Plan2D,
            pending_switch: None,
            active_circuit_index: 0,
            pending_wall: None,
            pending_stair: None,
            selected_wall: None,
            selected_symbol: None,
            selected_stair: None,
            selected_room: None,
            selected_opening: None,
            drag: None,
            new_wall_thickness_cm: DEFAULT_WALL_THICKNESS_M * 100.0,
            new_door_width_cm: 90.0,
            new_window_width_cm: 120.0,
            new_stair_width_cm: 100.0,
            new_stair_steps: 12,
            door_hinge_right: false,
            door_swing_flipped: false,
            show_dimensions: true,
            show_architecture: true,
            show_rooms: true,
            show_electrical: true,
            snap_endpoints: true,
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
        self.redo.clear();
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
    }

    /// Annule un glissement non confirme et restaure l'etat d'origine.
    fn cancel_drag(&mut self) {
        if let Some(session) = self.drag.take() {
            self.plan = session.original;
        }
    }

    fn clear_selection(&mut self) {
        self.selected_wall = None;
        self.selected_symbol = None;
        self.selected_stair = None;
        self.selected_room = None;
        self.selected_opening = None;
    }

    fn reset_pending(&mut self) {
        self.pending_wall = None;
        self.pending_stair = None;
        self.pending_switch = None;
    }

    fn undo(&mut self) {
        self.cancel_drag();
        if let Some(plan) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.plan, plan));
            self.reset_pending();
            self.clear_selection();
            self.status = "Derniere modification annulee".into();
        }
    }

    fn redo(&mut self) {
        self.cancel_drag();
        if let Some(plan) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.plan, plan));
            self.reset_pending();
            self.clear_selection();
            self.status = "Modification retablie".into();
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
                self.cancel_drag();
                self.reset_pending();
                self.clear_selection();
                self.active_circuit_index = 0;
                self.status = format!("Ouvert : {}", self.file_path);
            }
            Err(e) => self.status = format!("Erreur : {e}"),
        }
    }

    fn load_unifilar_example(&mut self) {
        self.cancel_drag();
        match serde_json::from_str::<Plan>(
            include_str!("../examples/unifilaire-deux-circuits.json")
        ) {
            Ok(plan) => {
                self.snapshot();
                self.plan = plan;
                self.reset_pending();
                self.clear_selection();
                self.active_circuit_index = 0;
                self.view = View::Unifilar;
                self.show_electrical = true;
                self.status = "Exemple charge : circuits A et B, trois lampes, deux interrupteurs".into();
            }
            Err(error) => self.status = format!("Exemple invalide : {error}"),
        }
    }

    fn load_room_example(&mut self) {
        self.cancel_drag();
        let result = serde_json::from_str::<Plan>(
            include_str!("../examples/maison-deux-pieces.json")
        );
        match result {
            Ok(plan) => {
                self.snapshot();
                self.plan = plan;
                self.reset_pending();
                self.clear_selection();
                self.active_circuit_index = 0;
                self.view = View::Plan2D;
                self.show_architecture = true;
                self.show_rooms = true;
                self.show_electrical = true;
                self.pan = Vec2::ZERO;
                self.zoom = 1.0;
                self.status = "Exemple charge : Salon, Cuisine, porte et eclairage (Annuler pour revenir)".into();
            }
            Err(err) => self.status = format!("Exemple invalide : {err}"),
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        self.cancel_drag();
        self.tool = tool;
        self.reset_pending();
        if matches!(tool, Tool::Outlet | Tool::Switch | Tool::Light | Tool::LinkLighting) {
            self.show_electrical = true;
        }
        if matches!(tool, Tool::Wall | Tool::Door | Tool::Window | Tool::Stair) {
            self.show_architecture = true;
        }
    }

    /// Priorite aux extremites des autres murs lorsqu'elles sont proches du pointeur.
    fn snap_point(&self, raw: Point, ppm: f32) -> Point {
        if self.snap_endpoints {
            if let Some(endpoint) = self.plan.walls.iter()
                .flat_map(|wall| [wall.start, wall.end])
                .filter(|endpoint| raw.distance_to(*endpoint) <= 13.0 / ppm)
                .min_by(|a, b| raw.distance_to(*a).total_cmp(&raw.distance_to(*b)))
            {
                return endpoint;
            }
        }
        raw.snapped(self.snap_m)
    }

    fn find_symbol_any(&self, point: Point, ppm: f32) -> Option<usize> {
        self.plan.symbols.iter().enumerate()
            .map(|(index, symbol)| (index, point.distance_to(symbol.position)))
            .filter(|(_, distance)| *distance <= 17.0 / ppm)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    fn find_stair(&self, point: Point, ppm: f32) -> Option<usize> {
        self.plan.stairs.iter().enumerate().find_map(|(index, stair)| {
            if !stair.is_valid() { return None; }
            let length = stair.length_m();
            let dx = (stair.end.x - stair.start.x) / length;
            let dy = (stair.end.y - stair.start.y) / length;
            let px = point.x - stair.start.x;
            let py = point.y - stair.start.y;
            let along = px * dx + py * dy;
            let across = -px * dy + py * dx;
            let tolerance = 7.0 / ppm;
            if along >= -tolerance && along <= length + tolerance
                && across.abs() <= stair.width_m / 2.0 + tolerance {
                Some(index)
            } else {
                None
            }
        })
    }

    /// Recherche une ouverture par sa projection sur le mur porteur.
    fn find_opening(&self, point: Point, ppm: f32) -> Option<usize> {
        self.plan.openings.iter().enumerate()
            .filter_map(|(index, opening)| {
                let wall = self.plan.walls.get(opening.wall_index)?;
                let length = wall.length_m();
                if length < 0.001 { return None; }
                let projection = wall.projected_distance_m(point);
                let distance_from_axis = {
                    let ratio = projection / length;
                    let closest = Point {
                        x: wall.start.x + (wall.end.x - wall.start.x) * ratio,
                        y: wall.start.y + (wall.end.y - wall.start.y) * ratio,
                    };
                    point.distance_to(closest)
                };
                let along_tolerance = 7.0 / ppm;
                let tolerance = wall.thickness_m * 0.5 + 11.0 / ppm;
                if projection >= opening.start_m() - along_tolerance
                    && projection <= opening.end_m() + along_tolerance
                    && distance_from_axis <= tolerance
                {
                    Some((index, distance_from_axis))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    /// Selectionne d'abord une ouverture, puis une poignee de mur, puis le corps du mur.
    fn drag_target_at(&self, point: Point, ppm: f32) -> Option<DragTarget> {
        if !self.show_architecture { return None; }
        if let Some(index) = self.find_opening(point, ppm) {
            return Some(DragTarget::Opening(index));
        }
        let mut nearest = None;
        let mut distance = 12.0 / ppm;
        for (index, wall) in self.plan.walls.iter().enumerate() {
            for (candidate, endpoint) in [
                (DragTarget::WallStart(index), wall.start),
                (DragTarget::WallEnd(index), wall.end),
            ] {
                let d = point.distance_to(endpoint);
                if d <= distance {
                    nearest = Some(candidate);
                    distance = d;
                }
            }
        }
        nearest.or_else(|| self.find_wall(point, ppm).map(DragTarget::WallBody))
    }

    fn snap_other_endpoint(&self, raw: Point, moving_wall: usize, ppm: f32) -> Point {
        if self.snap_endpoints {
            if let Some(point) = self.plan.walls.iter().enumerate()
                .filter(|(index, _)| *index != moving_wall)
                .flat_map(|(_, wall)| [wall.start, wall.end])
                .filter(|point| raw.distance_to(*point) <= 13.0 / ppm)
                .min_by(|a, b| raw.distance_to(*a).total_cmp(&raw.distance_to(*b)))
            {
                return point;
            }
        }
        raw.snapped(self.snap_m)
    }

    fn begin_drag(&mut self, point: Point, ppm: f32) {
        let Some(target) = self.drag_target_at(point, ppm) else { return; };
        self.clear_selection();
        match target {
            DragTarget::Opening(index) => {
                self.selected_opening = Some(index);
                self.status = "Glisse la porte ou la fenetre le long du mur".into();
            }
            DragTarget::WallStart(index) | DragTarget::WallEnd(index) => {
                self.selected_wall = Some(index);
                self.status = "Deplace la poignee du mur (accrochage disponible)".into();
            }
            DragTarget::WallBody(index) => {
                self.selected_wall = Some(index);
                self.status = "Deplace le mur entier avec ses ouvertures".into();
            }
        }
        self.drag = Some(ActiveDrag {
            target,
            from: point,
            original: self.plan.clone(),
            changed: false,
        });
    }

    fn update_drag(&mut self, cursor: Point, ppm: f32) {
        let Some(drag) = self.drag.as_ref() else { return; };
        let from = drag.from;
        let target = drag.target;
        let delta = Point { x: cursor.x - from.x, y: cursor.y - from.y };
        let mut updated = drag.original.clone();

        let outcome = match target {
            DragTarget::Opening(index) => {
                if let Some(opening) = updated.openings.get(index) {
                    if let Some(wall) = updated.walls.get(opening.wall_index) {
                        let along = wall.projected_distance_m(cursor) - wall.projected_distance_m(from);
                        let center = ((opening.center_m + along) / self.snap_m).round() * self.snap_m;
                        let min_center = opening.width_m / 2.0 + 0.05;
                        let max_center = wall.length_m() - min_center;
                        if min_center <= max_center {
                            updated.move_opening(index, center.clamp(min_center, max_center))
                        } else {
                            Err("Ce mur est trop court pour l'ouverture")
                        }
                    } else { Err("Mur introuvable") }
                } else { Err("Ouverture introuvable") }
            }
            DragTarget::WallBody(index) => {
                if let Some(wall) = updated.walls.get(index).cloned() {
                    let d = Point {
                        x: (delta.x / self.snap_m).round() * self.snap_m,
                        y: (delta.y / self.snap_m).round() * self.snap_m,
                    };
                    updated.update_wall(
                        index,
                        Point { x: wall.start.x + d.x, y: wall.start.y + d.y },
                        Point { x: wall.end.x + d.x, y: wall.end.y + d.y },
                        wall.thickness_m,
                    )
                } else { Err("Mur introuvable") }
            }
            DragTarget::WallStart(index) | DragTarget::WallEnd(index) => {
                if let Some(wall) = updated.walls.get(index).cloned() {
                    let original_endpoint = match target {
                        DragTarget::WallStart(_) => wall.start,
                        _ => wall.end,
                    };
                    let candidate = Point {
                        x: original_endpoint.x + delta.x,
                        y: original_endpoint.y + delta.y,
                    };
                    let snapped = self.snap_other_endpoint(candidate, index, ppm);
                    let (start, end) = match target {
                        DragTarget::WallStart(_) => (snapped, wall.end),
                        _ => (wall.start, snapped),
                    };
                    updated.update_wall(index, start, end, wall.thickness_m)
                } else { Err("Mur introuvable") }
            }
        };

        match outcome {
            Ok(()) => {
                let moved = match target {
                    DragTarget::Opening(index) => {
                        let old = &drag.original.openings[index];
                        let now = &updated.openings[index];
                        (old.center_m - now.center_m).abs() > 0.0001
                    }
                    DragTarget::WallBody(index)
                    | DragTarget::WallStart(index)
                    | DragTarget::WallEnd(index) => {
                        let old = &drag.original.walls[index];
                        let now = &updated.walls[index];
                        old.start.distance_to(now.start) > 0.0001
                            || old.end.distance_to(now.end) > 0.0001
                    }
                };
                // Toujours repartir de l'etat d'origine pour eviter l'accumulation d'erreurs
                // de grille pendant un glisser-deposer.
                self.plan = updated;
                if let Some(active) = self.drag.as_mut() {
                    active.changed = moved;
                }
            }
            Err(error) => self.status = format!("Deplacement refuse : {error}"),
        }
    }

    fn finish_drag(&mut self) {
        if let Some(session) = self.drag.take() {
            if session.changed {
                self.undo.push(session.original);
                if self.undo.len() > 100 { self.undo.remove(0); }
                self.redo.clear();
                self.status = "Deplacement termine : derniere position valide conservee (Ctrl+Z pour annuler)".into();
            }
        }
    }

    fn select_at(&mut self, point: Point, ppm: f32) {
        self.clear_selection();
        if self.show_electrical {
            self.selected_symbol = self.find_symbol_any(point, ppm);
            if self.selected_symbol.is_some() {
                self.status = "Symbole electrique selectionne".into();
                return;
            }
        }
        if self.show_architecture {
            self.selected_opening = self.find_opening(point, ppm);
            if self.selected_opening.is_some() {
                self.status = "Porte ou fenetre selectionnee : glisse-la pour la deplacer".into();
                return;
            }
            self.selected_stair = self.find_stair(point, ppm);
            if self.selected_stair.is_some() {
                self.status = "Escalier selectionne".into();
                return;
            }
            self.selected_wall = self.find_wall(point, ppm);
            if let Some(index) = self.selected_wall {
                self.status = format!("Mur n°{} selectionne", index + 1);
                return;
            }
            if let Some((index, _)) = rooms::detect_rooms(&self.plan.walls).iter().enumerate()
                .find(|(_, room)| room.contains(point))
            {
                self.selected_room = Some(point);
                self.status = format!("Piece {} selectionnee", index + 1);
                return;
            }
        }
        self.status = "Aucun element a cet endroit".into();
    }

    fn delete_selected(&mut self) {
        self.cancel_drag();
        let mut updated = self.plan.clone();
        let outcome = if let Some(index) = self.selected_opening {
            updated.remove_opening(index)
        } else if let Some(index) = self.selected_symbol {
            updated.remove_symbol(index)
        } else if let Some(index) = self.selected_stair {
            updated.remove_stair(index)
        } else if let Some(index) = self.selected_wall {
            updated.remove_wall(index)
        } else {
            self.status = "Selectionne d'abord un element a supprimer".into();
            return;
        };
        match outcome {
            Ok(()) => {
                self.snapshot();
                self.plan = updated;
                self.clear_selection();
                self.reset_pending();
                self.status = "Element supprime (Annuler permet de le recuperer)".into();
            }
            Err(error) => self.status = error.into(),
        }
    }

    /// Panneau d'edition numerique des objets selectionnes.
    fn selected_properties(&mut self, ui: &mut egui::Ui) {
        if let Some(index) = self.selected_opening {
            if let Some(opening) = self.plan.openings.get(index).cloned() {
                ui.heading(match opening.kind {
                    OpeningKind::Door => "Porte",
                    OpeningKind::Window => "Fenetre",
                });
                ui.small("Glisse le symbole sur le mur pour le repositionner.");
                let mut center = opening.center_m;
                let mut width_cm = opening.width_m * 100.0;
                let mut hinge_right = opening.hinge_right;
                let mut swing_flipped = opening.swing_flipped;
                let mut changed = false;
                ui.label("Centre depuis le debut du mur");
                changed |= ui.add(egui::DragValue::new(&mut center)
                    .speed(0.05).suffix(" m")).changed();
                ui.label("Largeur de l'ouverture");
                let limits = match opening.kind {
                    OpeningKind::Door => 50.0..=200.0,
                    OpeningKind::Window => 40.0..=400.0,
                };
                changed |= ui.add(egui::DragValue::new(&mut width_cm)
                    .range(limits).speed(1.0).suffix(" cm")).changed();
                if opening.kind == OpeningKind::Door {
                    changed |= ui.checkbox(&mut hinge_right, "Charniere a la fin du mur").changed();
                    changed |= ui.checkbox(&mut swing_flipped, "Inverser l'ouverture").changed();
                }
                if changed {
                    let mut updated = self.plan.clone();
                    match updated.update_opening(index, center, width_cm / 100.0) {
                        Ok(()) => {
                            if let Some(element) = updated.openings.get_mut(index) {
                                element.hinge_right = hinge_right;
                                element.swing_flipped = swing_flipped;
                            }
                            self.snapshot();
                            self.plan = updated;
                            self.status = "Ouverture modifiee".into();
                        }
                        Err(error) => self.status = error.into(),
                    }
                }
                if ui.button("Supprimer cette ouverture").clicked() {
                    self.delete_selected();
                }
                return;
            }
        }
        if let Some(index) = self.selected_wall {
            if let Some(mut edited) = self.plan.walls.get(index).cloned() {
                ui.heading(format!("Mur {}", index + 1));
                let mut changed = false;
                ui.label("Extremite de depart (metres)");
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut edited.start.x).speed(0.05).prefix("X: ").suffix(" m")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut edited.start.y).speed(0.05).prefix("Y: ").suffix(" m")).changed();
                });
                ui.label("Extremite d'arrivee (metres)");
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut edited.end.x).speed(0.05).prefix("X: ").suffix(" m")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut edited.end.y).speed(0.05).prefix("Y: ").suffix(" m")).changed();
                });

                let mut length = edited.length_m();
                if ui.add(egui::DragValue::new(&mut length).speed(0.05).range(0.05..=500.0)
                    .prefix("Longueur: ").suffix(" m")).changed() {
                    let current = edited.length_m();
                    let dx = if current > 0.0001 { (edited.end.x - edited.start.x) / current } else { 1.0 };
                    let dy = if current > 0.0001 { (edited.end.y - edited.start.y) / current } else { 0.0 };
                    edited.end = Point { x: edited.start.x + dx * length, y: edited.start.y + dy * length };
                    changed = true;
                }

                let mut thickness_cm = edited.thickness_m * 100.0;
                if ui.add(egui::DragValue::new(&mut thickness_cm).speed(1.0)
                    .range(5.0..=60.0).prefix("Epaisseur: ").suffix(" cm")).changed() {
                    edited.thickness_m = thickness_cm / 100.0;
                    changed = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("Horizontal").clicked() {
                        edited.end.y = edited.start.y;
                        changed = true;
                    }
                    if ui.button("Vertical").clicked() {
                        edited.end.x = edited.start.x;
                        changed = true;
                    }
                });

                if changed {
                    let mut updated = self.plan.clone();
                    match updated.update_wall(index, edited.start, edited.end, edited.thickness_m) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.status = "Mur modifie (portes et fenetres conservees)".into();
                        }
                        Err(error) => self.status = error.into(),
                    }
                }
                ui.small("Les ouvertures restent a leur distance depuis le debut du mur.");
                if ui.button("Supprimer ce mur").clicked() {
                    self.delete_selected();
                }
                return;
            }
        }

        if let Some(index) = self.selected_symbol {
            if let Some(symbol) = self.plan.symbols.get(index).cloned() {
                let kind = match symbol.kind {
                    ElectricalKind::Outlet => "Prise",
                    ElectricalKind::Switch => "Interrupteur",
                    ElectricalKind::Light => "Point lumineux",
                };
                ui.heading(kind);
                if symbol.kind == ElectricalKind::Outlet {
                    let ref_text = self.plan.point_ref(index)
                        .unwrap_or_else(|| "sans repere".into());
                    ui.label(format!("Repere sur les documents : {ref_text}"));
                    ui.label(format!(
                        "Circuit actif : {}",
                        self.plan.circuit(self.active_circuit_index)
                            .map(|c| c.letter.as_str()).unwrap_or("?")
                    ));
                    if ui.button("Affecter cette prise au circuit actif").clicked() {
                        let mut updated = self.plan.clone();
                        match updated.assign_outlet_to_circuit(index, self.active_circuit_index) {
                            Ok(()) => {
                                self.snapshot();
                                self.plan = updated;
                                self.status = "Prise affectee au circuit actif".into();
                            }
                            Err(err) => self.status = err.into(),
                        }
                    }
                    if ui.button("Retirer l'affectation de cette prise").clicked() {
                        self.snapshot();
                        self.plan.unassign_outlet(index);
                        self.status = "Affectation retiree : la prise sera signalee comme incomplete".into();
                    }
                }
                let mut position = symbol.position;
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut position.x)
                        .speed(0.05).prefix("X: ").suffix(" m")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut position.y)
                        .speed(0.05).prefix("Y: ").suffix(" m")).changed();
                });
                if changed && position.x.is_finite() && position.y.is_finite() {
                    self.snapshot();
                    self.plan.symbols[index].position = position;
                    self.status = "Position de l'element electrique modifiee".into();
                }
                if ui.button("Supprimer cet appareil").clicked() {
                    self.delete_selected();
                }
                ui.small("La suppression nettoie ses liaisons unifilaires.");
                return;
            }
        }

        if let Some(index) = self.selected_stair {
            if let Some(mut stair) = self.plan.stairs.get(index).cloned() {
                ui.heading("Escalier");
                let mut changed = false;
                ui.label("Depart");
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut stair.start.x).speed(0.05).prefix("X: ").suffix(" m")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut stair.start.y).speed(0.05).prefix("Y: ").suffix(" m")).changed();
                });
                ui.label("Arrivee");
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut stair.end.x).speed(0.05).prefix("X: ").suffix(" m")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut stair.end.y).speed(0.05).prefix("Y: ").suffix(" m")).changed();
                });
                let mut width_cm = stair.width_m * 100.0;
                if ui.add(egui::DragValue::new(&mut width_cm).range(60.0..=250.0)
                    .suffix(" cm").prefix("Largeur: ")).changed() {
                    stair.width_m = width_cm / 100.0;
                    changed = true;
                }
                changed |= ui.add(egui::Slider::new(&mut stair.steps, 3..=30)
                    .text("Marches")).changed();
                if changed {
                    if stair.is_valid() && stair.start.x.is_finite() && stair.end.x.is_finite()
                        && stair.start.y.is_finite() && stair.end.y.is_finite() {
                        self.snapshot();
                        self.plan.stairs[index] = stair;
                        self.status = "Escalier modifie".into();
                    } else {
                        self.status = "Dimensions d'escalier invalides".into();
                    }
                }
                if ui.button("Supprimer cet escalier").clicked() {
                    self.delete_selected();
                }
                return;
            }
        }

        if let Some(at) = self.selected_room {
            if let Some(room) = rooms::detect_rooms(&self.plan.walls).iter()
                .find(|room| room.contains(at))
            {
                ui.heading("Piece fermee");
                ui.label(format!("Surface interieure estimee : {:.2} m²", room.area_m2));
                if room.rectangular {
                    ui.label(format!("Dimensions interieures : {:.2} x {:.2} m", room.width_m, room.height_m));
                } else {
                    ui.label(format!("Emprise X/Y : {:.2} x {:.2} m", room.width_m, room.height_m));
                    ui.small("Pour une piece non rectangulaire, ces deux mesures correspondent a son emprise.");
                }
                let mut name = self.plan.room_name(room).to_owned();
                ui.label("Nom de la piece");
                if ui.text_edit_singleline(&mut name).changed() {
                    let mut updated = self.plan.clone();
                    match updated.set_room_name(at, &name) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.status = "Nom de la piece mis a jour".into();
                        },
                        Err(err) => self.status = err.into(),
                    }
                }
                ui.small("Les surfaces prennent en compte l'epaisseur des murs.");
                return;
            }
        }
        ui.small("Selectionne un mur, un escalier, un appareil ou l'interieur d'une piece.");
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
        let mut updated = self.plan.clone();
        match updated.add_demo_lighting_in_circuit(self.active_circuit_index) {
            Ok(()) => {
                self.snapshot();
                self.plan = updated;
                self.pending_switch = None;
                self.view = View::Plan2D;
                self.status = format!(
                    "Exemple ajoute au circuit {}. Ouvre le schema unifilaire.",
                    self.plan.circuit(self.active_circuit_index)
                        .map(|c| c.letter.as_str()).unwrap_or("?")
                );
            }
            Err(error) => self.status = error.into(),
        }
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
                        match updated.connect_switch_to_light_in_circuit(
                            switch_index, light_index, self.active_circuit_index
                        ) {
                            Ok(()) => {
                                self.snapshot();
                                self.plan = updated;
                                self.pending_switch = None;
                                self.status = format!(
                                    "Liaison ajoutee au circuit {} : visible dans l'unifilaire.",
                                    self.plan.circuit(self.active_circuit_index)
                                        .map(|c| c.letter.as_str()).unwrap_or("?")
                                );
                            }
                            Err(error) => self.status = error.into(),
                        }
                    } else {
                        self.status = "Clique sur une lampe (X) pour terminer la liaison.".into();
                    }
                } else if let Some(index) = self.find_electrical_symbol(position, ppm, ElectricalKind::Switch) {
                    self.pending_switch = Some(index);
                    self.status = format!(
                        "Interrupteur selectionne pour le circuit {}. Clique sur la lampe.",
                        self.plan.circuit(self.active_circuit_index)
                            .map(|c| c.letter.as_str()).unwrap_or("?")
                    );
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
                let new_index = self.plan.symbols.len();
                self.plan.symbols.push(ElectricalSymbol { position, kind });
                if kind == ElectricalKind::Outlet {
                    match self.plan.assign_outlet_to_circuit(new_index, self.active_circuit_index) {
                        Ok(()) => self.status = "Prise ajoutee au circuit actif".into(),
                        Err(err) => self.status = err.into(),
                    }
                } else {
                    self.status = "Symbole ajoute".into();
                }
            }
            Tool::SelectWall => self.select_at(position, ppm),
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

        if response.hovered() {
            let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                if let Some(cursor) = response.hover_pos() {
                    let old_zoom = self.zoom;
                    let new_zoom = (old_zoom * (scroll * 0.002).exp()).clamp(0.3, 4.0);
                    // Garder le point sous la souris fixe durant le zoom.
                    let center = rect.center();
                    let old_origin = center + self.pan;
                    self.pan = cursor - center - (cursor - old_origin) * (new_zoom / old_zoom);
                    self.zoom = new_zoom;
                }
            }
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

        if self.tool == Tool::SelectWall {
            if response.drag_started_by(egui::PointerButton::Primary) {
                let initial = ui.ctx().input(|i| i.pointer.press_origin());
                if let Some(pos) = initial.or(response.interact_pointer_pos()) {
                    self.begin_drag(to_world(pos), ppm);
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(cursor) = response.interact_pointer_pos() {
                    self.update_drag(to_world(cursor), ppm);
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                // Appliquer egalement la position de la souris au moment du relachement.
                if let Some(cursor) = response.interact_pointer_pos() {
                    self.update_drag(to_world(cursor), ppm);
                }
                self.finish_drag();
            }
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let world = to_world(pos);
                let point = match self.tool {
                    Tool::SelectWall | Tool::Door | Tool::Window | Tool::LinkLighting => world,
                    Tool::Wall => self.snap_point(world, ppm),
                    _ => world.snapped(self.snap_m),
                };
                self.place(point, ppm);
            }
        }

        if self.show_architecture {
        if self.show_rooms {
            let detected_rooms = rooms::detect_rooms(&self.plan.walls);
            for room in &detected_rooms {
                let points: Vec<Pos2> = room.inner_polygon.iter().map(|&p| to_screen(p)).collect();
                let selected = self.selected_room.is_some_and(|at| room.contains(at));
                if points.len() >= 3 {
                    painter.add(egui::Shape::closed_line(
                        points,
                        Stroke::new(
                            if selected { 2.8 } else { 1.0 },
                            if selected {
                                Color32::from_rgb(38, 127, 204)
                            } else {
                                Color32::from_rgb(174, 196, 211)
                            },
                        ),
                    ));
                }
                let pos = to_screen(room.center);
                let name = self.plan.room_name(room);
                painter.text(
                    pos + Vec2::new(0.0, -18.0),
                    Align2::CENTER_CENTER,
                    if name.is_empty() { "Piece" } else { name },
                    FontId::proportional(15.0),
                    Color32::from_rgb(36, 89, 135),
                );
                painter.text(
                    pos,
                    Align2::CENTER_CENTER,
                    format!("{:.2} m²", room.area_m2),
                    FontId::proportional(13.0),
                    Color32::from_rgb(36, 89, 135),
                );
                painter.text(
                    pos + Vec2::new(0.0, 16.0),
                    Align2::CENTER_CENTER,
                    if room.rectangular {
                        format!("{:.2} × {:.2} m", room.width_m, room.height_m)
                    } else {
                        "Dimensions d'emprise".to_owned()
                    },
                    FontId::proportional(11.0),
                    Color32::from_rgb(75, 111, 141),
                );
            }
        }
        for (index, wall) in self.plan.walls.iter().enumerate() {
            let a = to_screen(wall.start);
            let b = to_screen(wall.end);
            let width_px = wall.thickness_m.max(0.01) * ppm;
            let screen_length = (b - a).length();
            if screen_length > 0.001 {
                let direction = (b - a) / screen_length;
                let mut openings: Vec<(usize, &Opening)> = self.plan.openings.iter()
                    .enumerate()
                    .filter(|(_, o)| o.wall_index == index)
                    .collect();
                openings.sort_by(|a, b| a.1.start_m().total_cmp(&b.1.start_m()));
                let mut cursor_m = 0.0_f32;
                for (opening_index, opening) in openings {
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
                    if self.selected_opening == Some(opening_index) {
                        let handle = a + direction * (opening.center_m * ppm);
                        painter.circle_filled(handle, 5.0, Color32::from_rgb(242, 178, 73));
                        painter.circle_stroke(
                            handle, 8.0,
                            Stroke::new(2.0, Color32::from_rgb(40, 135, 230)),
                        );
                    }
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
            if self.selected_wall == Some(index) {
                for endpoint in [a, b] {
                    painter.circle_filled(endpoint, 6.0, Color32::WHITE);
                    painter.circle_stroke(
                        endpoint, 6.0,
                        Stroke::new(2.5, Color32::from_rgb(40, 135, 230)),
                    );
                }
            }
        }

        for (index, stair) in self.plan.stairs.iter().enumerate() {
            draw_stair(&painter, stair, origin, ppm);
            if self.selected_stair == Some(index) && stair.is_valid() {
                let corners = stair.corners().map(to_screen);
                for j in 0..4 {
                    painter.line_segment(
                        [corners[j], corners[(j + 1) % 4]],
                        Stroke::new(3.5, Color32::from_rgb(40, 135, 230))
                    );
                }
            }
        }
        } // fin du calque architectural

        if self.show_electrical {
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
            let ref_suffix = match symbol.kind {
                ElectricalKind::Outlet | ElectricalKind::Light =>
                    self.plan.point_ref(index).map(|name| format!(" ({name})")).unwrap_or_default(),
                ElectricalKind::Switch => {
                    let refs = self.plan.switch_point_refs(index);
                    if refs.is_empty() { String::new() }
                    else { format!(" ({})", refs.join(",")) }
                }
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
            if self.selected_symbol == Some(index) {
                painter.circle_stroke(pos, 19.0, Stroke::new(3.0, Color32::from_rgb(40, 135, 230)));
            }
        }
        } // fin du calque electrique

        if self.show_architecture {
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
                let end = self.snap_point(to_world(cursor), ppm);
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

        } // fin des apercus architecturaux

        if let Some(pos) = response.hover_pos() {
            let point = to_world(pos);
            painter.text(
                rect.right_top() + Vec2::new(-12.0, 12.0),
                Align2::RIGHT_TOP,
                format!("X: {:.2} m   Y: {:.2} m", point.x, point.y),
                FontId::proportional(13.0),
                Color32::DARK_GRAY,
            );
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
        if !ctx.wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)) {
                self.redo();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.cancel_drag();
                self.reset_pending();
                self.clear_selection();
                self.status = "Selection et trace en cours annules".into();
            }
            if self.view == View::Plan2D &&
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
                self.delete_selected();
            }
        }
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
                if ui.selectable_label(self.view == View::RgieReview, "Dossier RGIE (verification)").clicked() {
                    self.view = View::RgieReview;
                }
                ui.separator();
                if ui.button("Ajouter demo I -> L").clicked() { self.add_lighting_demo(); }
                ui.separator();
                if ui.button("Nouveau").clicked() {
                    self.cancel_drag();
                    self.snapshot();
                    self.plan = Plan::default();
                    self.reset_pending();
                    self.clear_selection();
                    self.active_circuit_index = 0;
                    self.view = View::Plan2D;
                    self.status = "Nouveau plan".into();
                }
                if ui.button("Annuler").clicked() { self.undo(); }
                if ui.button("Retablir").clicked() { self.redo(); }
                if self.view == View::Plan2D && ui.button("Supprimer selection").clicked() {
                    self.delete_selected();
                }
                if ui.button("Enregistrer").clicked() { self.save(); }
                if ui.button("Ouvrir").clicked() { self.load(); }
                if ui.button("Exporter PDF plan").clicked() { self.export_pdf(); }
                if self.view == View::Unifilar && ui.button("Exporter PDF unifilaire").clicked() {
                    self.export_unifilar_pdf();
                }
            });
        });

        egui::SidePanel::left("tools").resizable(true).default_width(275.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Outils");
            ui.separator();
            if ui.selectable_label(self.tool == Tool::Wall, "Mur").clicked() { self.select_tool(Tool::Wall); }
            if ui.selectable_label(self.tool == Tool::SelectWall, "Selectionner un element").clicked() {
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
            ui.small("Selection : clic sur un element. Glisse un mur, ses poignees bleues ou une ouverture.");
            ui.small("Deplacer une ouverture : elle reste sur son mur et ne peut pas chevaucher une autre.");
            ui.small("Ctrl+Z : annuler | Ctrl+Y : retablir.");
            ui.separator();
            if self.view == View::Plan2D && self.tool == Tool::Door {
                ui.label("Largeur de la porte");
                ui.add(egui::Slider::new(&mut self.new_door_width_cm, 50.0..=200.0)
                    .step_by(5.0).suffix(" cm"));
                ui.checkbox(&mut self.door_hinge_right, "Charniere a la fin du mur");
                ui.checkbox(&mut self.door_swing_flipped, "Inverser le sens d'ouverture");
                ui.small("Clique sur un mur pour inserer la porte.");
                ui.separator();
            }
            if self.view == View::Plan2D && self.tool == Tool::Stair {
                ui.label("Largeur de l'escalier");
                ui.add(egui::Slider::new(&mut self.new_stair_width_cm, 60.0..=250.0)
                    .step_by(5.0).suffix(" cm"));
                ui.label("Nombre de marches");
                ui.add(egui::Slider::new(&mut self.new_stair_steps, 3..=30));
                ui.small("Deux clics : depart puis arrivee (sens de montee).");
                ui.separator();
            }
            if self.view == View::Plan2D && self.tool == Tool::Window {
                ui.label("Largeur de la fenetre");
                ui.add(egui::Slider::new(&mut self.new_window_width_cm, 40.0..=400.0)
                    .step_by(5.0).suffix(" cm"));
                ui.small("Clique sur un mur pour inserer la fenetre.");
                ui.separator();
            }
            ui.label("Epaisseur des nouveaux murs");
            ui.add(egui::Slider::new(&mut self.new_wall_thickness_cm, 5.0..=60.0).text("cm").step_by(1.0));
            ui.separator();
            egui::CollapsingHeader::new("Proprietes de la selection").default_open(true).show(ui, |ui| {
                self.selected_properties(ui);
            });
            ui.separator();
            ui.label("Calques visibles");
            ui.checkbox(&mut self.show_architecture, "Architecture (murs, portes, escaliers)");
            ui.checkbox(&mut self.show_rooms, "Pieces, surfaces et dimensions");
            ui.checkbox(&mut self.show_electrical, "Electricite (symboles et liaisons)");
            ui.checkbox(&mut self.show_dimensions, "Afficher les cotes (axe des murs)");
            ui.checkbox(&mut self.snap_endpoints, "Aimantation aux extremites de murs");
            ui.label("Pas d'aimantation");
            ui.add(egui::Slider::new(&mut self.snap_m, 0.05..=0.5).step_by(0.05).suffix(" m"));
            ui.separator();
            ui.label("Zoom");
            ui.add(egui::Slider::new(&mut self.zoom, 0.3..=4.0));
            ui.small("Molette sur le plan : zoom centre sous le pointeur.");
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
            ui.label(format!("Pieces fermees : {}", rooms::detect_rooms(&self.plan.walls).len()));
            if ui.button("Ouvrir l'exemple de 2 pieces").clicked() {
                self.load_room_example();
            }
            ui.small("L'exemple remplace le plan actuel ; Annuler permet de revenir.");
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
            egui::CollapsingHeader::new("Identification du dossier RGIE")
                .default_open(self.view == View::RgieReview)
                .show(ui, |ui| {
                    let current = self.plan.installation_info.clone();
                    let mut info = current.clone();
                    ui.label("Adresse de l'installation");
                    ui.text_edit_singleline(&mut info.address);
                    ui.label("Responsable execution - nom");
                    ui.text_edit_singleline(&mut info.executor_name);
                    ui.label("Qualite du responsable");
                    ui.text_edit_singleline(&mut info.executor_role);
                    ui.label("N° TVA (si applicable)");
                    ui.text_edit_singleline(&mut info.executor_vat);
                    ui.label("Tension nominale (sans valeur supposee)");
                    ui.text_edit_singleline(&mut info.voltage);
                    ui.label("Nature du courant (AC/DC et configuration)");
                    ui.text_edit_singleline(&mut info.current_kind);
                    ui.label("Description alimentation");
                    ui.text_edit_singleline(&mut info.supply);
                    ui.label("Nom ou repere du tableau");
                    ui.text_edit_singleline(&mut info.panel_name);
                    ui.label("Date du dessin");
                    ui.text_edit_singleline(&mut info.drawing_date);
                    ui.label("Revision du document");
                    ui.text_edit_singleline(&mut info.revision);
                    if info != current {
                        self.snapshot();
                        self.plan.installation_info = info;
                        self.status = "Identification du dossier mise a jour".into();
                    }
                    ui.small("Ces champs servent au dossier, sans validation automatique RGIE.");
                });
            ui.separator();
            ui.heading("Circuits et protections");
            if ui.button("Charger l'exemple unifilaire A + B").clicked() {
                self.load_unifilar_example();
            }
            ui.small("L'exemple remplace le plan courant (Annuler pour revenir).");
            if self.active_circuit_index >= self.plan.circuit_count() {
                self.active_circuit_index = 0;
            }
            egui::ComboBox::from_label("Circuit actif pour Lier I -> L")
                .selected_text(
                    self.plan.circuit(self.active_circuit_index)
                        .map(|c| c.letter.clone()).unwrap_or_else(|| "A".into())
                )
                .show_ui(ui, |ui| {
                    for index in 0..self.plan.circuit_count() {
                        let title = format!(
                            "Circuit {}",
                            self.plan.circuit(index).map(|c| c.letter.as_str()).unwrap_or("?"),
                        );
                        ui.selectable_value(&mut self.active_circuit_index, index, title);
                    }
                });
            if ui.button("Ajouter un circuit d'eclairage").clicked() {
                let mut updated = self.plan.clone();
                match updated.add_lighting_circuit() {
                    Ok(index) => {
                        self.snapshot();
                        self.plan = updated;
                        self.active_circuit_index = index;
                        self.pending_switch = None;
                        self.status = "Nouveau circuit ajoute et selectionne".into();
                    }
                    Err(error) => self.status = error.into(),
                }
            }
            ui.small("Les prises et liaisons I -> L sont affectees au circuit actif.");
            ui.label(format!("Prises sans circuit : {}", self.plan.unassigned_outlets()));
            for circuit_index in 0..self.plan.circuit_count() {
                let Some(circuit_data) = self.plan.circuit(circuit_index).cloned() else { continue; };
                egui::CollapsingHeader::new(format!(
                    "Circuit {} - protection / conducteurs", circuit_data.letter
                )).default_open(circuit_index == self.active_circuit_index).show(ui, |ui| {
                    let mut circuit = circuit_data.clone();
                    ui.label("Repere du circuit (A, B, C...)");
                    ui.text_edit_singleline(&mut circuit.letter);
                    ui.label("Type de protection (disjoncteur, fusible...)");
                    ui.text_edit_singleline(&mut circuit.protection_kind);
                    ui.label("Protection contre surintensites - calibre");
                    ui.add(egui::DragValue::new(&mut circuit.breaker_amp).range(0..=125).suffix(" A"));
                    ui.label("Nombre de poles");
                    ui.add(egui::DragValue::new(&mut circuit.breaker_poles).range(0..=4));
                    ui.label("Courbe / type protection");
                    ui.text_edit_singleline(&mut circuit.breaker_curve);
                    ui.label("Differentiel en amont (type, A, mA)");
                    ui.text_edit_singleline(&mut circuit.upstream_rdc);
                    ui.label("Type de canalisation");
                    ui.text_edit_singleline(&mut circuit.cable_type);
                    ui.label("Nombre de conducteurs");
                    ui.add(egui::DragValue::new(&mut circuit.conductor_count).range(0..=10));
                    ui.label("Section des conducteurs");
                    ui.add(egui::DragValue::new(&mut circuit.section_mm2).speed(0.1)
                        .range(0.0..=35.0).suffix(" mm²"));
                    ui.label("Mode de pose");
                    ui.text_edit_singleline(&mut circuit.installation_method);
                    if circuit != circuit_data {
                        let mut updated = self.plan.clone();
                        match updated.update_lighting_circuit(circuit_index, circuit) {
                            Ok(()) => {
                                self.snapshot();
                                self.plan = updated;
                                self.status = "Caracteristiques du circuit mises a jour".into();
                            }
                            Err(error) => self.status = error.into(),
                        }
                    }
                    let missing = self.plan.circuit(circuit_index)
                        .map(|c| c.missing_fields()).unwrap_or_default();
                    if !missing.is_empty() {
                        ui.colored_label(
                            Color32::from_rgb(176, 91, 13),
                            format!("{} informations manquantes", missing.len()),
                        );
                    } else {
                        ui.small("Champs remplis : conformite non verifiee.");
                    }
                    if circuit_index > 0 && ui.button("Supprimer ce circuit vide").clicked() {
                        let mut updated = self.plan.clone();
                        match updated.remove_lighting_circuit(circuit_index) {
                            Ok(()) => {
                                self.snapshot();
                                self.plan = updated;
                                self.active_circuit_index = 0;
                                self.pending_switch = None;
                                self.status = "Circuit vide supprime".into();
                            }
                            Err(error) => self.status = error.into(),
                        }
                    }
                });
            }
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
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view {
                View::Plan2D => self.canvas(ui),
                View::RgieReview => rgie_review::show(ui, &self.plan),
                View::Unifilar => {
                    if let Some(action) = unifilar::show(ui, &self.plan) {
                        match action {
                            unifilar::Action::RemoveConnection(index) => {
                                self.remove_connection(index);
                            }
                            unifilar::Action::MoveGroup { switch_index, target_circuit } => {
                                let mut updated = self.plan.clone();
                                match updated.move_lighting_group(switch_index, target_circuit) {
                                    Ok(()) => {
                                        self.snapshot();
                                        self.plan = updated;
                                        self.status = "Commande et lampes transferees vers le circuit choisi".into();
                                    }
                                    Err(error) => self.status = error.into(),
                                }
                            }
                        }
                    }
                },
            }
        });
    }
}

#[cfg(test)]
mod plan_editor_tests {
    use super::*;

    #[test]
    fn undo_and_redo_restore_wall_and_new_changes_clear_redo() {
        let mut app = EasyDraft::default();
        app.snapshot();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 3.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.undo();
        assert!(app.plan.walls.is_empty());
        app.redo();
        assert_eq!(app.plan.walls.len(), 1);
        app.undo();
        assert_eq!(app.redo.len(), 1);
        app.snapshot();
        assert!(app.redo.is_empty());
    }

    #[test]
    fn wall_endpoint_snapping_has_priority_over_grid() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.03, y: 0.07 },
            end: Point { x: 2.03, y: 0.07 },
            thickness_m: 0.15,
        });
        let point = app.snap_point(Point { x: 2.08, y: 0.09 }, 80.0);
        assert_eq!(point, Point { x: 2.03, y: 0.07 });
        let grid = app.snap_point(Point { x: 5.08, y: 4.07 }, 80.0);
        assert!((grid.x - 5.10).abs() < 0.0001);
        assert!((grid.y - 4.05).abs() < 0.0001);
    }

    #[test]
    fn dragging_wall_keeps_attached_opening_and_is_one_undo_step() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.plan.add_opening(
            0, Point { x: 2.0, y: 0.0 }, 0.9,
            OpeningKind::Door, false, false, 0.05,
        ).unwrap();
        app.begin_drag(Point { x: 3.2, y: 0.0 }, 80.0);
        assert!(matches!(app.drag.as_ref().map(|d| d.target), Some(DragTarget::WallBody(0))));
        app.update_drag(Point { x: 3.2, y: 1.0 }, 80.0);
        app.update_drag(Point { x: 3.2, y: 1.5 }, 80.0);
        assert_eq!(app.undo.len(), 0);
        app.finish_drag();
        assert_eq!(app.undo.len(), 1);
        assert!((app.plan.walls[0].start.y - 1.5).abs() < 0.001);
        assert_eq!(app.plan.openings[0].center_m, 2.0);
        app.undo();
        assert_eq!(app.plan.walls[0].start.y, 0.0);
        app.redo();
        assert!((app.plan.walls[0].start.y - 1.5).abs() < 0.001);
    }

    #[test]
    fn door_drags_only_along_its_wall_and_stops_before_another_opening() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.plan.add_opening(
            0, Point { x: 1.0, y: 0.0 }, 0.9,
            OpeningKind::Door, false, false, 0.05,
        ).unwrap();
        app.plan.add_opening(
            0, Point { x: 4.0, y: 0.0 }, 1.2,
            OpeningKind::Window, false, false, 0.05,
        ).unwrap();

        app.begin_drag(Point { x: 1.0, y: 0.0 }, 80.0);
        assert!(matches!(app.drag.as_ref().map(|d| d.target), Some(DragTarget::Opening(0))));
        app.update_drag(Point { x: 2.0, y: 0.3 }, 80.0);
        assert!((app.plan.openings[0].center_m - 2.0).abs() < 0.001);
        // La tentative suivante est invalide : l'ouverture ne se superpose pas.
        app.update_drag(Point { x: 3.2, y: 0.0 }, 80.0);
        assert!((app.plan.openings[0].center_m - 2.0).abs() < 0.001);
        app.finish_drag();
        assert_eq!(app.undo.len(), 1);
        app.undo();
        assert!((app.plan.openings[0].center_m - 1.0).abs() < 0.001);
    }

    #[test]
    fn cancel_drag_restores_original_and_does_not_create_undo_step() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.begin_drag(Point { x: 2.0, y: 0.0 }, 80.0);
        app.update_drag(Point { x: 2.0, y: 1.0 }, 80.0);
        assert!(app.plan.walls[0].start.y > 0.5);
        app.cancel_drag();
        assert_eq!(app.plan.walls[0].start.y, 0.0);
        assert!(app.undo.is_empty());
    }

    #[test]
    fn stationary_snap_does_not_add_to_history() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.begin_drag(Point { x: 2.0, y: 0.0 }, 80.0);
        app.update_drag(Point { x: 2.01, y: 0.0 }, 80.0);
        app.finish_drag();
        assert!(app.undo.is_empty());
    }

    #[test]
    fn wall_end_handles_resize_without_changing_other_endpoint() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.begin_drag(Point { x: 4.0, y: 0.0 }, 80.0);
        assert!(matches!(app.drag.as_ref().map(|d| d.target), Some(DragTarget::WallEnd(0))));
        app.update_drag(Point { x: 5.0, y: 0.0 }, 80.0);
        app.finish_drag();
        assert_eq!(app.plan.walls[0].start.x, 0.0);
        assert!((app.plan.walls[0].end.x - 5.0).abs() < 0.001);
    }

    #[test]
    fn removing_electric_symbol_cleans_unifilar_from_editor() {
        let mut app = EasyDraft::default();
        app.plan.add_demo_lighting();
        app.selected_symbol = Some(0);
        app.delete_selected();
        assert_eq!(app.plan.lighting_connections.len(), 0);
        assert_eq!(app.plan.symbols.len(), 1);
        app.undo();
        assert_eq!(app.plan.lighting_connections.len(), 1);
        assert_eq!(app.plan.symbols.len(), 2);
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
