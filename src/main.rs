mod model;
mod pdf_export;
mod unifilar;
mod rooms;
mod rgie_review;
mod rgie_symbols;

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Sense, Stroke, Vec2};
use std::path::Path;
use model::{
    ElectricalAppearance, ElectricalKind, ElectricalSymbol, Opening, OpeningKind, Plan, Point, Stair, Wall,
    BackgroundPlan, WallEndpoint as WallHandle, DEFAULT_WALL_THICKNESS_M,
};

const GRID_METERS: f32 = 0.5;
const PIXELS_PER_METER: f32 = 80.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Plan2D,
    Unifilar,
    RgieReview,
}

/// Onglets de la barre latérale : un seul ensemble de commandes à la fois.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Dessin,
    Inspecteur,
    Affichage,
    Electricite,
    Dossier,
    Fichiers,
}

/// Filtre les appareils et les liaisons affiches sur le plan de situation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum CircuitVisibility {
    All,
    Active,
    None,
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
            Stroke::new(1.6_f32, Color32::from_rgb(31, 135, 192)),
        );
        along += 14.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Wall,
    SelectWall,
    Door,
    Window,
    Passage,
    Stair,
    Outlet,
    Switch,
    Light,
    LinkLighting,
}

/// Orientation d'un mur en cours de placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WallAlignment {
    Horizontal,
    Vertical,
}

/// Oriente automatiquement le mur lorsque le curseur est proche de 0° ou 90°.
/// Maj force l'orientation cardinale la plus proche, meme pour un angle oblique.
/// Ce calcul est partage entre l'apercu et le second clic de validation.
fn aligned_wall_end(
    start: Point, candidate: Point, force_orthogonal: bool,
) -> (Point, Option<WallAlignment>) {
    let dx = candidate.x - start.x;
    let dy = candidate.y - start.y;
    if dx.abs() + dy.abs() < 0.0001 {
        return (candidate, None);
    }
    // Environ 5 degres de tolerance pour la detection visuelle et l'aimantation.
    let near_horizontal = dy.abs() <= dx.abs() * 0.0875;
    let near_vertical = dx.abs() <= dy.abs() * 0.0875;
    if force_orthogonal || near_horizontal || near_vertical {
        if dx.abs() >= dy.abs() {
            return (
                Point { x: candidate.x, y: start.y },
                Some(WallAlignment::Horizontal),
            );
        }
        return (
            Point { x: start.x, y: candidate.y },
            Some(WallAlignment::Vertical),
        );
    }
    (candidate, None)
}

fn wall_heading(start: Point, end: Point, orientation: Option<WallAlignment>) -> String {
    let length = start.distance_to(end);
    match orientation {
        Some(WallAlignment::Horizontal) => format!("HORIZONTAL  0°   |   {length:.2} m"),
        Some(WallAlignment::Vertical) => format!("VERTICAL  90°   |   {length:.2} m"),
        None => {
            let angle = (end.y - start.y).atan2(end.x - start.x).to_degrees();
            format!("INCLINE  {angle:.1}°   |   {length:.2} m")
        }
    }
}

/// Palette filtrable, chaque bouton active réellement son outil de dessin.
fn palette_section(
    ui: &mut egui::Ui,
    title: &str,
    selected: Tool,
    search: &str,
    tools: &[(Tool, &str)],
) -> Option<Tool> {
    let needle = search.trim().to_lowercase();
    if !needle.is_empty() && !tools.iter().any(|(_, name)|
        name.to_lowercase().contains(&needle) || title.to_lowercase().contains(&needle)
    ) {
        return None;
    }
    let mut chosen = None;
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            for &(tool, name) in tools {
                if needle.is_empty() || name.to_lowercase().contains(&needle)
                    || title.to_lowercase().contains(&needle)
                {
                    if ui.add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new(name).selected(selected == tool)
                    ).clicked() {
                        chosen = Some(tool);
                    }
                }
            }
        });
    chosen
}

#[derive(Clone, Copy, Debug)]
enum DragTarget {
    WallStart(usize),
    WallEnd(usize),
    WallBody(usize),
    Opening(usize),
    Symbol(usize),
    Stair(usize),
    Group,
    /// Plusieurs extremites soudees sur une seule coordonnee.
    Joint,
}

/// Deplacement geometrique atomique conserve pour les tests.
/// Les liens explicites sont crees uniquement par la commande Joindre.
#[cfg(test)]
fn weld_wall_handles(
    plan: &mut Plan, handles: &[WallHandle], target: Point,
) -> Result<(), &'static str> {
    plan.move_wall_endpoints_to(handles, target)
}

/// Selection de plusieurs elements, sans selectionner les surfaces de piece derivees.
#[derive(Debug, Default, Clone)]
struct SelectionSet {
    walls: Vec<usize>,
    openings: Vec<usize>,
    stairs: Vec<usize>,
    symbols: Vec<usize>,
}

impl SelectionSet {
    fn len(&self) -> usize {
        self.walls.len() + self.openings.len() + self.stairs.len() + self.symbols.len()
    }

    fn includes(&self, target: DragTarget) -> bool {
        match target {
            DragTarget::WallStart(i) | DragTarget::WallEnd(i) | DragTarget::WallBody(i) =>
                self.walls.contains(&i),
            DragTarget::Opening(i) => self.openings.contains(&i),
            DragTarget::Stair(i) => self.stairs.contains(&i),
            DragTarget::Symbol(i) => self.symbols.contains(&i),
            DragTarget::Group | DragTarget::Joint => false,
        }
    }

    fn from_marquee(plan: &Plan, area: Marquee) -> Self {
        let mut selected = Self::default();
        for (i, wall) in plan.walls.iter().enumerate() {
            if area.selects_segment(wall.start, wall.end) {
                selected.walls.push(i);
            }
        }
        for (i, symbol) in plan.symbols.iter().enumerate() {
            if area.contains(symbol.position) { selected.symbols.push(i); }
        }
        for (i, stair) in plan.stairs.iter().enumerate() {
            if area.selects_segment(stair.start, stair.end) {
                selected.stairs.push(i);
            }
        }
        for (i, opening) in plan.openings.iter().enumerate() {
            if let Some(wall) = plan.walls.get(opening.wall_index) {
                let length = wall.length_m();
                if length > 0.001 {
                    let t = opening.center_m / length;
                    let center = Point {
                        x: wall.start.x + (wall.end.x - wall.start.x) * t,
                        y: wall.start.y + (wall.end.y - wall.start.y) * t,
                    };
                    if area.contains(center) { selected.openings.push(i); }
                }
            }
        }
        selected
    }
}

#[derive(Clone, Copy, Debug)]
struct Marquee {
    start: Point,
    current: Point,
}

impl Marquee {
    fn bounds(self) -> (f32, f32, f32, f32) {
        (self.start.x.min(self.current.x), self.start.x.max(self.current.x),
         self.start.y.min(self.current.y), self.start.y.max(self.current.y))
    }

    fn contains(self, p: Point) -> bool {
        let (x0,x1,y0,y1) = self.bounds();
        p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1
    }

    /// Fenetre de gauche a droite : contient l'objet entier.
    /// De droite a gauche : capture aussi les segments qui traversent la fenetre.
    fn selects_segment(self, a: Point, b: Point) -> bool {
        if self.start.x <= self.current.x {
            return self.contains(a) && self.contains(b);
        }
        if self.contains(a) || self.contains(b) { return true; }
        let (x0,x1,y0,y1) = self.bounds();
        let (dx,dy) = (b.x-a.x,b.y-a.y);
        let mut lo = 0.0_f32;
        let mut hi = 1.0_f32;
        // Intersection d'un segment et d'un rectangle (Liang-Barsky).
        for (p,q) in [(-dx, a.x-x0), (dx,x1-a.x), (-dy,a.y-y0), (dy,y1-a.y)] {
            if p.abs() < 1e-7 {
                if q < 0.0 { return false; }
            } else {
                let t = q/p;
                if p < 0.0 { lo = lo.max(t); }
                else { hi = hi.min(t); }
                if lo > hi { return false; }
            }
        }
        true
    }
}

/// Deplacement atomique de la selection : les ouvertures restent attachees aux murs.
fn translate_selection(plan: &mut Plan, selection: &SelectionSet, shift: Point)
    -> Result<(), &'static str>
{
    for &index in &selection.walls {
        let wall = plan.walls.get(index).cloned().ok_or("Mur selectionne introuvable")?;
        plan.update_wall(
            index,
            Point { x: wall.start.x + shift.x, y: wall.start.y + shift.y },
            Point { x: wall.end.x + shift.x, y: wall.end.y + shift.y },
            wall.thickness_m,
        )?;
    }
    for &index in &selection.symbols {
        let symbol = plan.symbols.get_mut(index).ok_or("Symbole introuvable")?;
        symbol.position.x += shift.x;
        symbol.position.y += shift.y;
    }
    for &index in &selection.stairs {
        let stair = plan.stairs.get_mut(index).ok_or("Escalier introuvable")?;
        stair.start.x += shift.x;
        stair.start.y += shift.y;
        stair.end.x += shift.x;
        stair.end.y += shift.y;
    }
    for &index in &selection.openings {
        let opening = plan.openings.get(index).ok_or("Ouverture introuvable")?.clone();
        if selection.walls.contains(&opening.wall_index) { continue; }
        let wall = plan.walls.get(opening.wall_index).ok_or("Mur introuvable")?;
        let length = wall.length_m();
        if length < 0.001 { return Err("Mur porteur trop court"); }
        let along = (shift.x * (wall.end.x-wall.start.x)
            + shift.y * (wall.end.y-wall.start.y)) / length;
        let edge = if opening.kind == OpeningKind::Passage {0.0} else {0.05};
        let limit = opening.width_m/2.0 + edge;
        if length < 2.0*limit { return Err("Ouverture trop large pour son mur"); }
        plan.move_opening(index, (opening.center_m + along).clamp(limit,length-limit))?;
    }
    Ok(())
}

struct ActiveDrag {
    target: DragTarget,
    /// Position de la souris au debut du glissement (en metres).
    from: Point,
    /// Etat du plan avant le glissement : une seule entree d'annulation par glissement.
    original: Plan,
    changed: bool,
}

const DEFAULT_SHEET_NAME: &str = "Rez-de-chaussée";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PlanSheet {
    name: String,
    plan: Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct ProjectSettings {
    show_underlay: bool,
    show_dimensions: bool,
    show_architecture: bool,
    show_rooms: bool,
    show_electrical: bool,
    circuit_visibility: CircuitVisibility,
    active_circuit_index: usize,
    snap_endpoints: bool,
    snap_m: f32,
    zoom: f32,
    pan: [f32; 2],
    grid: bool,
    new_wall_thickness_cm: f32,
    new_door_width_cm: f32,
    new_window_width_cm: f32,
    new_passage_width_cm: f32,
    new_stair_width_cm: f32,
    new_stair_steps: u16,
    door_hinge_right: bool,
    door_swing_flipped: bool,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            show_underlay: true,
            show_dimensions: true,
            show_architecture: true,
            show_rooms: true,
            show_electrical: true,
            circuit_visibility: CircuitVisibility::All,
            active_circuit_index: 0,
            snap_endpoints: true,
            snap_m: 0.05,
            zoom: 1.0,
            pan: [0.0; 2],
            grid: true,
            new_wall_thickness_cm: DEFAULT_WALL_THICKNESS_M * 100.0,
            new_door_width_cm: 90.0,
            new_window_width_cm: 120.0,
            new_passage_width_cm: 110.0,
            new_stair_width_cm: 100.0,
            new_stair_steps: 12,
            door_hinge_right: false,
            door_swing_flipped: false,
        }
    }
}

impl ProjectSettings {
    fn clamp(&mut self, circuit_count: usize) {
        self.active_circuit_index = self.active_circuit_index.min(circuit_count - 1);
        self.snap_m = self.snap_m.clamp(0.05, 0.5);
        self.zoom = self.zoom.clamp(0.3, 4.0);
        self.new_wall_thickness_cm = self.new_wall_thickness_cm.clamp(5.0, 60.0);
        self.new_door_width_cm = self.new_door_width_cm.clamp(50.0, 200.0);
        self.new_window_width_cm = self.new_window_width_cm.clamp(40.0, 400.0);
        self.new_passage_width_cm = self.new_passage_width_cm.clamp(40.0, 400.0);
        self.new_stair_width_cm = self.new_stair_width_cm.clamp(60.0, 250.0);
        self.new_stair_steps = self.new_stair_steps.clamp(3, 30);
    }
}

struct SheetNameDialog {
    name: String,
    rename: bool,
    focus_input: bool,
}

/// La première feuille conserve le format des anciens fichiers EasyDraft.
#[derive(serde::Serialize, serde::Deserialize)]
struct Project {
    #[serde(flatten)]
    first_plan: Plan,
    #[serde(default = "default_sheet_name")]
    first_sheet_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    additional_sheets: Vec<PlanSheet>,
    #[serde(default)]
    active_sheet: usize,
    #[serde(default)]
    settings: ProjectSettings,
}

fn default_sheet_name() -> String {
    DEFAULT_SHEET_NAME.into()
}

enum HistoryEntry {
    Plan { sheet: usize, plan: Plan },
    Project { project: Project, restore_settings: bool },
    SheetName { sheet: usize, name: String },
}

struct EasyDraft {
    plan: Plan,
    // Le dessin de la feuille active est déplacé dans `plan` pour l'éditeur.
    sheets: Vec<PlanSheet>,
    active_sheet: usize,
    sheet_name_dialog: Option<SheetNameDialog>,
    reveal_circuit: Option<usize>,
    settings: ProjectSettings,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    tool: Tool,
    view: View,
    sidebar_tab: SidebarTab,
    palette_search: String,
    underlay_texture: Option<egui::TextureHandle>,
    underlay_loaded_key: Option<String>,
    underlay_dimensions: [usize; 2],
    pending_switch: Option<usize>,
    pending_wall: Option<Point>,
    pending_stair: Option<Point>,
    selected_wall: Option<usize>,
    selected_symbol: Option<usize>,
    selected_stair: Option<usize>,
    selected_room: Option<Point>,
    selected_opening: Option<usize>,
    drag: Option<ActiveDrag>,
    marquee: Option<Marquee>,
    selection: SelectionSet,
    selected_handles: Vec<WallHandle>,
    marquee_handles: bool,
    file_path: String,
    /// Nouveau projet : pas encore de chemin sauvegarde.
    file_path_assigned: bool,
    pdf_path: String,
    unifilar_pdf_path: String,
    status: String,
}

impl Default for EasyDraft {
    fn default() -> Self {
        Self {
            plan: Plan::default(),
            sheets: vec![PlanSheet { name: default_sheet_name(), plan: Plan::default() }],
            active_sheet: 0,
            sheet_name_dialog: None,
            reveal_circuit: None,
            settings: ProjectSettings::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            tool: Tool::SelectWall,
            view: View::Plan2D,
            sidebar_tab: SidebarTab::Affichage,
            palette_search: String::new(),
            underlay_texture: None,
            underlay_loaded_key: None,
            underlay_dimensions: [1, 1],
            pending_switch: None,
            pending_wall: None,
            pending_stair: None,
            selected_wall: None,
            selected_symbol: None,
            selected_stair: None,
            selected_room: None,
            selected_opening: None,
            drag: None,
            marquee: None,
            selection: SelectionSet::default(),
            selected_handles: Vec::new(),
            marquee_handles: false,
            file_path: "mon-plan.json".into(),
            file_path_assigned: false,
            pdf_path: "plan-easydraft.pdf".into(),
            unifilar_pdf_path: "unifilaire-easydraft.pdf".into(),
            status: "Sélection / Cadre actif. Choisis un outil pour ajouter un élément.".into(),
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
    let stroke = Stroke::new(1.1_f32, Color32::from_rgb(71, 108, 148));
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
        painter.line_segment([p - tick, p + tick], Stroke::new(1.5_f32, stroke.color));
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
    let frame = Stroke::new(1.5_f32, Color32::from_rgb(45, 53, 65));

    // Deux traits perpendiculaires ferment les extremites coupees du mur.
    for pos in [left, right] {
        painter.line_segment(
            [pos - normal * half_thickness, pos + normal * half_thickness],
            frame,
        );
    }

    match opening.kind {
        OpeningKind::Window => {
            let glass = Stroke::new(1.8_f32, Color32::from_rgb(35, 136, 188));
            for offset in [-0.3_f32, 0.0, 0.3] {
                let shift = normal * (width_px * offset);
                painter.line_segment([left + shift, right + shift], glass);
            }
        }
        OpeningKind::Passage => {
            // L'ouverture reste vide : elle definit toutefois une limite entre pieces.
        }
        OpeningKind::Door => {
            let hinge = if opening.hinge_right { right } else { left };
            let along_sign = if opening.hinge_right { -1.0 } else { 1.0 };
            let swing_sign = if opening.swing_flipped { -1.0 } else { 1.0 };
            let radius = opening.width_m * ppm;
            let leaf_end = hinge + normal * (swing_sign * radius);
            let stroke = Stroke::new(1.6_f32, Color32::from_rgb(45, 92, 139));
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
                Stroke::new(1.0_f32, Color32::from_rgb(98, 130, 164)),
            ));
        }
    }
}

/// Dessine un escalier droit vu de dessus, avec marches et fleche de montee.
fn draw_stair(painter: &Painter, stair: &Stair, origin: Pos2, ppm: f32) {
    if !stair.is_valid() { return; }
    let to_screen = |p: Point| Pos2::new(origin.x + p.x * ppm, origin.y + p.y * ppm);
    let corners = stair.corners().map(to_screen);
    let dark = Stroke::new(1.8_f32, Color32::from_rgb(49, 59, 73));
    for i in 0..4 {
        painter.line_segment([corners[i], corners[(i + 1) % 4]], dark);
    }
    for i in 1..stair.steps {
        let t = i as f32 / stair.steps as f32;
        let p = corners[0] + (corners[1] - corners[0]) * t;
        let q = corners[3] + (corners[2] - corners[3]) * t;
        painter.line_segment([p, q], Stroke::new(1.0_f32, Color32::from_rgb(105, 116, 131)));
    }
    let a = to_screen(stair.start);
    let b = to_screen(stair.end);
    let direction = (b - a).normalized();
    let normal = Vec2::new(-direction.y, direction.x);
    let from = a + (b - a) * 0.2;
    let to = a + (b - a) * 0.83;
    let arrow = Stroke::new(2.0_f32, Color32::from_rgb(27, 123, 204));
    painter.line_segment([from, to], arrow);
    painter.line_segment([to, to - direction * 13.0 + normal * 6.0], arrow);
    painter.line_segment([to, to - direction * 13.0 - normal * 6.0], arrow);
}

fn electrical_choices<T: PartialEq + Clone>(
    ui: &mut egui::Ui, value: &mut T, choices: impl IntoIterator<Item = (T, String)>,
) {
    ui.push_id(ui.next_auto_id(), |ui| ui.menu_button("Choix rapides", |ui| {
        for (choice, label) in choices {
            if ui.selectable_value(value, choice, label).clicked() {
                ui.close();
            }
        }
    }));
}

fn electrical_text(ui: &mut egui::Ui, value: &mut String, choices: &[&str]) {
    ui.horizontal(|ui| {
        electrical_choices(ui, value, choices.iter().map(|&label| (label.into(), label.into())));
        ui.add(egui::TextEdit::singleline(value).desired_width(110.0));
    });
}

fn electrical_number<T: egui::emath::Numeric>(
    ui: &mut egui::Ui, value: &mut T, range: std::ops::RangeInclusive<T>,
    choices: &[T], suffix: &str,
) {
    ui.horizontal(|ui| {
        electrical_choices(ui, value, choices.iter().map(|&n| {
            let label = format!("{}{suffix}", n.to_f64()).replace('.', ",");
            (n, label)
        }));
        ui.add(egui::DragValue::new(value).range(range).suffix(suffix));
    });
}

impl EasyDraft {
    fn project(&self) -> Project {
        let mut sheets = self.sheets.clone();
        sheets[self.active_sheet].plan = self.plan.clone();
        let first = sheets.remove(0);
        Project {
            first_plan: first.plan,
            first_sheet_name: first.name,
            additional_sheets: sheets,
            active_sheet: self.active_sheet,
            settings: self.settings,
        }
    }

    fn set_project(&mut self, project: Project) {
        self.sheets = vec![PlanSheet { name: project.first_sheet_name, plan: project.first_plan }];
        self.sheets.extend(project.additional_sheets);
        self.active_sheet = project.active_sheet.min(self.sheets.len() - 1);
        self.plan = std::mem::take(&mut self.sheets[self.active_sheet].plan);
        self.sheet_name_dialog = None;
        self.reset_sheet_editor();
        self.settings = project.settings;
        self.settings.clamp(self.plan.circuit_count());
    }

    fn replace_project(&mut self, project: Project) {
        self.cancel_drag();
        self.record_history(HistoryEntry::Project { project: self.project(), restore_settings: true });
        self.set_project(project);
    }

    fn replace_with_single_plan(&mut self, plan: Plan) {
        self.replace_project(Project {
            first_plan: plan,
            first_sheet_name: default_sheet_name(),
            additional_sheets: Vec::new(),
            active_sheet: 0,
            settings: ProjectSettings::default(),
        });
    }

    fn reset_sheet_editor(&mut self) {
        self.reset_pending();
        self.clear_selection();
        self.reveal_circuit = None;
        self.settings.active_circuit_index = 0;
    }

    fn switch_sheet(&mut self, index: usize) {
        if index != self.active_sheet {
            self.cancel_drag();
            self.sheets[self.active_sheet].plan = std::mem::take(&mut self.plan);
            self.active_sheet = index;
            self.plan = std::mem::take(&mut self.sheets[index].plan);
            self.reset_sheet_editor();
        }
    }

    fn add_sheet(&mut self, name: String) {
        self.cancel_drag();
        self.record_history(HistoryEntry::Project { project: self.project(), restore_settings: false });
        let mut plan = Plan::default();
        plan.installation_info = self.plan.installation_info.clone();
        self.sheets.push(PlanSheet { name, plan });
        self.switch_sheet(self.sheets.len() - 1);
        self.view = View::Plan2D;
        self.settings.pan = [0.0; 2];
        self.settings.zoom = 1.0;
    }

    fn sheet_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Feuille");
        let mut index = self.active_sheet;
        egui::ComboBox::from_id_salt("active_plan_sheet")
            .selected_text(&self.sheets[index].name)
            .show_ui(ui, |ui| {
                for (sheet_index, sheet) in self.sheets.iter().enumerate() {
                    ui.selectable_value(&mut index, sheet_index, &sheet.name);
                }
            });
        self.switch_sheet(index);

        ui.menu_button("Gérer les feuilles", |ui| {
            ui.label("Ajouter un niveau");
            for name in [DEFAULT_SHEET_NAME, "1er étage", "2e étage", "3e étage", "Grenier", "Cave"] {
                if ui.add_enabled(
                    !self.sheets.iter().any(|sheet| sheet.name == name),
                    egui::Button::new(name),
                ).clicked() {
                    self.add_sheet(name.into());
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Ajouter une partie…").clicked() {
                self.open_sheet_name_dialog(false);
                ui.close();
            }
            if ui.button("Renommer la feuille active…").clicked() {
                self.open_sheet_name_dialog(true);
                ui.close();
            }
        });
    }

    fn open_sheet_name_dialog(&mut self, rename: bool) {
        self.sheet_name_dialog = Some(SheetNameDialog {
            name: if rename { self.sheets[self.active_sheet].name.clone() } else { String::new() },
            rename,
            focus_input: true,
        });
    }

    fn show_sheet_name_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.sheet_name_dialog.take() else { return };
        let modal = egui::Modal::new(egui::Id::new("sheet_name_dialog")).show(ctx, |ui| {
            ui.heading(if dialog.rename { "Renommer la feuille" } else { "Ajouter une partie" });
            ui.label("Nom de la feuille");
            let input = ui.add(egui::TextEdit::singleline(&mut dialog.name)
                .hint_text("Ex. Annexe")
                .desired_width(260.0)
                .char_limit(60));
            if std::mem::take(&mut dialog.focus_input) {
                input.request_focus();
            }

            let name = dialog.name.trim();
            let duplicate = self.sheets.iter().enumerate().any(|(index, sheet)| {
                (!dialog.rename || index != self.active_sheet) && sheet.name == name
            });
            if duplicate {
                ui.label("Une feuille porte déjà ce nom.");
            }
            let valid = !name.is_empty() && !duplicate;
            ui.horizontal(|ui| {
                let confirm = ui.add_enabled(valid, egui::Button::new(
                    if dialog.rename { "Renommer" } else { "Ajouter" }
                )).clicked();
                let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let cancel = ui.button("Annuler").clicked();
                (valid && (confirm || enter), cancel)
            }).inner
        });

        let (confirmed, cancelled) = modal.inner;
        if confirmed {
            let name = dialog.name.trim().to_owned();
            if dialog.rename {
                self.rename_active_sheet(name);
            } else {
                self.add_sheet(name);
            }
        } else if !cancelled && !modal.should_close() {
            self.sheet_name_dialog = Some(dialog);
        }
    }

    fn rename_active_sheet(&mut self, name: String) {
        let sheet = self.active_sheet;
        if self.sheets[sheet].name != name {
            let previous = std::mem::replace(&mut self.sheets[sheet].name, name);
            self.record_history(HistoryEntry::SheetName { sheet, name: previous });
        }
    }

    fn record_history(&mut self, entry: HistoryEntry) {
        self.undo.push(entry);
        self.redo.clear();
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
    }

    fn snapshot(&mut self) {
        self.record_history(HistoryEntry::Plan { sheet: self.active_sheet, plan: self.plan.clone() });
    }

    fn restore_history(&mut self, entry: HistoryEntry) -> HistoryEntry {
        match entry {
            HistoryEntry::Plan { sheet, plan } => {
                self.switch_sheet(sheet);
                let previous = std::mem::replace(&mut self.plan, plan);
                self.reset_pending();
                self.clear_selection();
                HistoryEntry::Plan { sheet, plan: previous }
            }
            HistoryEntry::Project { mut project, restore_settings } => {
                let previous = self.project();
                if !restore_settings {
                    project.settings = self.settings;
                }
                self.set_project(project);
                HistoryEntry::Project { project: previous, restore_settings }
            }
            HistoryEntry::SheetName { sheet, name } => {
                let previous = std::mem::replace(&mut self.sheets[sheet].name, name);
                HistoryEntry::SheetName { sheet, name: previous }
            }
        }
    }

    /// Annule un glissement non confirme et restaure l'etat d'origine.
    fn cancel_drag(&mut self) {
        if let Some(session) = self.drag.take() {
            self.plan = session.original;
        }
        self.marquee = None;
        self.marquee_handles = false;
    }

    fn clear_selection(&mut self) {
        self.selected_handles.clear();
        self.selection = SelectionSet::default();
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
        if let Some(entry) = self.undo.pop() {
            let previous = self.restore_history(entry);
            self.redo.push(previous);
            self.status = "Derniere modification annulee".into();
        }
    }

    fn redo(&mut self) {
        self.cancel_drag();
        if let Some(entry) = self.redo.pop() {
            let previous = self.restore_history(entry);
            self.undo.push(previous);
            self.status = "Modification retablie".into();
        }
    }

    fn refresh_underlay_texture(&mut self, ctx: &egui::Context) {
        let Some(bg) = self.plan.background.as_ref() else {
            self.underlay_texture = None;
            self.underlay_loaded_key = None;
            return;
        };
        let key = format!("{}|{}", bg.path, bg.rotation_quarters % 4);
        if self.underlay_loaded_key.as_ref() == Some(&key) { return; }
        self.underlay_loaded_key = Some(key);
        self.underlay_texture = None;
        let decoded = std::fs::read(&bg.path)
            .map_err(|error| format!("Impossible de lire le fond de plan : {error}"))
            .and_then(|data| image::load_from_memory(&data)
                .map_err(|error| format!("Image invalide : {error}")));
        match decoded {
            Ok(image) => {
                let rgba = image.to_rgba8();
                let pixels = match bg.rotation_quarters % 4 {
                    1 => image::imageops::rotate90(&rgba),
                    2 => image::imageops::rotate180(&rgba),
                    3 => image::imageops::rotate270(&rgba),
                    _ => rgba,
                };
                let dimensions = [pixels.width() as usize, pixels.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(dimensions, pixels.as_raw());
                self.underlay_texture = Some(ctx.load_texture(
                    "easydraft-plan-underlay", color, egui::TextureOptions::LINEAR
                ));
                self.underlay_dimensions = dimensions;
            }
            Err(error) => self.status = error,
        }
    }

    fn import_underlay(&mut self, ctx: &egui::Context) {
        let chosen = rfd::FileDialog::new()
            .set_title("Importer un fond de plan (image)")
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
            .pick_file();
        let Some(path) = chosen else { return; };
        // Confirme le format avant de modifier le plan ou l'historique.
        let image = std::fs::read(&path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| image::load_from_memory(&bytes).map_err(|error| error.to_string()));
        match image {
            Ok(image) => {
                let ratio = image.height() as f32 / image.width().max(1) as f32;
                self.snapshot();
                self.plan.background = Some(BackgroundPlan {
                    path: path.to_string_lossy().into_owned(),
                    width_m: 15.0,
                    origin: Point { x: -7.5, y: -7.5 * ratio },
                    opacity: 0.46,
                    rotation_quarters: 0,
                });
                self.settings.show_underlay = true;
                self.underlay_loaded_key = None;
                self.refresh_underlay_texture(ctx);
                self.sidebar_tab = SidebarTab::Affichage;
                self.status = "Fond de plan importe : calibre sa largeur reelle dans Parametres > Plan".into();
            }
            Err(error) => self.status = format!("Import impossible : {error}"),
        }
    }

    fn file_picker(current: &str, title: &str, label: &str, extension: &'static str) -> rfd::FileDialog {
        let path = Path::new(current);
        let suggested_name = path.file_name().and_then(|name| name.to_str())
            .unwrap_or("easydraft");
        let mut picker = rfd::FileDialog::new()
            .set_title(title)
            .add_filter(label, &[extension])
            .set_file_name(suggested_name);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            picker = picker.set_directory(parent);
        }
        picker
    }

    fn save_current(&mut self) {
        let result = serde_json::to_string_pretty(&self.project())
            .map_err(|e| e.to_string())
            .and_then(|data| std::fs::write(&self.file_path, data).map_err(|e| e.to_string()));
        self.status = match result {
            Ok(()) => {
                self.file_path_assigned = true;
                format!("Plan enregistre : {}", self.file_path)
            }
            Err(e) => format!("Erreur d'enregistrement : {e}"),
        };
    }

    /// Premiere sauvegarde : choisir un fichier. Ensuite, enregistrer directement.
    fn save(&mut self) {
        if self.file_path_assigned {
            self.save_current();
        } else {
            self.save_as();
        }
    }

    /// Demande toujours une destination. Annuler le dialogue ne modifie rien.
    fn save_as(&mut self) {
        if let Some(path) = Self::file_picker(
            &self.file_path, "Enregistrer le plan EasyDraft", "Plan EasyDraft (JSON)", "json"
        ).save_file() {
            let path = path.with_extension("json");
            let result = serde_json::to_string_pretty(&self.project())
                .map_err(|e| e.to_string())
                .and_then(|data| std::fs::write(&path, data).map_err(|e| e.to_string()));
            match result {
                Ok(()) => {
                    self.file_path = path.to_string_lossy().into_owned();
                    self.file_path_assigned = true;
                    self.status = format!("Plan enregistre : {}", self.file_path);
                }
                Err(error) => self.status = format!("Erreur d'enregistrement : {error}"),
            }
        }
    }

    /// Navigateur natif d'ouverture d'un fichier JSON.
    fn open_dialog(&mut self) {
        if let Some(path) = Self::file_picker(
            &self.file_path, "Ouvrir un plan EasyDraft", "Plan EasyDraft (JSON)", "json"
        ).pick_file() {
            let result = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<Project>(&text).map_err(|e| e.to_string()));
            match result {
                Ok(project) => {
                    self.replace_project(project);
                    self.file_path = path.to_string_lossy().into_owned();
                    self.file_path_assigned = true;
                    self.status = format!("Plan ouvert : {}", self.file_path);
                }
                Err(error) => self.status = format!("Erreur d'ouverture : {error}"),
            }
        }
    }

    fn load_board_example(&mut self) {
        self.cancel_drag();
        match serde_json::from_str::<Plan>(
            include_str!("../examples/tableau-deux-ddr-trois-circuits.json")
        ) {
            Ok(plan) => {
                self.replace_with_single_plan(plan);
                self.view = View::Unifilar;
                self.settings.show_electrical = true;
                self.status = "Exemple de tableau charge : D1 vers A/B, D2 vers C (caracteristiques a renseigner)".into();
            }
            Err(error) => self.status = format!("Exemple tableau invalide : {error}"),
        }
    }

    fn load_unifilar_example(&mut self) {
        self.cancel_drag();
        match serde_json::from_str::<Plan>(
            include_str!("../examples/unifilaire-deux-circuits.json")
        ) {
            Ok(plan) => {
                self.replace_with_single_plan(plan);
                self.view = View::Unifilar;
                self.settings.show_electrical = true;
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
                self.replace_with_single_plan(plan);
                self.view = View::Plan2D;
                self.settings.show_architecture = true;
                self.settings.show_rooms = true;
                self.settings.show_electrical = true;
                self.settings.pan = [0.0; 2];
                self.settings.zoom = 1.0;
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
            self.settings.show_electrical = true;
        }
        if matches!(tool, Tool::Wall | Tool::Door | Tool::Window | Tool::Passage | Tool::Stair) {
            self.settings.show_architecture = true;
        }
    }

    /// Priorite aux extremites des autres murs lorsqu'elles sont proches du pointeur.
    fn snap_point(&self, raw: Point, ppm: f32) -> Point {
        if self.settings.snap_endpoints {
            if let Some(endpoint) = self.plan.walls.iter()
                .flat_map(|wall| [wall.start, wall.end])
                .filter(|endpoint| raw.distance_to(*endpoint) <= 13.0 / ppm)
                .min_by(|a, b| raw.distance_to(*a).total_cmp(&raw.distance_to(*b)))
            {
                return endpoint;
            }
        }
        raw.snapped(self.settings.snap_m)
    }

    fn symbol_visible_on_plan(&self, symbol_index: usize) -> bool {
        if !self.settings.show_electrical || self.settings.circuit_visibility == CircuitVisibility::None {
            return false;
        }
        if self.settings.circuit_visibility == CircuitVisibility::All {
            return true;
        }
        let Some(symbol) = self.plan.symbols.get(symbol_index) else { return false; };
        match symbol.kind {
            ElectricalKind::Outlet => self.plan.outlet_assignments.iter().any(|assignment|
                assignment.symbol_index == symbol_index
                    && assignment.circuit_index == self.settings.active_circuit_index
            ),
            ElectricalKind::Switch | ElectricalKind::Light =>
                self.plan.active_lighting_connections_for_circuit(self.settings.active_circuit_index)
                    .iter().any(|link|
                        link.switch_index == symbol_index || link.light_index == symbol_index
                    ),
        }
    }

    fn find_symbol_any(&self, point: Point, ppm: f32) -> Option<usize> {
        self.plan.symbols.iter().enumerate()
            .map(|(index, symbol)| (index, point.distance_to(symbol.position)))
            .filter(|(index, distance)| *distance <= 17.0 / ppm
                && self.symbol_visible_on_plan(*index))
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

    fn wall_handles_at(&self, point: Point, ppm: f32) -> Vec<WallHandle> {
        if !self.settings.show_architecture { return Vec::new(); }
        let nearest = self.plan.walls.iter().enumerate()
            .flat_map(|(wall_index, wall)| [
                (WallHandle { wall_index, is_start: true }, wall.start),
                (WallHandle { wall_index, is_start: false }, wall.end),
            ])
            .filter(|(_, position)| position.distance_to(point) <= 12.0 / ppm)
            .min_by(|a, b| point.distance_to(a.1).total_cmp(&point.distance_to(b.1)));
        nearest.map(|(handle, _)| self.plan.linked_wall_endpoints(handle))
            .unwrap_or_default()
    }

    /// Parmi des points geométriquement confondus mais NON joints, alterner
    /// celui qui est choisi a chaque clic sur le meme endroit.
    /// Un groupe explicitement lie reste selectionne comme un seul noeud.
    fn cycle_coincident_wall_handle(&self, point: Point, ppm: f32) -> Vec<WallHandle> {
        let mut handles = self.wall_handles_at(point, ppm);
        if handles.is_empty() || self.selected_handles.len() != 1 {
            return handles;
        }
        let first = handles[0];
        let Some(location) = first.position(&self.plan) else { return handles };
        let candidates: Vec<WallHandle> = self.plan.walls.iter().enumerate()
            .flat_map(|(wall_index, wall)| [
                (WallHandle { wall_index, is_start: true }, wall.start),
                (WallHandle { wall_index, is_start: false }, wall.end),
            ])
            .filter(|(_, at)| at.distance_to(location) < 0.0001)
            .map(|(handle, _)| handle).collect();
        if candidates.len() < 2 { return handles; }
        if let Some(position) = candidates.iter()
            .position(|handle| *handle == self.selected_handles[0])
        {
            let next = candidates[(position + 1) % candidates.len()];
            handles = self.plan.linked_wall_endpoints(next);
        }
        handles
    }

    /// Maj + clic ajoute ou retire un groupe de points deja raccordes.
    fn toggle_wall_handles(&mut self, point: Point, ppm: f32) -> bool {
        let mut clicked = self.wall_handles_at(point, ppm);
        if clicked.is_empty() { return false; }
        // Permet aussi de Maj-selectionner plusieurs extremites superposees
        // qui n'ont PAS ete jointes explicitement.
        if clicked.len() == 1 && self.selected_handles.contains(&clicked[0]) {
            let location = clicked[0].position(&self.plan);
            if let Some(location) = location {
                let other = self.plan.walls.iter().enumerate().flat_map(|(wall_index, wall)| [
                    (WallHandle {wall_index,is_start:true},wall.start),
                    (WallHandle {wall_index,is_start:false},wall.end),
                ]).find(|(handle, pos)| pos.distance_to(location) < 0.0001
                    && !self.selected_handles.contains(handle));
                if let Some((next, _)) = other {
                    clicked = self.plan.linked_wall_endpoints(next);
                }
            }
        }
        if self.selected_handles.is_empty() { self.clear_selection(); }
        let all_selected = clicked.iter().all(|handle| self.selected_handles.contains(handle));
        if all_selected {
            self.selected_handles.retain(|handle| !clicked.contains(handle));
        } else {
            for handle in clicked {
                if !self.selected_handles.contains(&handle) {
                    self.selected_handles.push(handle);
                }
            }
        }
        self.sidebar_tab = SidebarTab::Inspecteur;
        self.status = format!(
            "{} point(s) selectionne(s) : Joindre pour lier / Dissoudre pour separer",
            self.selected_handles.len()
        );
        true
    }

    fn snap_joint_destination(&self, raw: Point, ppm: f32) -> Point {
        if self.settings.snap_endpoints {
            if let Some(position) = self.plan.walls.iter().enumerate()
                .flat_map(|(wall_index, wall)| [
                    (WallHandle { wall_index, is_start: true }, wall.start),
                    (WallHandle { wall_index, is_start: false }, wall.end),
                ])
                .filter(|(handle, position)| !self.selected_handles.contains(handle)
                    && position.distance_to(raw) <= 13.0 / ppm)
                .min_by(|a, b| raw.distance_to(a.1).total_cmp(&raw.distance_to(b.1)))
                .map(|(_, position)| position)
            {
                return position;
            }
        }
        raw.snapped(self.settings.snap_m)
    }

    /// Selectionne d'abord une ouverture, puis une poignee de mur, puis le corps du mur.
    fn drag_target_at(&self, point: Point, ppm: f32) -> Option<DragTarget> {
        if self.settings.show_electrical {
            if let Some(index) = self.find_symbol_any(point, ppm) {
                return Some(DragTarget::Symbol(index));
            }
        }
        if !self.settings.show_architecture { return None; }
        if let Some(index) = self.find_stair(point, ppm) {
            return Some(DragTarget::Stair(index));
        }
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
        if self.settings.snap_endpoints {
            if let Some(point) = self.plan.walls.iter().enumerate()
                .filter(|(index, _)| *index != moving_wall)
                .flat_map(|(_, wall)| [wall.start, wall.end])
                .filter(|point| raw.distance_to(*point) <= 13.0 / ppm)
                .min_by(|a, b| raw.distance_to(*a).total_cmp(&raw.distance_to(*b)))
            {
                return point;
            }
        }
        raw.snapped(self.settings.snap_m)
    }

    fn begin_drag(&mut self, point: Point, ppm: f32) {
        let Some(hit) = self.drag_target_at(point, ppm) else {
            if !self.marquee_handles { self.clear_selection(); }
            self.marquee = Some(Marquee { start: point, current: point });
            self.sidebar_tab = SidebarTab::Inspecteur;
            self.status = if self.marquee_handles {
                "Encadre les extremites des murs a raccorder (Maj + glisser)".into()
            } else {
                "Selection rectangulaire : encadre les elements".into()
            };
            return;
        };
        // Seuls les groupes explicitement Joints suivent automatiquement.
        // Une coincidence geometrique n'est pas une liaison d'edition.
        if matches!(hit, DragTarget::WallStart(_) | DragTarget::WallEnd(_)) {
            let mut clicked = self.wall_handles_at(point, ppm);
            // Conserver le point choisi lors d'un clic precedent, utile
            // lorsque deux extremites dissoutes se superposent encore.
            let chosen = self.selected_handles.first().copied()
                .filter(|h| h.position(&self.plan)
                    .is_some_and(|p| p.distance_to(point) <= 12.0 / ppm));
            if let Some(chosen) = chosen {
                clicked = self.plan.linked_wall_endpoints(chosen);
            }
            let keep = clicked.iter().any(|handle| self.selected_handles.contains(handle));
            if !keep { self.clear_selection(); }
            for handle in clicked {
                if !self.selected_handles.contains(&handle) {
                    self.selected_handles.push(handle);
                }
            }
            let target = if self.selected_handles.len() > 1 {
                DragTarget::Joint
            } else {
                hit
            };
            self.sidebar_tab = SidebarTab::Inspecteur;
            self.status = if self.selected_handles.len() > 1 {
                format!("Deplace {} points ensemble ; utiliser Joindre pour les lier", self.selected_handles.len())
            } else {
                "Deplace le point seul (Maj + clic pour en ajouter, Joindre pour les lier)".into()
            };
            self.drag = Some(ActiveDrag {
                target,
                from: point,
                original: self.plan.clone(),
                changed: false,
            });
            return;
        }
        let target = if self.selection.len() > 1 && self.selection.includes(hit) {
            DragTarget::Group
        } else {
            hit
        };
        if !matches!(target, DragTarget::Group) {
            self.clear_selection();
            match target {
                DragTarget::Opening(index) => {
                    self.selected_opening = Some(index);
                    self.status = "Glisse l'ouverture le long de son mur".into();
                }
                DragTarget::WallStart(index) | DragTarget::WallEnd(index) => {
                    self.selected_wall = Some(index);
                    self.status = "Deplace la poignee du mur".into();
                }
                DragTarget::WallBody(index) => {
                    self.selected_wall = Some(index);
                    self.status = "Deplace le mur avec ses ouvertures".into();
                }
                DragTarget::Symbol(index) => {
                    self.selected_symbol = Some(index);
                    self.status = "Deplace le symbole electrique".into();
                }
                DragTarget::Stair(index) => {
                    self.selected_stair = Some(index);
                    self.status = "Deplace l'escalier".into();
                }
                DragTarget::Group | DragTarget::Joint => unreachable!(),
            }
        } else {
            self.status = format!("Deplacement de {} elements", self.selection.len());
        }
        self.sidebar_tab = SidebarTab::Inspecteur;
        self.drag = Some(ActiveDrag {
            target,
            from: point,
            original: self.plan.clone(),
            changed: false,
        });
    }

    /// Joindre est une action EXPLICITE. Si un seul point est selectionne,
    /// il peut s'accrocher a un mur perpendiculaire proche (utile pour fermer
    /// les petits manques de jonction en T dans les pieces).
    fn weld_selected_to_first(&mut self) {
        let Some(&first) = self.selected_handles.first() else {
            self.status = "Selectionne une ou plusieurs extremites de murs".into();
            return;
        };
        let mut updated = self.plan.clone();
        if self.selected_handles.len() >= 2 {
            match updated.join_wall_endpoints(&self.selected_handles) {
                Ok(count) => {
                    self.snapshot();
                    self.plan = updated;
                    self.selected_handles = self.plan.linked_wall_endpoints(first);
                    self.status = format!(
                        "Joindre : {count} extremites liees, desormais deplacables ensemble (Ctrl+Z)"
                    );
                }
                Err(error) => self.status = format!("Joindre refuse : {error}"),
            }
            return;
        }

        // Une seule extremite choisie : accrochage volontaire au mur dont
        // elle approche l'axe a moins de 20 cm. Cette commande ne met pas
        // le plan en etat de liaison automatique.
        let Some(point) = first.position(&self.plan) else {
            self.status = "Sommet introuvable".into();
            return;
        };
        let mut best: Option<(usize, Point, f32)> = None;
        for (index, wall) in self.plan.walls.iter().enumerate() {
            if index == first.wall_index { continue; }
            let segment = Point {
                x: wall.end.x - wall.start.x,
                y: wall.end.y - wall.start.y,
            };
            let sq = segment.x * segment.x + segment.y * segment.y;
            if sq < 0.0025 { continue; }
            let along = ((point.x - wall.start.x) * segment.x
                + (point.y - wall.start.y) * segment.y) / sq;
            if !(-0.03..=1.03).contains(&along) { continue; }
            let t = along.clamp(0.0, 1.0);
            let projection = Point {
                x: wall.start.x + t * segment.x,
                y: wall.start.y + t * segment.y,
            };
            let distance = point.distance_to(projection);
            if distance <= 0.20 &&
                best.as_ref().is_none_or(|previous| distance < previous.2)
            {
                best = Some((index, projection, distance));
            }
        }
        let Some((wall_index, target, distance)) = best else {
            self.status = "Joindre : selectionne aussi un autre point, ou approche ce sommet d'un mur (20 cm max)".into();
            return;
        };
        if distance <= 0.0001 {
            self.status = "Ce point touche deja le mur. Pour lier deux extremites, selectionne les deux avec Maj".into();
            return;
        }
        // Joindre a une extremite du mur cible cree un vrai groupe de liens.
        // Joindre en T accroche geometriquement l'extremite a l'axe cible.
        let wall = &self.plan.walls[wall_index];
        let other = if target.distance_to(wall.start) <= 0.0001 {
            Some(WallHandle {wall_index,is_start:true})
        } else if target.distance_to(wall.end) <= 0.0001 {
            Some(WallHandle {wall_index,is_start:false})
        } else { None };
        let outcome = if let Some(other) = other {
            updated.join_wall_endpoints(&[first,other]).map(|count| {
                format!("Joindre : {count} sommets lies sur un noeud commun")
            })
        } else {
            updated.move_wall_endpoints_to(&[first], target).map(|_| {
                format!("Jonction en T creee : sommet raccorde a l'axe du mur {wall_index}")
            })
        };
        match outcome {
            Ok(message) => {
                self.snapshot();
                self.plan = updated;
                self.selected_handles = self.plan.linked_wall_endpoints(first);
                self.status = message;
            }
            Err(error) => self.status = format!("Joindre refuse : {error}"),
        }
    }

    /// Dissoudre les liaisons explicites sans deplacer la geometrie du plan.
    /// Une extremite peut ensuite etre glissee independamment.
    fn dissolve_selected_joints(&mut self) {
        if self.selected_handles.is_empty() {
            self.status = "Selectionne un point d'une jonction".into();
            return;
        }
        let mut updated = self.plan.clone();
        let count = updated.dissolve_wall_joints(&self.selected_handles);
        if count == 0 {
            self.status = "Aucun groupe de sommets explicitement joint a dissoudre".into();
            return;
        }
        self.snapshot();
        self.plan = updated;
        self.selected_handles.truncate(1);
        self.sidebar_tab = SidebarTab::Inspecteur;
        self.status = format!(
            "Dissoudre : {count} jonction(s) liberee(s). Deplace ce sommet seul (Ctrl+Z)"
        );
    }

    fn update_marquee(&mut self, point: Point) {
        if let Some(marquee) = self.marquee.as_mut() {
            marquee.current = point;
        }
    }

    fn finish_marquee(&mut self) {
        let Some(marquee) = self.marquee.take() else { return; };
        let handles_only = self.marquee_handles;
        self.marquee_handles = false;
        if handles_only {
            let mut added = 0;
            for (wall_index, wall) in self.plan.walls.iter().enumerate() {
                for (handle, position) in [
                    (WallHandle { wall_index, is_start: true }, wall.start),
                    (WallHandle { wall_index, is_start: false }, wall.end),
                ] {
                    if marquee.contains(position) && !self.selected_handles.contains(&handle) {
                        self.selected_handles.push(handle);
                        added += 1;
                    }
                }
            }
            self.sidebar_tab = SidebarTab::Inspecteur;
            self.status = format!(
                "{added} point(s) ajoute(s), {} au total : glisse pour deplacer, puis Joindre",
                self.selected_handles.len()
            );
            return;
        }
        self.clear_selection();
        self.selection = SelectionSet::from_marquee(&self.plan, marquee);
        let count = self.selection.len();
        if count == 1 {
            self.selected_wall = self.selection.walls.first().copied();
            self.selected_opening = self.selection.openings.first().copied();
            self.selected_symbol = self.selection.symbols.first().copied();
            self.selected_stair = self.selection.stairs.first().copied();
        }
        self.sidebar_tab = SidebarTab::Inspecteur;
        self.status = if count == 0 {
            "Aucun element dans le rectangle".into()
        } else {
            format!("{count} element(s) selectionne(s) : glisse l'un d'eux pour deplacer le groupe")
        };
    }

    fn update_drag(&mut self, cursor: Point, ppm: f32) {
        let Some(drag) = self.drag.as_ref() else { return; };
        let from = drag.from;
        let target = drag.target;
        let delta = Point { x: cursor.x - from.x, y: cursor.y - from.y };
        let mut updated = drag.original.clone();

        let outcome = match target {
            DragTarget::Joint => {
                // Les points choisis avancent ensemble en conservant leurs
                // positions relatives. Ils ne deviennent lies QUE si le
                // bouton Joindre est utilise.
                let destination = self.snap_joint_destination(cursor, ppm);
                let shift = Point {
                    x: destination.x - from.x,
                    y: destination.y - from.y,
                };
                updated.move_wall_endpoints_by(&self.selected_handles, shift)
            }
            DragTarget::Opening(index) => {
                if let Some(opening) = updated.openings.get(index) {
                    if let Some(wall) = updated.walls.get(opening.wall_index) {
                        let along = wall.projected_distance_m(cursor) - wall.projected_distance_m(from);
                        let center = ((opening.center_m + along) / self.settings.snap_m).round() * self.settings.snap_m;
                        let margin = if opening.kind == OpeningKind::Passage { 0.0 } else { 0.05 };
                        let min_center = opening.width_m / 2.0 + margin;
                        let max_center = wall.length_m() - min_center;
                        if min_center <= max_center {
                            updated.move_opening(index, center.clamp(min_center, max_center))
                        } else {
                            Err("Ce mur est trop court pour l'ouverture")
                        }
                    } else { Err("Mur introuvable") }
                } else { Err("Ouverture introuvable") }
            }
            DragTarget::Symbol(_) | DragTarget::Stair(_) | DragTarget::Group => {
                let shift = Point {
                    x: (delta.x / self.settings.snap_m).round() * self.settings.snap_m,
                    y: (delta.y / self.settings.snap_m).round() * self.settings.snap_m,
                };
                let selected = match target {
                    DragTarget::Symbol(i) => SelectionSet {
                        symbols: vec![i], ..SelectionSet::default()
                    },
                    DragTarget::Stair(i) => SelectionSet {
                        stairs: vec![i], ..SelectionSet::default()
                    },
                    DragTarget::Group => self.selection.clone(),
                    _ => unreachable!(),
                };
                translate_selection(&mut updated, &selected, shift)
            }
            DragTarget::WallBody(index) => {
                if let Some(wall) = updated.walls.get(index).cloned() {
                    let d = Point {
                        x: (delta.x / self.settings.snap_m).round() * self.settings.snap_m,
                        y: (delta.y / self.settings.snap_m).round() * self.settings.snap_m,
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
                    DragTarget::Joint => self.selected_handles.iter().any(|&handle| {
                        match (handle.position(&drag.original), handle.position(&updated)) {
                            (Some(before), Some(after)) => before.distance_to(after) > 0.0001,
                            _ => false,
                        }
                    }),
                    DragTarget::Symbol(index) => {
                        let old = drag.original.symbols[index].position;
                        let now = updated.symbols[index].position;
                        old.distance_to(now) > 0.0001
                    }
                    DragTarget::Stair(index) => {
                        let old = &drag.original.stairs[index];
                        let now = &updated.stairs[index];
                        old.start.distance_to(now.start) > 0.0001
                    }
                    DragTarget::Group => {
                        delta.x.abs() + delta.y.abs() > 0.0001
                            && self.selection.len() > 1
                            && (self.selection.walls.iter().any(|&i|
                                drag.original.walls[i].start.distance_to(updated.walls[i].start) > 0.0001)
                            || self.selection.symbols.iter().any(|&i|
                                drag.original.symbols[i].position.distance_to(updated.symbols[i].position) > 0.0001)
                            || self.selection.stairs.iter().any(|&i|
                                drag.original.stairs[i].start.distance_to(updated.stairs[i].start) > 0.0001)
                            || self.selection.openings.iter().any(|&i|
                                (drag.original.openings[i].center_m - updated.openings[i].center_m).abs() > 0.0001))
                    }
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
                self.record_history(HistoryEntry::Plan {
                    sheet: self.active_sheet,
                    plan: session.original,
                });
                self.status = if matches!(session.target, DragTarget::Joint) {
                    "Points de murs deplaces (Ctrl+Z pour annuler ; Joindre pour les lier)".into()
                } else {
                    "Deplacement termine : derniere position valide conservee (Ctrl+Z pour annuler)".into()
                };
            }
        }
    }

    fn select_at(&mut self, point: Point, ppm: f32) {
        let clicked = self.cycle_coincident_wall_handle(point, ppm);
        if !clicked.is_empty() {
            self.clear_selection();
            self.selected_handles = clicked;
            self.sidebar_tab = SidebarTab::Inspecteur;
            self.status = format!(
                "{} sommet(s) selectionne(s) : Maj + clic, puis Joindre ou Dissoudre",
                self.selected_handles.len()
            );
            return;
        }
        self.clear_selection();
        self.sidebar_tab = SidebarTab::Inspecteur;
        if self.settings.show_electrical {
            self.selected_symbol = self.find_symbol_any(point, ppm);
            if self.selected_symbol.is_some() {
                self.status = "Symbole electrique selectionne".into();
                return;
            }
        }
        if self.settings.show_architecture {
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
        if self.selection.len() > 1 {
            let mut updated = self.plan.clone();
            // Supprimer par indices decroissants pour eviter le decalage des tableaux.
            let mut openings = self.selection.openings.clone();
            openings.sort_unstable_by(|a,b| b.cmp(a));
            let mut walls = self.selection.walls.clone();
            walls.sort_unstable_by(|a,b| b.cmp(a));
            let mut symbols = self.selection.symbols.clone();
            symbols.sort_unstable_by(|a,b| b.cmp(a));
            let mut stairs = self.selection.stairs.clone();
            stairs.sort_unstable_by(|a,b| b.cmp(a));
            let result = (|| -> Result<(), &'static str> {
                for index in openings { updated.remove_opening(index)?; }
                for index in walls { updated.remove_wall(index)?; }
                for index in symbols { updated.remove_symbol(index)?; }
                for index in stairs { updated.remove_stair(index)?; }
                Ok(())
            })();
            match result {
                Ok(()) => {
                    self.snapshot();
                    self.plan = updated;
                    self.clear_selection();
                    self.status = "Groupe supprime (Ctrl+Z pour annuler)".into();
                }
                Err(error) => self.status = format!("Suppression impossible : {error}"),
            }
            return;
        }
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
    /// Edition des appareils de protection amont. Aucune valeur technique
    /// n'est pre-remplie : les caracteristiques doivent etre relevees sur place.
    fn board_editor(&mut self, ui: &mut egui::Ui) {
        if ui.button("Charger l'exemple tableau D1 / D2").clicked() {
            self.load_board_example();
        }
        ui.small("Exemple de topologie uniquement : aucune valeur de protection n'est supposee.");
        ui.label(format!(
            "{} dispositif(s) differentiel(s) - {} circuit(s) non attribue(s)",
            self.plan.differentials.len(),
            self.plan.circuits_without_differential().len(),
        ));
        if ui.button("Ajouter un differentiel (D1, D2...)").clicked() {
            let mut updated = self.plan.clone();
            match updated.add_differential() {
                Ok(_) => {
                    self.snapshot();
                    self.plan = updated;
                    self.status = "Protection differentielle creee ; renseigne ses caracteristiques".into();
                }
                Err(error) => self.status = error.into(),
            }
        }
        for index in 0..self.plan.differentials.len() {
            let Some(original) = self.plan.differentials.get(index).cloned() else { break; };
            let circuits = self.plan.circuits_for_differential(index);
            egui::CollapsingHeader::new(format!(
                "{} - {} circuit(s) en aval", original.reference, circuits.len()
            )).default_open(false).show(ui, |ui| {
                let mut edited = original.clone();
                ui.label("Repere du differentiel (D1, D2...)");
                ui.text_edit_singleline(&mut edited.reference);
                ui.label("Type du DDR (ex. A, F, B selon appareil)");
                electrical_text(ui, &mut edited.kind, &["AC", "A", "F", "B"]);
                ui.label("Courant nominal In");
                electrical_number(ui, &mut edited.rated_current_amp, 0..=125, &[25, 40, 63, 80, 100], " A");
                ui.label("Sensibilite I delta n");
                electrical_number(ui, &mut edited.sensitivity_ma, 0..=1000, &[10, 30, 100, 300, 500], " mA");
                ui.label("Nombre de poles");
                electrical_number(ui, &mut edited.poles, 0..=4, &[2, 4], "");
                ui.label("Observation / modele");
                ui.text_edit_singleline(&mut edited.notes);
                if edited != original {
                    let mut updated = self.plan.clone();
                    match updated.update_differential(index, edited) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.status = "Dispositif differentiel modifie".into();
                        }
                        Err(error) => self.status = error.into(),
                    }
                }
                let refs = circuits.iter().filter_map(|&i|
                    self.plan.circuit(i).map(|c| c.letter.as_str())
                ).collect::<Vec<_>>().join(", ");
                ui.small(format!(
                    "Departs associes : {}",
                    if refs.is_empty() { "(aucun)" } else { &refs }
                ));
                if ui.button("Supprimer ce differentiel non utilise").clicked() {
                    let mut updated = self.plan.clone();
                    match updated.remove_differential(index) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.status = "Differentiel inutilise supprime (references recalculees)".into();
                        }
                        Err(error) => self.status = error.into(),
                    }
                }
            });
        }
        ui.small("Ordre modele : alimentation -> DDR -> protection de circuit -> appareils.");
        ui.small("Aucune valeur par defaut ni validation de dimensionnement.");
    }

    fn selected_properties(&mut self, ui: &mut egui::Ui) {
        if let Some(index) = self.selected_opening {
            if let Some(opening) = self.plan.openings.get(index).cloned() {
                ui.heading(match opening.kind {
                    OpeningKind::Door => "Porte",
                    OpeningKind::Window => "Fenetre",
                    OpeningKind::Passage => "Passage libre",
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
                    OpeningKind::Passage => 40.0..=400.0,
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
                let current_style = self.plan.symbol_appearance(index);
                let mut selected_style = current_style;
                egui::ComboBox::from_label("Symbole (RGIE tableau 2.23)")
                    .selected_text(current_style.label())
                    .show_ui(ui, |ui| {
                        for &option in ElectricalAppearance::choices(symbol.kind) {
                            ui.selectable_value(
                                &mut selected_style, option, option.label()
                            );
                        }
                    });
                if selected_style != current_style {
                    let mut updated = self.plan.clone();
                    match updated.set_symbol_appearance(index, selected_style) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.status = format!(
                                "Symbole modifie : {} (verifier la caracteristique reelle)",
                                selected_style.label()
                            );
                        }
                        Err(err) => self.status = err.into(),
                    }
                }
                if matches!(selected_style,
                    ElectricalAppearance::SwitchTwoWay | ElectricalAppearance::PushButton) {
                    ui.small("Ce pictogramme ne simule pas encore les commandes va-et-vient / telerupteur.");
                }
                self.show_component_circuit(ui, index, symbol.kind);
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
        match updated.add_demo_lighting_in_circuit(self.settings.active_circuit_index) {
            Ok(()) => {
                self.snapshot();
                self.plan = updated;
                self.pending_switch = None;
                self.view = View::Plan2D;
                self.status = format!(
                    "Exemple ajoute au circuit {}. Ouvre le schema unifilaire.",
                    self.plan.circuit(self.settings.active_circuit_index)
                        .map(|c| c.letter.as_str()).unwrap_or("?")
                );
            }
            Err(error) => self.status = error.into(),
        }
    }

    fn show_component_circuit(&mut self, ui: &mut egui::Ui, index: usize, kind: ElectricalKind) {
        let (current, command) = if kind == ElectricalKind::Outlet {
            let circuit = self.plan.active_outlet_assignments().into_iter()
                .find(|a| a.symbol_index == index).map(|a| a.circuit_index);
            ui.label(format!("Repère : {}", self.plan.point_ref(index).as_deref().unwrap_or("—")));
            (circuit, None)
        } else {
            let link = self.plan.active_lighting_connections().into_iter().find(|link| {
                match kind {
                    ElectricalKind::Switch => link.switch_index == index,
                    ElectricalKind::Light => link.light_index == index,
                    ElectricalKind::Outlet => false,
                }
            });
            let Some(link) = link else {
                ui.small("Utilise « Lier I → L » pour associer cet appareil à un circuit.");
                return;
            };
            ui.small("Le changement déplace ensemble l’interrupteur et les lampes qu’il commande.");
            (Some(link.circuit_index), Some(link.switch_index))
        };

        let mut target = current;
        egui::ComboBox::from_id_salt(("component_circuit", index))
            .selected_text(current.and_then(|i| self.plan.circuit(i))
                .map_or_else(|| "Sans affectation".into(), |c| format!("Circuit {}", c.letter)))
            .show_ui(ui, |ui| {
                if kind == ElectricalKind::Outlet {
                    ui.selectable_value(&mut target, None, "Sans affectation");
                }
                for i in 0..self.plan.circuit_count() {
                    if let Some(circuit) = self.plan.circuit(i) {
                        ui.selectable_value(&mut target, Some(i), format!("Circuit {}", circuit.letter));
                    }
                }
            });
        if target != current {
            let mut updated = self.plan.clone();
            let result = match (command, target) {
                (Some(switch), Some(circuit)) => updated.move_lighting_group(switch, circuit),
                (None, Some(circuit)) => updated.assign_outlet_to_circuit(index, circuit),
                (None, None) => {
                    updated.unassign_outlet(index);
                    Ok(())
                }
                (Some(_), None) => return,
            };
            match result {
                Ok(()) => {
                    self.snapshot();
                    self.plan = updated;
                    self.status = match target.and_then(|i| self.plan.circuit(i)) {
                        Some(circuit) => format!("Affectation mise à jour : circuit {}", circuit.letter),
                        None => "Affectation de la prise retirée".into(),
                    };
                }
                Err(error) => self.status = error.into(),
            }
        }
    }

    fn export_unifilar_pdf_dialog(&mut self) {
        if let Some(path) = Self::file_picker(
            &self.unifilar_pdf_path, "Exporter le schema unifilaire", "Document PDF", "pdf"
        ).save_file() {
            self.unifilar_pdf_path = path.with_extension("pdf").to_string_lossy().into_owned();
            self.export_unifilar_pdf();
        }
    }

    fn export_pdf_dialog(&mut self) {
        if let Some(path) = Self::file_picker(
            &self.pdf_path, "Exporter le plan en PDF", "Document PDF", "pdf"
        ).save_file() {
            self.pdf_path = path.with_extension("pdf").to_string_lossy().into_owned();
            self.export_pdf();
        }
    }

    fn export_unifilar_pdf(&mut self) {
        self.status = match pdf_export::export_unifilar_pdf(&self.unifilar_pdf_path, &self.plan) {
            Ok(()) => format!("PDF unifilaire exporte : {}", self.unifilar_pdf_path),
            Err(error) => error,
        };
    }

    fn export_pdf(&mut self) {
        let sheets = self.sheets.iter().enumerate().map(|(index, sheet)| {
            let plan = if index == self.active_sheet { &self.plan } else { &sheet.plan };
            (sheet.name.as_str(), plan)
        });
        self.status = match pdf_export::export_plan_sheets(&self.pdf_path, sheets, self.settings.show_dimensions) {
            Ok(()) => format!("PDF exporté ({} feuille(s)) : {}", self.sheets.len(), self.pdf_path),
            Err(error) => error,
        };
    }

    fn finish_placement(&mut self) {
        self.tool = Tool::SelectWall;
        self.reset_pending();
        self.clear_selection();
        self.sidebar_tab = SidebarTab::Inspecteur;
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
                            thickness_m: self.settings.new_wall_thickness_cm / 100.0,
                        });
                        self.finish_placement();
                        self.selected_wall = Some(self.plan.walls.len() - 1);
                        self.status = format!(
                            "Mur de {distance:.2} m ajoute. Mode Selection actif : deplace ou modifie le mur."
                        );
                    }
                } else {
                    self.pending_wall = Some(position);
                    self.status = "Deuxieme point : vert = horizontal/vertical, Maj = angle droit, clic droit = deplacer la vue".into();
                }
            }
            Tool::Stair => {
                if let Some(start) = self.pending_stair.take() {
                    let stair = Stair {
                        start,
                        end: position,
                        width_m: self.settings.new_stair_width_cm / 100.0,
                        steps: self.settings.new_stair_steps,
                    };
                    if stair.is_valid() {
                        self.snapshot();
                        self.plan.stairs.push(stair);
                        self.finish_placement();
                        self.selected_stair = Some(self.plan.stairs.len() - 1);
                        self.status = "Escalier ajoute (fleche dans le sens de la montee)".into();
                    } else {
                        self.status = "L'escalier doit mesurer au moins 5 cm".into();
                    }
                } else {
                    self.pending_stair = Some(position);
                    self.status = "Clique le deuxieme point pour definir la montee".into();
                }
            }
            Tool::Door | Tool::Window | Tool::Passage => {
                let previous_rooms = crate::rooms::detect_rooms(&self.plan.walls).len();
                // Les passages peuvent relier deux troncons ou une cloison a
                // un mur en T. Tester d'abord ce raccord VIRTUEL : sinon
                // l'outil trouvait souvent le bout du mur voisin et y posait
                // une ouverture qui ne fermait aucune piece.
                let gap_candidate = if self.tool == Tool::Passage {
                    let mut preview = self.plan.clone();
                    preview.bridge_gap_with_passage(position).ok().map(|width| {
                        let room_count = crate::rooms::detect_rooms(&preview.walls).len();
                        (preview, width, room_count)
                    })
                } else { None };
                if gap_candidate.as_ref().is_some_and(|(_, _, count)| *count > previous_rooms) {
                    let (updated, width, rooms) = gap_candidate.unwrap();
                    self.snapshot();
                    self.plan = updated;
                    self.finish_placement();
                    self.selected_opening = Some(self.plan.openings.len() - 1);
                    self.status = format!(
                        "Passage libre de {:.2} m : {} pieces reconnues (separation invisible)",
                        width, rooms
                    );
                    return;
                }
                if let Some(wall_index) = self.find_wall(position, ppm) {
                    let kind = match self.tool {
                        Tool::Door => OpeningKind::Door,
                        Tool::Window => OpeningKind::Window,
                        _ => OpeningKind::Passage,
                    };
                    let width_m = match self.tool {
                        Tool::Door => self.settings.new_door_width_cm / 100.0,
                        Tool::Window => self.settings.new_window_width_cm / 100.0,
                        _ => self.settings.new_passage_width_cm / 100.0,
                    };
                    // On valide avant de creer une entree dans l'historique d'annulation.
                    let mut updated = self.plan.clone();
                    match updated.add_opening(
                        wall_index,
                        position,
                        width_m,
                        kind,
                        self.settings.door_hinge_right,
                        self.settings.door_swing_flipped,
                        self.settings.snap_m,
                    ) {
                        Ok(()) => {
                            self.snapshot();
                            self.plan = updated;
                            self.finish_placement();
                            self.selected_opening = Some(self.plan.openings.len() - 1);
                            self.status = match kind {
                                OpeningKind::Door => format!("Porte de {:.0} cm ajoutee", width_m * 100.0),
                                OpeningKind::Window => format!("Fenetre de {:.0} cm ajoutee", width_m * 100.0),
                                OpeningKind::Passage => {
                                    let room_count = crate::rooms::detect_rooms(&self.plan.walls).len();
                                    if room_count == 0 {
                                        format!(
                                            "Passage libre ajoute ({:.0} cm), mais aucun contour ferme : raccorde les murs autour de la piece",
                                            width_m * 100.0
                                        )
                                    } else {
                                        format!(
                                            "Passage libre ajoute ({:.0} cm) : {} piece(s) conservee(s)",
                                            width_m * 100.0, room_count
                                        )
                                    }
                                }
                            };
                        }
                        Err(err) => self.status = err.into(),
                    }
                } else if self.tool == Tool::Passage {
                    if let Some((updated, width, rooms)) = gap_candidate {
                        self.snapshot();
                        self.plan = updated;
                        self.finish_placement();
                        self.selected_opening = Some(self.plan.openings.len() - 1);
                        self.status = format!(
                            "Passage libre de {:.2} m ajoute. {} piece(s) reconnue(s) ; verifie les autres jonctions si besoin.",
                            width, rooms
                        );
                    } else {
                        self.status = "Clique dans un passage entre deux murs alignes, ou entre une cloison et un mur perpendiculaire".into();
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
                            switch_index, light_index, self.settings.active_circuit_index
                        ) {
                            Ok(()) => {
                                self.snapshot();
                                self.plan = updated;
                                self.pending_switch = None;
                                self.status = format!(
                                    "Liaison ajoutee au circuit {} : visible dans l'unifilaire.",
                                    self.plan.circuit(self.settings.active_circuit_index)
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
                        self.plan.circuit(self.settings.active_circuit_index)
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
                    match self.plan.assign_outlet_to_circuit(new_index, self.settings.active_circuit_index) {
                        Ok(()) => self.status = "Prise ajoutee au circuit actif".into(),
                        Err(err) => self.status = err.into(),
                    }
                } else {
                    self.status = "Symbole ajoute".into();
                }
                self.finish_placement();
                self.selected_symbol = Some(new_index);
            }
            Tool::SelectWall => self.select_at(position, ppm),
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(249, 250, 252));

        // La navigation reste disponible dans TOUS les outils, y compris
        // Selection et pendant un trace : clic droit maintenu + glisser.
        // Ne touche ni aux objets selectionnes ni au plan en cours.
        if response.dragged_by(egui::PointerButton::Secondary) {
            self.settings.pan = (Vec2::from(self.settings.pan) + response.drag_delta()).into();
        }

        if response.hovered() {
            let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                if let Some(cursor) = response.hover_pos() {
                    let old_zoom = self.settings.zoom;
                    let new_zoom = (old_zoom * (scroll * 0.002).exp()).clamp(0.3, 4.0);
                    // Garder le point sous la souris fixe durant le zoom.
                    let center = rect.center();
                    let old_origin = center + Vec2::from(self.settings.pan);
                    self.settings.pan = (cursor - center - (cursor - old_origin) * (new_zoom / old_zoom)).into();
                    self.settings.zoom = new_zoom;
                }
            }
        }

        let ppm = PIXELS_PER_METER * self.settings.zoom;
        let origin = rect.center() + Vec2::from(self.settings.pan);
        let to_screen = |p: Point| -> Pos2 {
            Pos2::new(origin.x + p.x * ppm, origin.y + p.y * ppm)
        };
        let to_world = |p: Pos2| -> Point {
            Point { x: (p.x - origin.x) / ppm, y: (p.y - origin.y) / ppm }
        };

        // Calque image sous les murs, cotes et symboles ; dimensions en metres.
        if self.settings.show_underlay {
            if let (Some(bg), Some(texture)) = (
                self.plan.background.as_ref(), self.underlay_texture.as_ref()
            ) {
                let aspect = self.underlay_dimensions[1] as f32
                    / self.underlay_dimensions[0].max(1) as f32;
                let at = to_screen(bg.origin);
                let size = Vec2::new(bg.width_m * ppm, bg.width_m * aspect * ppm);
                let underlay_rect = egui::Rect::from_min_size(at, size);
                if underlay_rect.intersects(rect) {
                    painter.image(
                        texture.id(),
                        underlay_rect,
                        egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_white_alpha((bg.opacity.clamp(0.0, 1.0) * 255.0) as u8),
                    );
                }
            }
        }

        if self.settings.grid {
            let step = GRID_METERS * ppm;
            let left = ((rect.left() - origin.x) / step).floor() as i32;
            let right = ((rect.right() - origin.x) / step).ceil() as i32;
            let top = ((rect.top() - origin.y) / step).floor() as i32;
            let bottom = ((rect.bottom() - origin.y) / step).ceil() as i32;
            for i in left.max(-200)..=right.min(200) {
                let x = origin.x + i as f32 * step;
                let color = if i == 0 { Color32::GRAY } else { Color32::from_gray(225) };
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(1.0_f32, color));
            }
            for i in top.max(-200)..=bottom.min(200) {
                let y = origin.y + i as f32 * step;
                let color = if i == 0 { Color32::GRAY } else { Color32::from_gray(225) };
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(1.0_f32, color));
            }
        }

        if self.tool == Tool::SelectWall {
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.marquee_handles = ui.ctx().input(|i| i.modifiers.shift);
                let initial = ui.ctx().input(|i| i.pointer.press_origin());
                if let Some(pos) = initial.or(response.interact_pointer_pos()) {
                    self.begin_drag(to_world(pos), ppm);
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(cursor) = response.interact_pointer_pos() {
                    if self.marquee.is_some() {
                        self.update_marquee(to_world(cursor));
                    } else {
                        self.update_drag(to_world(cursor), ppm);
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(cursor) = response.interact_pointer_pos() {
                    if self.marquee.is_some() {
                        self.update_marquee(to_world(cursor));
                    } else {
                        self.update_drag(to_world(cursor), ppm);
                    }
                }
                if self.marquee.is_some() { self.finish_marquee(); }
                else { self.finish_drag(); }
            }
        }

        // Seul le clic GAUCHE pose un objet : le clic droit est reserve au pan.
        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let world = to_world(pos);
                if self.tool == Tool::SelectWall
                    && ui.ctx().input(|i| i.modifiers.shift)
                    && self.toggle_wall_handles(world, ppm)
                {
                    // Maj + clic : ne pas reinitialiser les autres sommets.
                } else {
                let point = match self.tool {
                    Tool::SelectWall | Tool::Door | Tool::Window | Tool::Passage | Tool::LinkLighting => world,
                    Tool::Wall => {
                        let candidate = self.snap_point(world, ppm);
                        if let Some(start) = self.pending_wall {
                            let shift = ui.ctx().input(|i| i.modifiers.shift);
                            aligned_wall_end(start, candidate, shift).0
                        } else {
                            candidate
                        }
                    }
                    _ => world.snapped(self.settings.snap_m),
                };
                self.place(point, ppm);
                }
            }
        }

        if self.settings.show_architecture {
        if self.settings.show_rooms {
            let detected_rooms = rooms::detect_rooms(&self.plan.walls);
            for room in &detected_rooms {
                let points: Vec<Pos2> = room.inner_polygon.iter().map(|&p| to_screen(p)).collect();
                let selected = self.selected_room.is_some_and(|at| room.contains(at));
                if points.len() >= 3 {
                    painter.add(egui::Shape::closed_line(
                        points,
                        Stroke::new(
                            if selected { 2.8_f32 } else { 1.0_f32 },
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
                            self.selected_wall == Some(index) || self.selection.walls.contains(&index),
                        );
                    }
                    draw_opening(&painter, a, direction, width_px, ppm, opening);
                    if self.selected_opening == Some(opening_index)
                        || self.selection.openings.contains(&opening_index) {
                        let handle = a + direction * (opening.center_m * ppm);
                        painter.circle_filled(handle, 5.0, Color32::from_rgb(242, 178, 73));
                        painter.circle_stroke(
                            handle, 8.0,
                            Stroke::new(2.0_f32, Color32::from_rgb(40, 135, 230)),
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
                        self.selected_wall == Some(index) || self.selection.walls.contains(&index),
                    );
                }
            }
            if self.settings.show_dimensions {
                draw_dimension(&painter, a, b, width_px, wall.length_m());
            }
            if self.tool == Tool::SelectWall {
                // Les sommets doivent etre visibles meme avant leur selection.
                for endpoint in [a, b] {
                    painter.circle_filled(endpoint, 3.0, Color32::from_rgb(94, 151, 210));
                }
            }
            if self.selected_wall == Some(index) || self.selection.walls.contains(&index) {
                for endpoint in [a, b] {
                    painter.circle_filled(endpoint, 6.0, Color32::WHITE);
                    painter.circle_stroke(
                        endpoint, 6.0,
                        Stroke::new(2.5_f32, Color32::from_rgb(40, 135, 230)),
                    );
                }
            }
        }

        // Une poignee ambre represente un ou plusieurs sommets selectionnes
        // exactement au meme point, en particulier les jonctions des murs.
        let mut drawn_nodes: Vec<Point> = Vec::new();
        for handle in &self.selected_handles {
            let Some(point) = handle.position(&self.plan) else { continue };
            if drawn_nodes.iter().any(|&previous| previous.distance_to(point) < 0.0001) {
                continue;
            }
            drawn_nodes.push(point);
            let pos = to_screen(point);
            painter.circle_filled(pos, 7.0, Color32::from_rgb(250, 187, 62));
            painter.circle_stroke(
                pos, 9.0,
                Stroke::new(2.0_f32, Color32::from_rgb(166, 98, 9))
            );
            let linked = self.selected_handles.iter()
                .filter(|h| h.position(&self.plan)
                    .is_some_and(|p| p.distance_to(point) < 0.0001))
                .count();
            if linked > 1 {
                painter.text(
                    pos + Vec2::new(12.0, -12.0), Align2::LEFT_CENTER,
                    format!("{linked} pts"), FontId::proportional(12.0),
                    Color32::from_rgb(125, 78, 7),
                );
            }
        }

        for (index, stair) in self.plan.stairs.iter().enumerate() {
            draw_stair(&painter, stair, origin, ppm);
            if (self.selected_stair == Some(index) || self.selection.stairs.contains(&index))
                && stair.is_valid() {
                let corners = stair.corners().map(to_screen);
                for j in 0..4 {
                    painter.line_segment(
                        [corners[j], corners[(j + 1) % 4]],
                        Stroke::new(3.5_f32, Color32::from_rgb(40, 135, 230))
                    );
                }
            }
        }
        } // fin du calque architectural

        if self.settings.show_electrical && self.settings.circuit_visibility != CircuitVisibility::None {
        for link in &self.plan.lighting_connections {
            if self.settings.circuit_visibility == CircuitVisibility::Active
                && link.circuit_index != self.settings.active_circuit_index { continue; }
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
            if !self.symbol_visible_on_plan(index) { continue; }
            let pos = to_screen(symbol.position);
            let label = match symbol.kind {
                ElectricalKind::Outlet => "P",
                ElectricalKind::Switch => "I",
                ElectricalKind::Light => "L",
            };
            // Formes RGIE uniques partagees avec l'unifilaire et les PDFs.
            rgie_symbols::draw_egui(
                &painter, pos,
                &rgie_symbols::device_primitives(
                    symbol.kind, self.plan.symbol_appearance(index)
                ),
                Color32::from_rgb(20, 106, 190),
                0.83,
            );
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
                pos + Vec2::new(0.0, 30.0),
                Align2::CENTER_CENTER,
                format!("{label}{number}{ref_suffix}"),
                FontId::proportional(12.0),
                Color32::from_rgb(35, 77, 123),
            );
            if self.pending_switch == Some(index) {
                painter.circle_stroke(pos, 17.0, Stroke::new(2.0_f32, Color32::from_rgb(233, 139, 40)));
            }
            if self.selected_symbol == Some(index) || self.selection.symbols.contains(&index) {
                painter.circle_stroke(pos, 24.0, Stroke::new(3.0_f32, Color32::from_rgb(40, 135, 230)));
            }
        }
        } // fin du calque electrique

        if self.settings.show_architecture {
        if let Some(start) = self.pending_stair {
            if let Some(cursor) = response.hover_pos() {
                let end = to_world(cursor).snapped(self.settings.snap_m);
                let preview = Stair {
                    start, end,
                    width_m: self.settings.new_stair_width_cm / 100.0,
                    steps: self.settings.new_stair_steps,
                };
                draw_stair(&painter, &preview, origin, ppm);
            }
        }

        if let Some(start) = self.pending_wall {
            if let Some(cursor) = response.hover_pos() {
                let candidate = self.snap_point(to_world(cursor), ppm);
                let shift = ui.ctx().input(|i| i.modifiers.shift);
                let (end, orientation) = aligned_wall_end(start, candidate, shift);
                let width_px = self.settings.new_wall_thickness_cm / 100.0 * ppm;
                let (stroke, label_color) = if orientation.is_some() {
                    (
                        Stroke::new(width_px.max(1.0), Color32::from_rgb(27, 162, 106)),
                        Color32::from_rgb(20, 125, 80),
                    )
                } else {
                    (
                        Stroke::new(width_px.max(1.0), Color32::from_rgb(40, 130, 215)),
                        Color32::from_rgb(38, 104, 178),
                    )
                };
                painter.line_segment([to_screen(start), to_screen(end)], stroke);
                // Axes de construction et marqueur d'equerre verts sur mur droit.
                if let Some(axis) = orientation {
                    let guide = Stroke::new(1.0_f32, Color32::from_rgb(66, 183, 131));
                    match axis {
                        WallAlignment::Horizontal => {
                            let y = to_screen(start).y;
                            painter.line_segment(
                                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], guide
                            );
                        }
                        WallAlignment::Vertical => {
                            let x = to_screen(start).x;
                            painter.line_segment(
                                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], guide
                            );
                        }
                    }
                    painter.circle_stroke(to_screen(end), 6.0, Stroke::new(2.0_f32, label_color));
                }
                painter.text(
                    to_screen(end) + Vec2::new(15.0, -22.0),
                    Align2::LEFT_CENTER,
                    wall_heading(start, end, orientation),
                    FontId::proportional(13.0),
                    label_color,
                );
                if self.settings.show_dimensions {
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

        if let Some(marquee) = self.marquee {
            let from = to_screen(marquee.start);
            let to = to_screen(marquee.current);
            let bounds = egui::Rect::from_two_pos(from, to).intersect(rect);
            painter.rect_filled(bounds, 0.0, Color32::from_rgba_unmultiplied(36, 130, 225, 32));
            let stroke = Stroke::new(1.5_f32, Color32::from_rgb(35, 115, 215));
            for (a, b) in [
                (bounds.left_top(), bounds.right_top()),
                (bounds.right_top(), bounds.right_bottom()),
                (bounds.right_bottom(), bounds.left_bottom()),
                (bounds.left_bottom(), bounds.left_top()),
            ] {
                painter.line_segment([a, b], stroke);
            }
        }

        painter.text(
            rect.left_top() + Vec2::new(12.0, 12.0),
            Align2::LEFT_TOP,
            format!(
                "Grille : 0,5 m | Pas : {:.0} cm | Zoom : {:.0} %",
                self.settings.snap_m * 100.0,
                self.settings.zoom * 100.0
            ),
            FontId::proportional(13.0),
            Color32::DARK_GRAY,
        );
    }
}

impl eframe::App for EasyDraft {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.sheet_name_dialog.is_none() && !ctx.wants_keyboard_input() {
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
        // Barre globale : navigation principale et actions de fichier, sans melanger
        // les outils de construction avec les exports.
        egui::TopBottomPanel::top("workspace_top")
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.heading(egui::RichText::new("EasyDraft").color(Color32::from_rgb(28, 88, 186)));
                    ui.separator();
                    ui.small("PLAN TYPE");
                    ui.separator();
                    if ui.selectable_label(self.view == View::Unifilar, "Schema unifilaire").clicked() {
                        self.view = View::Unifilar;
                        self.sidebar_tab = SidebarTab::Electricite;
                    }
                    if ui.selectable_label(self.view == View::Plan2D, "Plan de situation").clicked() {
                        self.view = View::Plan2D;
                        self.sidebar_tab = SidebarTab::Affichage;
                    }
                    if ui.selectable_label(self.view == View::RgieReview, "Dossier RGIE").clicked() {
                        self.view = View::RgieReview;
                        self.sidebar_tab = SidebarTab::Dossier;
                    }
                    ui.separator();
                    self.sheet_controls(ui);
                    ui.separator();
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau plan").clicked() {
                            self.replace_with_single_plan(Plan::default());
                            self.file_path_assigned = false;
                            self.file_path = "mon-plan.json".into();
                            self.view = View::Plan2D;
                            self.sidebar_tab = SidebarTab::Affichage;
                            self.status = "Nouveau plan".into();
                            ui.close();
                        }
                        if ui.button("Ouvrir...").clicked() { self.open_dialog(); ui.close(); }
                        if ui.button("Enregistrer").clicked() { self.save(); ui.close(); }
                        if ui.button("Enregistrer sous...").clicked() { self.save_as(); ui.close(); }
                        if ui.button("Chemins des fichiers").clicked() {
                            self.sidebar_tab = SidebarTab::Fichiers;
                            ui.close();
                        }
                    });
                    ui.menu_button("Edition", |ui| {
                        if ui.button("Annuler  Ctrl+Z").clicked() { self.undo(); ui.close(); }
                        if ui.button("Retablir  Ctrl+Y").clicked() { self.redo(); ui.close(); }
                        if self.view == View::Plan2D && ui.button("Supprimer  Suppr").clicked() {
                            self.delete_selected();
                            ui.close();
                        }
                    });
                    ui.menu_button("Exporter", |ui| {
                        if ui.button("Plan de situation en PDF (toutes les feuilles)…").clicked() {
                            self.export_pdf_dialog(); ui.close();
                        }
                        if ui.button("Schéma unifilaire en PDF (feuille active)…").clicked() {
                            self.export_unifilar_pdf_dialog(); ui.close();
                        }
                    });
                    if ui.button("Enregistrer").clicked() { self.save(); }
                });
                ui.add_space(5.0);
                ui.separator();
                ui.horizontal(|ui| {
                    let caption = match self.view {
                        View::Plan2D => "PLAN DE SITUATION",
                        View::Unifilar => "SCHEMA UNIFILAIRE",
                        View::RgieReview => "DOSSIER / CONTROLES",
                    };
                    ui.strong(caption);
                    ui.separator();
                    if self.view == View::Plan2D {
                        if ui.selectable_label(self.tool == Tool::SelectWall, "Selection").clicked() {
                            self.select_tool(Tool::SelectWall);
                            self.sidebar_tab = SidebarTab::Inspecteur;
                        }
                        let join_enabled = !self.selected_handles.is_empty();
                        if ui.add_enabled(join_enabled, egui::Button::new("Joindre")).clicked() {
                            self.weld_selected_to_first();
                        }
                        let dissolve_enabled = self.plan.wall_joints.iter()
                            .any(|joint| joint.endpoints.iter()
                                .any(|endpoint| self.selected_handles.contains(endpoint)));
                        if ui.add_enabled(dissolve_enabled, egui::Button::new("Dissoudre")).clicked() {
                            self.dissolve_selected_joints();
                        }
                        ui.small("Clic droit + glisser : naviguer");
                        if ui.button("Recentrer").clicked() {
                            self.settings.pan = [0.0; 2];
                            self.settings.zoom = 1.0;
                        }
                        ui.separator();
                        ui.checkbox(&mut self.settings.grid, "Grille");
                        ui.checkbox(&mut self.settings.show_dimensions, "Cotes");
                        ui.separator();
                        ui.small(format!("Zoom {:.0} %", self.settings.zoom * 100.0));
                        ui.separator();
                        ui.strong(format!(
                            "{} piece(s) detectee(s)",
                            rooms::detect_rooms(&self.plan.walls).len()
                        ));
                    } else if self.view == View::Unifilar {
                        ui.label(format!(
                            "{} circuit(s)  •  {} differentiel(s)  •  {} appareil(s)",
                            self.plan.circuit_count(),
                            self.plan.differentials.len(),
                            self.plan.symbols.len()
                        ));
                    } else {
                        ui.small("Verification des informations saisies - conformite non certifiee");
                    }
                });
                ui.add_space(4.0);
            });

        // Bibliotheque de composants, comparable a une palette CAO : toujours
        // distincte de l'inspecteur et du grand espace central.
        egui::SidePanel::left("component_library")
            .resizable(true)
            .default_width(220.0).min_width(180.0).max_width(325.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("COMPOSANTS");
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut self.palette_search)
                    .hint_text("Rechercher un outil..."));
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false]).show(ui, |ui| {
                    if self.view == View::Plan2D {
                        let families: [(&str, &[(Tool, &str)]); 3] = [
                            ("OUTILS DE DESSIN", &[
                                (Tool::SelectWall, "Selection / cadre"),
                                (Tool::Wall, "Mur droit"),
                             ]),
                            ("CONSTRUCTION", &[
                                (Tool::Door, "Porte"),
                                (Tool::Window, "Fenetre"),
                                (Tool::Passage, "Passage libre"),
                                (Tool::Stair, "Escalier"),
                            ]),
                            ("ELECTRICITE", &[
                                (Tool::Outlet, "Prise de courant"),
                                (Tool::Switch, "Interrupteur"),
                                (Tool::Light, "Point lumineux"),
                                (Tool::LinkLighting, "Lier interrupteur / lampe"),
                            ]),
                        ];
                        for (family, items) in families {
                            if let Some(tool) = palette_section(
                                ui, family, self.tool, &self.palette_search, items
                            ) {
                                self.select_tool(tool);
                                self.sidebar_tab = match tool {
                                    Tool::SelectWall => SidebarTab::Inspecteur,
                                    Tool::Outlet | Tool::Switch | Tool::Light | Tool::LinkLighting =>
                                        SidebarTab::Electricite,
                                     _ => SidebarTab::Dessin,
                                };
                            }
                            ui.add_space(4.0);
                        }
                        ui.separator();
                        ui.small("Clic gauche : selectionner ou encadrer.");
                        ui.small("Maj + clic sur un sommet : selectionner plusieurs extremites.");
                        ui.small("Maj + rectangle : choisir plusieurs sommets de murs.");
                        ui.small("Glisser : deplacer les points sans les lier.");
                        ui.small("Joindre / Dissoudre : commandes explicites en haut et dans Objet.");
                        ui.small("Clic droit + glisser : deplacer la vue, sans changer d'outil.");
                        ui.small("Mur : repere vert = horizontal / vertical ; Maj verrouille l'axe.");
                        ui.small("Un mur termine revient automatiquement a Selection.");
                        if ui.button("Ouvrir l'exemple de 2 pieces").clicked() {
                            self.load_room_example();
                        }
                    } else if self.view == View::Unifilar {
                        ui.strong("CIRCUITS");
                        for i in 0..self.plan.circuit_count() {
                            let name = self.plan.circuit(i)
                                .map(|c| c.letter.as_str()).unwrap_or("?");
                            if ui.add_sized(
                                [ui.available_width(), 29.0],
                                egui::Button::new(format!("Circuit {name}"))
                                    .selected(self.settings.active_circuit_index == i)
                            ).clicked() {
                                self.settings.active_circuit_index = i;
                                self.sidebar_tab = SidebarTab::Electricite;
                            }
                        }
                        if ui.button("+ Ajouter un circuit").clicked() {
                            let mut updated = self.plan.clone();
                            match updated.add_lighting_circuit() {
                                Ok(i) => {
                                    self.snapshot();
                                    self.plan = updated;
                                    self.settings.active_circuit_index = i;
                                    self.status = "Nouveau circuit ajoute".into();
                                }
                                Err(error) => self.status = error.into(),
                            }
                        }
                        ui.separator();
                        ui.strong("APPAREILS");
                        ui.small("Les appareils du schema sont derives du plan de situation.");
                        for (name, tool) in [
                            ("Prise de courant", Tool::Outlet),
                            ("Interrupteur", Tool::Switch),
                            ("Point lumineux", Tool::Light),
                        ] {
                            let query = self.palette_search.trim().to_lowercase();
                            if !query.is_empty() && !name.to_lowercase().contains(&query) {
                                continue;
                            }
                            if ui.add_sized([ui.available_width(), 30.0], egui::Button::new(name)).clicked() {
                                self.view = View::Plan2D;
                                self.select_tool(tool);
                                self.sidebar_tab = SidebarTab::Electricite;
                                self.status = format!("Place {name} sur le plan, il apparaitra dans l'unifilaire");
                            }
                        }
                        ui.separator();
                        if ui.button("Charger l'exemple unifilaire A + B").clicked() {
                            self.load_unifilar_example();
                        }
                    } else {
                        ui.strong("DOSSIER");
                        ui.label("Les controles proviennent du modele du plan et des circuits.");
                        if ui.button("Configurer l'installation").clicked() {
                            self.sidebar_tab = SidebarTab::Dossier;
                        }
                        if ui.button("Configurer les circuits").clicked() {
                            self.sidebar_tab = SidebarTab::Electricite;
                        }
                    }
                });
            });

        // Inspecteur contextuel et reglages du document a droite.
        egui::SidePanel::right("property_inspector")
            .resizable(true)
            .default_width(302.0).min_width(235.0).max_width(410.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("PARAMETRES");
                ui.horizontal_wrapped(|ui| {
                    if self.view == View::Plan2D {
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Inspecteur, "Objet");
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Dessin, "Outil");
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Affichage, "Plan");
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Electricite, "Circuits");
                    } else if self.view == View::Unifilar {
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Electricite, "Circuits");
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Dossier, "Dossier");
                    } else {
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Dossier, "Dossier");
                        ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Electricite, "Circuits");
                    }
                    ui.selectable_value(&mut self.sidebar_tab, SidebarTab::Fichiers, "Export");
                });
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if self.sidebar_tab == SidebarTab::Dessin {
            ui.label("Outil actif");
            ui.heading(match self.tool {
                Tool::SelectWall => "Selection",
                Tool::Wall => "Mur",
                Tool::Door => "Porte",
                Tool::Window => "Fenetre",
                Tool::Passage => "Passage libre",
                Tool::Stair => "Escalier",
                Tool::Outlet => "Prise",
                Tool::Switch => "Interrupteur",
                Tool::Light => "Point lumineux",
                Tool::LinkLighting => "Lier I → L",
             });
            ui.small("Choisis un outil dans la barre Architecture / Electricite en haut.");
            ui.separator();
            if self.view == View::Plan2D && self.tool == Tool::Door {
                ui.label("Largeur de la porte");
                ui.add(egui::Slider::new(&mut self.settings.new_door_width_cm, 50.0..=200.0)
                    .step_by(5.0).suffix(" cm"));
                ui.checkbox(&mut self.settings.door_hinge_right, "Charniere a la fin du mur");
                ui.checkbox(&mut self.settings.door_swing_flipped, "Inverser le sens d'ouverture");
                ui.small("Clique sur un mur pour inserer la porte.");
                ui.separator();
            }
            if self.view == View::Plan2D && self.tool == Tool::Stair {
                ui.label("Largeur de l'escalier");
                ui.add(egui::Slider::new(&mut self.settings.new_stair_width_cm, 60.0..=250.0)
                    .step_by(5.0).suffix(" cm"));
                ui.label("Nombre de marches");
                ui.add(egui::Slider::new(&mut self.settings.new_stair_steps, 3..=30));
                ui.small("Deux clics : depart puis arrivee (sens de montee).");
                ui.separator();
            }
            if self.view == View::Plan2D && self.tool == Tool::Window {
                ui.label("Largeur de la fenetre");
                ui.add(egui::Slider::new(&mut self.settings.new_window_width_cm, 40.0..=400.0)
                    .step_by(5.0).suffix(" cm"));
                ui.small("Clique sur un mur pour inserer la fenetre.");
                ui.separator();
            }
            if self.view == View::Plan2D && self.tool == Tool::Passage {
                ui.label("Largeur du passage libre");
                ui.add(egui::Slider::new(&mut self.settings.new_passage_width_cm, 40.0..=400.0)
                    .step_by(5.0).suffix(" cm"));
                ui.small("Sur un mur : ouvre le passage sans porte.");
                ui.small("Dans un vide entre murs alignes : relie les 2 bouts sans mur visible.");
                ui.small("Les pieces restent distinctes pour le calcul des surfaces.");
                ui.separator();
            }
            ui.label("Epaisseur des nouveaux murs");
            ui.add(egui::Slider::new(&mut self.settings.new_wall_thickness_cm, 5.0..=60.0).text("cm").step_by(1.0));
            ui.separator();
            } // Dessin
            if self.sidebar_tab == SidebarTab::Inspecteur {
                ui.heading("Element selectionne");
                if !self.selected_handles.is_empty() {
                    ui.strong(format!("{} sommet(s) de mur", self.selected_handles.len()));
                    ui.small("Maj + clic (ou Maj + rectangle) : ajouter des points.");
                    ui.small("Glisser : deplacer ensemble SANS les joindre automatiquement.");
                    ui.small("Joindre : lier les points choisis au premier. Avec un seul point, raccorder a un mur proche (20 cm).");
                    ui.small("Dissoudre : liberer une liaison. Les murs restent en place, mais les points peuvent ensuite bouger seuls.");
                    let linked_count = self.plan.wall_joints.iter()
                        .filter(|joint| joint.endpoints.iter()
                            .any(|end| self.selected_handles.contains(end))).count();
                    if linked_count > 0 {
                        ui.label(format!("{linked_count} liaison(s) explicite(s)"));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Joindre").clicked() {
                            self.weld_selected_to_first();
                        }
                        if ui.add_enabled(linked_count > 0, egui::Button::new("Dissoudre")).clicked() {
                            self.dissolve_selected_joints();
                        }
                    });
                    if ui.button("Effacer la selection des points").clicked() {
                        self.clear_selection();
                    }
                } else if self.selection.len() <= 1 {
                    self.selected_properties(ui);
                }
                ui.separator();
                ui.small("Clic : selectionner un objet. Glisser depuis le vide : rectangle multi-selection.");
                ui.small("Rectangle gauche → droite : elements entiers ; droite → gauche : elements traverses.");
                ui.small("Glisser un element selectionne : deplacer l'ensemble. Ctrl+Z : annuler.");
                if self.selection.len() > 1 {
                    ui.separator();
                    ui.strong(format!("{} elements selectionnes", self.selection.len()));
                    ui.label(format!(
                        "{} mur(s), {} ouverture(s), {} symbole(s), {} escalier(s)",
                        self.selection.walls.len(), self.selection.openings.len(),
                        self.selection.symbols.len(), self.selection.stairs.len()
                    ));
                    if ui.button("Supprimer le groupe").clicked() { self.delete_selected(); }
                }
            }
            if self.sidebar_tab == SidebarTab::Affichage {
            ui.strong("FOND DE PLAN");
            if ui.button("Importer une image...").clicked() {
                self.import_underlay(ui.ctx());
            }
            if let Some(current) = self.plan.background.as_ref().cloned() {
                ui.checkbox(&mut self.settings.show_underlay, "Afficher le fond de plan");
                ui.small("Fichier image externe au JSON");
                ui.label(current.path.rsplit(['/', '\\']).next().unwrap_or("image"));
                let mut edited = current.clone();
                ui.label("Largeur reelle de l'image");
                ui.add(egui::Slider::new(&mut edited.width_m, 2.0..=80.0)
                    .step_by(0.1).suffix(" m"));
                ui.label("Transparence");
                ui.add(egui::Slider::new(&mut edited.opacity, 0.0..=1.0)
                    .step_by(0.01).text("Opacite"));
                ui.label("Position (metres)");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut edited.origin.x).speed(0.05).prefix("X "));
                    ui.add(egui::DragValue::new(&mut edited.origin.y).speed(0.05).prefix("Y "));
                });
                ui.horizontal(|ui| {
                    if ui.button("Pivoter de 90 degres").clicked() {
                        edited.rotation_quarters = (edited.rotation_quarters + 1) % 4;
                    }
                    if ui.button("Retirer").clicked() {
                        self.snapshot();
                        self.plan.background = None;
                        self.underlay_texture = None;
                        self.underlay_loaded_key = None;
                        self.status = "Fond de plan retire".into();
                    }
                });
                if self.plan.background.is_some() && edited != current {
                    self.snapshot();
                    self.plan.background = Some(edited);
                }
            } else {
                ui.small("PNG, JPG ou BMP comme calque sous les symboles.");
            }
            ui.separator();
            ui.strong("AFFICHAGE");
            ui.label("Calques visibles");
            ui.checkbox(&mut self.settings.show_architecture, "Architecture (murs, portes, escaliers)");
            ui.checkbox(&mut self.settings.show_rooms, "Pieces, surfaces et dimensions");
            ui.checkbox(&mut self.settings.show_electrical, "Electricite (symboles et liaisons)");
            ui.label("Lignes de circuit visibles");
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.settings.circuit_visibility, CircuitVisibility::All, "Toutes");
                ui.selectable_value(&mut self.settings.circuit_visibility, CircuitVisibility::Active, "Circuit actif");
                ui.selectable_value(&mut self.settings.circuit_visibility, CircuitVisibility::None, "Aucune");
            });
            if self.settings.circuit_visibility == CircuitVisibility::Active {
                egui::ComboBox::from_label("Circuit actif")
                    .selected_text(self.plan.circuit(self.settings.active_circuit_index)
                        .map(|c| c.letter.as_str()).unwrap_or("?"))
                    .show_ui(ui, |ui| {
                        for i in 0..self.plan.circuit_count() {
                            let letter = self.plan.circuit(i)
                                .map(|c| c.letter.as_str()).unwrap_or("?");
                            ui.selectable_value(&mut self.settings.active_circuit_index, i, format!("Circuit {letter}"));
                        }
                    });
            }
            ui.checkbox(&mut self.settings.show_dimensions, "Afficher les cotes (axe des murs)");
            ui.checkbox(&mut self.settings.snap_endpoints, "Aimantation aux extremites de murs");
            ui.label("Pas d'aimantation");
            ui.add(egui::Slider::new(&mut self.settings.snap_m, 0.05..=0.5).step_by(0.05).suffix(" m"));
            ui.separator();
            ui.label("Zoom");
            ui.add(egui::Slider::new(&mut self.settings.zoom, 0.3..=4.0));
            ui.small("Molette sur le plan : zoom centre sous le pointeur.");
            ui.checkbox(&mut self.settings.grid, "Afficher la grille");
            if ui.button("Recentrer").clicked() {
                self.settings.pan = [0.0; 2];
                self.settings.zoom = 1.0;
            }
            ui.separator();
            } // Affichage
            if self.sidebar_tab == SidebarTab::Fichiers {
            ui.label("Fichier JSON");
            if ui.text_edit_singleline(&mut self.file_path).changed() {
                self.file_path_assigned = false;
            }
            ui.horizontal(|ui| {
                if ui.button("Ouvrir...").clicked() { self.open_dialog(); }
                if ui.button("Enregistrer sous...").clicked() { self.save_as(); }
            });
            ui.separator();
            ui.label("PDF du plan de situation · toutes les feuilles");
            ui.text_edit_singleline(&mut self.pdf_path);
            if ui.button("Exporter le plan PDF...").clicked() { self.export_pdf_dialog(); }
            ui.separator();
            ui.label("PDF unifilaire · feuille active");
            ui.text_edit_singleline(&mut self.unifilar_pdf_path);
            if ui.button("Exporter l'unifilaire PDF...").clicked() {
                self.export_unifilar_pdf_dialog();
            }
            ui.separator();
            } // Fichiers
            if self.sidebar_tab == SidebarTab::Dessin {
            ui.separator();
            egui::CollapsingHeader::new("Statistiques et exemples")
                .default_open(false).show(ui, |ui| {
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
            ui.label(format!(
                "Passages : {}",
                self.plan.openings.iter().filter(|o| o.kind == OpeningKind::Passage).count()
            ));
            ui.label(format!("Escaliers : {}", self.plan.stairs.len()));
            ui.label(format!("Symboles : {}", self.plan.symbols.len()));
            ui.label(format!("Liaisons I -> L : {}", self.plan.lighting_connections.len()));
            ui.label(format!("Lampes non liees : {}", self.plan.unlinked_lights()));
            ui.label(format!("Interrupteurs non lies : {}", self.plan.unlinked_switches()));
            ui.separator();
                }); // Statistiques et exemples
            } // Résumé du dessin
            if self.sidebar_tab == SidebarTab::Dossier {
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
                    ui.label("Cable d'entree du tableau (type et section)");
                    ui.text_edit_singleline(&mut info.incoming_cable);
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
            } // Dossier
            if self.sidebar_tab == SidebarTab::Electricite {
            if ui.button("Ajouter un exemple I → L").clicked() { self.add_lighting_demo(); }
            egui::CollapsingHeader::new("Tableau electrique - protections differentielles")
                .default_open(self.view == View::Unifilar)
                .show(ui, |ui| self.board_editor(ui));
            ui.separator();
            ui.heading("Circuits (eclairage / prises)");
            if ui.button("Charger l'exemple unifilaire A + B").clicked() {
                self.load_unifilar_example();
            }
            ui.small("L'exemple remplace le plan courant (Annuler pour revenir).");
            if self.settings.active_circuit_index >= self.plan.circuit_count() {
                self.settings.active_circuit_index = 0;
            }
            egui::ComboBox::from_label("Circuit actif pour lampes et prises")
                .selected_text(
                    self.plan.circuit(self.settings.active_circuit_index)
                        .map(|c| c.letter.clone()).unwrap_or_else(|| "A".into())
                )
                .show_ui(ui, |ui| {
                    for index in 0..self.plan.circuit_count() {
                        let title = format!(
                            "Circuit {}",
                            self.plan.circuit(index).map(|c| c.letter.as_str()).unwrap_or("?"),
                        );
                        ui.selectable_value(&mut self.settings.active_circuit_index, index, title);
                    }
                });
            if ui.button("Ajouter un circuit (prototype)").clicked() {
                let mut updated = self.plan.clone();
                match updated.add_lighting_circuit() {
                    Ok(index) => {
                        self.snapshot();
                        self.plan = updated;
                        self.settings.active_circuit_index = index;
                        self.pending_switch = None;
                        self.status = "Nouveau circuit ajoute et selectionne".into();
                    }
                    Err(error) => self.status = error.into(),
                }
            }
            ui.small("Les prises et liaisons I -> L sont affectees au circuit actif.");
            ui.label(format!("Prises sans circuit : {}", self.plan.unassigned_outlets()));
            let reveal_circuit = self.reveal_circuit.take();
            for circuit_index in 0..self.plan.circuit_count() {
                let Some(circuit_data) = self.plan.circuit(circuit_index).cloned() else { continue; };
                let reveal = reveal_circuit == Some(circuit_index);
                let section = egui::CollapsingHeader::new(format!(
                    "Circuit {} - protection / conducteurs", circuit_data.letter
                )).id_salt(("circuit_properties", self.active_sheet, circuit_index))
                    .open(reveal.then_some(true))
                    .default_open(circuit_index == self.settings.active_circuit_index).show(ui, |ui| {
                    let mut circuit = circuit_data.clone();
                    ui.label("Repere du circuit (A, B, C...)");
                    ui.text_edit_singleline(&mut circuit.letter);
                    ui.label("Type de protection (disjoncteur, fusible...)");
                    electrical_text(ui, &mut circuit.protection_kind, &["Disjoncteur", "Fusible"]);
                    ui.label("Protection contre surintensites - calibre");
                    electrical_number(ui, &mut circuit.breaker_amp, 0..=125, &[6, 10, 13, 16, 20, 25, 32, 40, 50, 63], " A");
                    ui.label("Nombre de poles");
                    electrical_number(ui, &mut circuit.breaker_poles, 0..=4, &[1, 2, 3, 4], "");
                    ui.label("Courbe / type protection");
                    electrical_text(ui, &mut circuit.breaker_curve, &["B", "C", "D"]);
                    let ddr_label = circuit.upstream_differential
                        .and_then(|i| self.plan.differentials.get(i))
                        .map(|d| d.description())
                        .unwrap_or_else(|| "Non attribue".to_owned());
                    ui.label("Différentiel en amont");
                    egui::ComboBox::from_id_salt(("circuit_ddr", circuit_index))
                        .selected_text(ddr_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut circuit.upstream_differential,
                                None,
                                "Aucun differentiel attribue",
                            );
                            for (d_index, ddr) in self.plan.differentials.iter().enumerate() {
                                ui.selectable_value(
                                    &mut circuit.upstream_differential,
                                    Some(d_index),
                                    ddr.description(),
                                );
                            }
                        });
                    if !circuit.upstream_rdc.trim().is_empty() {
                        ui.colored_label(
                            Color32::from_rgb(163, 102, 27),
                            format!(
                                "Ancienne note libre (non connectee au tableau) : {}",
                                circuit.upstream_rdc
                            ),
                        );
                    }
                    ui.label("Type de canalisation");
                    electrical_text(ui, &mut circuit.cable_type, &["VOB", "XVB", "XGB", "EXVB", "H07RN-F"]);
                    ui.label("Nombre de conducteurs");
                    electrical_number(ui, &mut circuit.conductor_count, 0..=10, &[2, 3, 4, 5], "");
                    ui.label("Section des conducteurs");
                    electrical_number(ui, &mut circuit.section_mm2, 0.0..=35.0,
                        &[1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0, 35.0], " mm²");
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
                    let missing = self.plan.circuit_missing_fields(circuit_index);
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
                                self.settings.active_circuit_index = 0;
                                self.pending_switch = None;
                                self.status = "Circuit vide supprime".into();
                            }
                            Err(error) => self.status = error.into(),
                        }
                    }
                });
                if reveal {
                    section.header_response.scroll_to_me(Some(egui::Align::Min));
                }
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
            egui::CollapsingHeader::new("Legende RGIE - symboles disponibles")
                .show(ui, |ui| {
                    for (kind, style) in [
                        (ElectricalKind::Switch, ElectricalAppearance::General),
                        (ElectricalKind::Switch, ElectricalAppearance::SwitchBipolar),
                        (ElectricalKind::Switch, ElectricalAppearance::SwitchTwoWay),
                        (ElectricalKind::Switch, ElectricalAppearance::PushButton),
                        (ElectricalKind::Outlet, ElectricalAppearance::General),
                        (ElectricalKind::Outlet, ElectricalAppearance::OutletEarth),
                        (ElectricalKind::Outlet, ElectricalAppearance::OutletEarthChild),
                        (ElectricalKind::Light, ElectricalAppearance::General),
                        (ElectricalKind::Light, ElectricalAppearance::WallLight),
                    ] {
                        ui.horizontal(|ui| {
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(65.0, 54.0), Sense::hover()
                            );
                            let painter = ui.painter_at(rect);
                            rgie_symbols::draw_egui(
                                &painter, rect.center(),
                                &rgie_symbols::device_primitives(kind, style),
                                Color32::from_rgb(35, 105, 160), 0.75
                            );
                            ui.label(style.label());
                            let _ = response;
                        });
                    }
                    ui.small("Reference : Livre 1 RGIE, chapitre 2.13, tableau 2.23.");
                    ui.small("Variants choisis manuellement, selon la realite sur place.");
                });
            ui.small("Symboles du tableau 2.23 pour les appareils couverts ; dossier RGIE incomplet.");
            } // Electricite
                });
            });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });

        self.refresh_underlay_texture(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view {
                View::Plan2D => self.canvas(ui),
                View::RgieReview => rgie_review::show(ui, &self.plan),
                View::Unifilar => {
                    if let Some(action) = unifilar::show(ui, &self.plan, self.settings.active_circuit_index) {
                        match action {
                            unifilar::Action::SelectCircuit(index) => {
                                self.settings.active_circuit_index = index;
                                self.pending_switch = None;
                                self.sidebar_tab = SidebarTab::Electricite;
                                self.reveal_circuit = Some(index);
                                ctx.request_repaint();
                            }
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
        self.show_sheet_name_dialog(ctx);
    }
}

#[cfg(test)]
mod plan_editor_tests {
    use super::*;

    #[test]
    fn example_board_topology_is_shared_by_three_circuits() {
        let plan: Plan = serde_json::from_str(
            include_str!("../examples/tableau-deux-ddr-trois-circuits.json")
        ).unwrap();
        assert_eq!(plan.circuit_count(), 3);
        assert_eq!(plan.differentials.len(), 2);
        assert_eq!(plan.circuits_for_differential(0), vec![0, 1]);
        assert_eq!(plan.circuits_for_differential(1), vec![2]);
        assert_eq!(plan.unassigned_outlets(), 0);
        assert!(plan.differentials.iter().all(|d| !d.missing_fields().is_empty()));
    }

    #[test]
    fn placing_socket_assigns_it_to_selected_circuit() {
        let mut app = EasyDraft::default();
        let other = app.plan.add_lighting_circuit().unwrap();
        app.settings.active_circuit_index = other;
        app.tool = Tool::Outlet;
        app.place(Point { x: 1.0, y: 2.0 }, 80.0);
        assert_eq!(app.plan.unassigned_outlets(), 0);
        assert_eq!(app.plan.point_ref(0).as_deref(), Some("B1"));
        assert_eq!(app.plan.outlets_on_circuit(other), vec![0]);
        app.undo();
        assert!(app.plan.symbols.is_empty());
    }

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
    fn joint_drag_moves_all_connected_wall_ends_and_undoes_once() {
        let mut app = EasyDraft::default();
        app.tool = Tool::SelectWall;
        app.plan.walls = vec![
            Wall {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 2.0, y: 0.0 },
                thickness_m: 0.15,
            },
            Wall {
                start: Point { x: 2.0, y: 0.0 },
                end: Point { x: 2.0, y: 3.0 },
                thickness_m: 0.15,
            },
        ];
        // Ces deux extremites sont geometriquement confondues, mais elles
        // ne sont solidaires QU'APRES la commande explicite Joindre.
        app.plan.join_wall_endpoints(&[
            WallHandle {wall_index:0,is_start:false},
            WallHandle {wall_index:1,is_start:true},
        ]).unwrap();
        app.begin_drag(Point { x: 2.0, y: 0.0 }, 80.0);
        assert!(matches!(
            app.drag.as_ref().map(|d| d.target), Some(DragTarget::Joint)
        ));
        assert_eq!(app.selected_handles.len(), 2);
        app.update_drag(Point { x: 2.75, y: 1.25 }, 80.0);
        assert_eq!(app.plan.walls[0].end, Point { x: 2.75, y: 1.25 });
        assert_eq!(app.plan.walls[1].start, Point { x: 2.75, y: 1.25 });
        app.finish_drag();
        assert_eq!(app.undo.len(), 1);
        app.undo();
        assert_eq!(app.plan.walls[0].end, Point { x: 2.0, y: 0.0 });
        assert_eq!(app.plan.walls[1].start, Point { x: 2.0, y: 0.0 });
        app.redo();
        assert_eq!(app.plan.walls[0].end, app.plan.walls[1].start);
        let restored: Plan = serde_json::from_str(
            &serde_json::to_string(&app.plan).unwrap()
        ).unwrap();
        assert_eq!(restored.walls[0].end, restored.walls[1].start);
        assert_eq!(restored.wall_joints.len(), 1);
    }

    #[test]
    fn shift_adds_independent_handles_and_weld_button_joins_them() {
        let mut app = EasyDraft::default();
        app.tool = Tool::SelectWall;
        app.plan.walls = vec![
            Wall {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 2.0, y: 0.0 },
                thickness_m: 0.15,
            },
            Wall {
                start: Point { x: 4.0, y: 2.0 },
                end: Point { x: 4.0, y: 5.0 },
                thickness_m: 0.15,
            },
        ];
        app.select_at(Point { x: 2.0, y: 0.0 }, 80.0);
        assert_eq!(app.selected_handles.len(), 1);
        assert!(app.toggle_wall_handles(Point { x: 4.0, y: 2.0 }, 80.0));
        assert_eq!(app.selected_handles.len(), 2);
        // A second Shift-click is an actual toggle.
        assert!(app.toggle_wall_handles(Point { x: 4.0, y: 2.0 }, 80.0));
        assert_eq!(app.selected_handles.len(), 1);
        assert!(app.toggle_wall_handles(Point { x: 4.0, y: 2.0 }, 80.0));
        app.weld_selected_to_first();
        assert_eq!(app.plan.walls[0].end, app.plan.walls[1].start);
        assert_eq!(app.plan.walls[1].start, Point { x: 2.0, y: 0.0 });
        assert_eq!(app.plan.wall_joints.len(), 1);
        assert_eq!(app.undo.len(), 1);
        app.dissolve_selected_joints();
        assert!(app.plan.wall_joints.is_empty());
        assert_eq!(app.undo.len(), 2);
        app.undo();
        assert_eq!(app.plan.wall_joints.len(), 1);
        app.undo();
        assert_eq!(app.plan.walls[1].start, Point { x: 4.0, y: 2.0 });
        assert!(app.plan.wall_joints.is_empty());
    }

    #[test]
    fn multi_handle_drag_snaps_to_a_third_unselected_wall() {
        let mut app = EasyDraft::default();
        app.tool = Tool::SelectWall;
        app.plan.walls = vec![
            Wall {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 2.0, y: 0.0 },
                thickness_m: 0.15,
            },
            Wall {
                start: Point { x: 4.0, y: 0.0 },
                end: Point { x: 4.0, y: 3.0 },
                thickness_m: 0.15,
            },
            Wall {
                start: Point { x: 5.0, y: 1.0 },
                end: Point { x: 7.0, y: 1.0 },
                thickness_m: 0.15,
            },
        ];
        app.select_at(Point { x: 2.0, y: 0.0 }, 80.0);
        assert!(app.toggle_wall_handles(Point { x: 4.0, y: 0.0 }, 80.0));
        app.begin_drag(Point { x: 2.0, y: 0.0 }, 80.0);
        assert!(matches!(
            app.drag.as_ref().map(|d| d.target), Some(DragTarget::Joint)
        ));
        app.update_drag(Point { x: 5.08, y: 1.05 }, 80.0);
        assert_eq!(app.plan.walls[0].end, Point { x: 5.0, y: 1.0 });
        assert_eq!(app.plan.walls[1].start, Point { x: 7.0, y: 1.0 });
        assert_eq!(app.plan.walls[2].start, Point { x: 5.0, y: 1.0 });
        // Plusieurs points deplaces ensemble NE deviennent plus joints.
        assert!(app.plan.wall_joints.is_empty());
        app.finish_drag();
        assert_eq!(app.undo.len(), 1);
    }

    #[test]
    fn welding_rejects_invalid_shortening_atomically() {
        let mut plan = Plan::default();
        plan.walls = vec![
            Wall {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 4.0, y: 0.0 },
                thickness_m: 0.15,
            },
            Wall {
                start: Point { x: 6.0, y: 1.0 },
                end: Point { x: 6.0, y: 3.0 },
                thickness_m: 0.15,
            },
        ];
        plan.add_opening(0, Point { x: 2.0, y: 0.0 }, 1.0,
            OpeningKind::Door, false, false, 0.05).unwrap();
        let originals = plan.walls.clone();
        let handles = [
            WallHandle { wall_index: 0, is_start: false },
            WallHandle { wall_index: 1, is_start: true },
        ];
        assert!(weld_wall_handles(
            &mut plan, &handles, Point { x: 1.0, y: 0.0 },
        ).is_err());
        assert_eq!(plan.walls[0].end, originals[0].end);
        assert_eq!(plan.walls[1].start, originals[1].start);
        let both_ends = [
            WallHandle { wall_index: 0, is_start: true },
            WallHandle { wall_index: 0, is_start: false },
        ];
        assert!(weld_wall_handles(
            &mut plan, &both_ends, Point { x: 2.0, y: 0.0 },
        ).is_err());
    }

    #[test]
    fn shift_marquee_selects_only_wall_endpoints_not_entire_walls() {
        let mut app = EasyDraft::default();
        app.tool = Tool::SelectWall;
        app.plan.walls = vec![
            Wall {
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 2.0, y: 0.0 },
                thickness_m: 0.15,
            },
            Wall {
                start: Point { x: 2.0, y: 1.0 },
                end: Point { x: 4.0, y: 1.0 },
                thickness_m: 0.15,
            },
        ];
        app.marquee_handles = true;
        app.begin_drag(Point { x: 1.5, y: -0.5 }, 80.0);
        assert!(app.marquee.is_some());
        app.update_marquee(Point { x: 2.5, y: 1.5 });
        app.finish_marquee();
        assert_eq!(app.selected_handles.len(), 2);
        assert!(app.selection.walls.is_empty());
        assert!(app.selected_handles.contains(&WallHandle {
            wall_index: 0, is_start: false
        }));
        assert!(app.selected_handles.contains(&WallHandle {
            wall_index: 1, is_start: true
        }));
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
    fn wall_preview_snaps_to_true_horizontal_and_vertical() {
        let start = Point { x: 1.0, y: 2.0 };
        let (horizontal, axis) =
            aligned_wall_end(start, Point { x: 5.0, y: 2.2 }, false);
        assert_eq!(axis, Some(WallAlignment::Horizontal));
        assert_eq!(horizontal, Point { x: 5.0, y: 2.0 });
        let (vertical, axis) =
            aligned_wall_end(start, Point { x: 1.12, y: -2.0 }, false);
        assert_eq!(axis, Some(WallAlignment::Vertical));
        assert_eq!(vertical, Point { x: 1.0, y: -2.0 });
        assert!(wall_heading(start, horizontal, Some(WallAlignment::Horizontal))
            .contains("0°"));
        assert!(wall_heading(start, vertical, Some(WallAlignment::Vertical))
            .contains("90°"));
    }

    #[test]
    fn oblique_wall_keeps_its_angle_except_with_shift_held() {
        let start = Point { x: 0.0, y: 0.0 };
        let candidate = Point { x: 3.0, y: 1.5 };
        let (free, no_axis) = aligned_wall_end(start, candidate, false);
        assert_eq!(no_axis, None);
        assert_eq!(free, candidate);
        assert!(wall_heading(start, free, no_axis).contains("INCLINE"));
        let (locked, axis) = aligned_wall_end(start, candidate, true);
        assert_eq!(axis, Some(WallAlignment::Horizontal));
        assert_eq!(locked, Point { x: 3.0, y: 0.0 });
        let (locked, axis) = aligned_wall_end(
            start, Point { x: 1.5, y: 3.0 }, true
        );
        assert_eq!(axis, Some(WallAlignment::Vertical));
        assert_eq!(locked, Point { x: 0.0, y: 3.0 });
    }

    #[test]
    fn snapped_wall_is_recorded_straight_in_plan() {
        let mut app = EasyDraft::default();
        let start = Point { x: 0.0, y: 0.0 };
        app.select_tool(Tool::Wall);
        app.place(start, 80.0);
        let (end, axis) = aligned_wall_end(
            start, Point { x: 4.0, y: 0.25 }, true
        );
        assert_eq!(axis, Some(WallAlignment::Horizontal));
        app.place(end, 80.0);
        assert_eq!(app.plan.walls.len(), 1);
        assert_eq!(app.plan.walls[0].start.y, app.plan.walls[0].end.y);
        assert_eq!(app.tool, Tool::SelectWall);
    }

    #[test]
    fn passage_tool_fixes_an_existing_gap_even_when_click_is_near_a_wall() {
        let mut app = EasyDraft::default();
        app.tool = Tool::Passage;
        // L'enveloppe est fermee, mais sa cloison a un passage deja dessine
        // dans le vide. Auparavant un clic pres du bord touchait un mur
        // voisin et ne reparait pas la detection des pieces.
        app.plan.walls = vec![
            Wall { start: Point{x:0.0,y:0.0}, end: Point{x:6.0,y:0.0}, thickness_m:0.15 },
            Wall { start: Point{x:6.0,y:0.0}, end: Point{x:6.0,y:4.0}, thickness_m:0.15 },
            Wall { start: Point{x:6.0,y:4.0}, end: Point{x:0.0,y:4.0}, thickness_m:0.15 },
            Wall { start: Point{x:0.0,y:4.0}, end: Point{x:0.0,y:0.0}, thickness_m:0.15 },
            Wall { start: Point{x:3.0,y:0.0}, end: Point{x:3.0,y:1.5}, thickness_m:0.15 },
            Wall { start: Point{x:3.0,y:2.5}, end: Point{x:3.0,y:4.0}, thickness_m:0.15 },
        ];
        assert_eq!(rooms::detect_rooms(&app.plan.walls).len(), 1);
        let click = Point { x: 3.0, y: 1.58 };
        assert!(app.find_wall(click, 80.0).is_some());
        app.place(click, 80.0);
        assert_eq!(rooms::detect_rooms(&app.plan.walls).len(), 2);
        assert_eq!(app.plan.openings.len(), 1);
        assert_eq!(app.plan.openings[0].kind, OpeningKind::Passage);
        assert_eq!(app.undo.len(), 1);
        app.undo();
        assert_eq!(rooms::detect_rooms(&app.plan.walls).len(), 1);
    }

    #[test]
    fn passage_tool_closes_t_gap_and_keeps_the_opening_visible() {
        let mut app = EasyDraft::default();
        app.tool = Tool::Passage;
        app.plan.walls = vec![
            Wall { start: Point{x:0.0,y:0.0}, end: Point{x:6.0,y:0.0}, thickness_m:0.15 },
            Wall { start: Point{x:6.0,y:0.0}, end: Point{x:6.0,y:4.0}, thickness_m:0.15 },
            Wall { start: Point{x:6.0,y:4.0}, end: Point{x:0.0,y:4.0}, thickness_m:0.15 },
            Wall { start: Point{x:0.0,y:4.0}, end: Point{x:0.0,y:0.0}, thickness_m:0.15 },
            Wall { start: Point{x:3.0,y:0.0}, end: Point{x:3.0,y:3.0}, thickness_m:0.15 },
        ];
        assert_eq!(rooms::detect_rooms(&app.plan.walls).len(), 1);
        app.place(Point { x: 3.0, y: 3.6 }, 80.0);
        assert_eq!(rooms::detect_rooms(&app.plan.walls).len(), 2);
        assert_eq!(app.plan.openings.len(), 1);
        assert_eq!(app.plan.openings[0].kind, OpeningKind::Passage);
    }

    #[test]
    fn wall_auto_switches_to_selection_after_second_click() {
        let mut app = EasyDraft::default();
        assert_eq!(app.tool, Tool::SelectWall);
        app.select_tool(Tool::Wall);
        app.place(Point { x: 0.0, y: 0.0 }, 80.0);
        assert_eq!(app.tool, Tool::Wall);
        app.place(Point { x: 4.0, y: 0.0 }, 80.0);
        assert_eq!(app.plan.walls.len(), 1);
        assert_eq!(app.tool, Tool::SelectWall);
        assert_eq!(app.selected_wall, Some(0));
        assert!(app.pending_wall.is_none());
    }

    #[test]
    fn placement_returns_to_selection_and_selects_the_new_element() {
        let mut app = EasyDraft::default();
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 5.0, y: 0.0 };
        app.select_tool(Tool::Wall);
        app.place(start, 80.0);
        app.place(end, 80.0);

        app.select_tool(Tool::Door);
        app.place(Point { x: 2.5, y: 3.0 }, 80.0);
        assert_eq!(app.tool, Tool::Door);
        app.place(Point { x: 2.5, y: 0.0 }, 80.0);
        assert_eq!(app.tool, Tool::SelectWall);
        assert_eq!(app.selected_opening, Some(0));

        app.select_tool(Tool::Stair);
        app.place(start, 80.0);
        assert_eq!(app.tool, Tool::Stair);
        app.place(end, 80.0);
        assert_eq!(app.tool, Tool::SelectWall);
        assert_eq!(app.selected_stair, Some(0));

        for (index, tool) in [Tool::Outlet, Tool::Switch, Tool::Light].into_iter().enumerate() {
            app.select_tool(tool);
            app.place(start, 80.0);
            assert_eq!(app.tool, Tool::SelectWall);
            assert_eq!(app.selected_symbol, Some(index));
        }
    }

    #[test]
    fn window_selection_and_crossing_selection_work_differently() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 1.0, y: 0.6 },
            kind: ElectricalKind::Outlet,
        });
        let window = Marquee {
            start: Point { x: 0.5, y: -0.5 },
            current: Point { x: 2.0, y: 1.0 },
        };
        let selected = SelectionSet::from_marquee(&plan, window);
        assert_eq!(selected.walls.len(), 0);
        assert_eq!(selected.symbols, vec![0]);
        let crossing = Marquee { start: window.current, current: window.start };
        let selected = SelectionSet::from_marquee(&plan, crossing);
        assert_eq!(selected.walls, vec![0]);
        assert_eq!(selected.symbols, vec![0]);
    }

    #[test]
    fn group_drag_moves_wall_and_symbol_preserving_its_opening_and_undo() {
        let mut app = EasyDraft::default();
        app.tool = Tool::SelectWall;
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.plan.add_opening(0, Point { x: 1.0, y: 0.0 }, 0.9,
            OpeningKind::Door, false, false, 0.05).unwrap();
        app.plan.symbols.push(ElectricalSymbol {
            position: Point { x: 2.0, y: 1.0 },
            kind: ElectricalKind::Light,
        });
        // Un rectangle autour des deux objets sans toucher la porte.
        app.selection = SelectionSet {
            walls: vec![0], symbols: vec![0], ..SelectionSet::default()
        };
        app.begin_drag(Point { x: 3.0, y: 0.0 }, 80.0);
        assert!(matches!(app.drag.as_ref().map(|d| d.target), Some(DragTarget::Group)));
        app.update_drag(Point { x: 3.0, y: 1.2 }, 80.0);
        app.finish_drag();
        assert!((app.plan.walls[0].start.y - 1.2).abs() < 0.001);
        assert!((app.plan.symbols[0].position.y - 2.2).abs() < 0.001);
        assert_eq!(app.plan.openings[0].center_m, 1.0);
        assert_eq!(app.undo.len(), 1);
        app.undo();
        assert_eq!(app.plan.walls[0].start.y, 0.0);
        assert_eq!(app.plan.symbols[0].position.y, 1.0);
    }

    #[test]
    fn symbol_and_stair_can_be_dragged_without_a_group() {
        let mut app = EasyDraft::default();
        app.tool = Tool::SelectWall;
        app.plan.symbols.push(ElectricalSymbol {
            position: Point { x: 2.0, y: 2.0 }, kind: ElectricalKind::Outlet
        });
        app.plan.stairs.push(Stair {
            start: Point { x: 5.0, y: 0.0 },
            end: Point { x: 5.0, y: 3.0 },
            width_m: 1.0,
            steps: 12,
        });
        app.begin_drag(Point { x: 2.0, y: 2.0 }, 80.0);
        assert!(matches!(app.drag.as_ref().map(|d| d.target), Some(DragTarget::Symbol(0))));
        app.update_drag(Point { x: 2.5, y: 2.0 }, 80.0);
        app.finish_drag();
        assert!((app.plan.symbols[0].position.x - 2.5).abs() < 0.001);
        app.begin_drag(Point { x: 5.0, y: 1.5 }, 80.0);
        assert!(matches!(app.drag.as_ref().map(|d| d.target), Some(DragTarget::Stair(0))));
        app.update_drag(Point { x: 6.0, y: 1.5 }, 80.0);
        app.finish_drag();
        assert!((app.plan.stairs[0].start.x - 6.0).abs() < 0.001);
    }

    #[test]
    fn save_current_writes_valid_json_and_pdf_export_creates_file() {
        let mut app = EasyDraft::default();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        let filename = format!("easydraft-file-test-{}", std::process::id());
        let json_path = std::env::temp_dir().join(format!("{filename}.json"));
        let pdf_path = std::env::temp_dir().join(format!("{filename}.pdf"));
        app.file_path = json_path.to_string_lossy().into_owned();
        app.file_path_assigned = true;
        app.save();
        assert!(app.status.starts_with("Plan enregistre"));
        let restored: Plan = serde_json::from_slice(&std::fs::read(&json_path).unwrap()).unwrap();
        assert_eq!(restored.walls.len(), 1);
        pdf_export::export_pdf(pdf_path.to_str().unwrap(), &restored, true).unwrap();
        let pdf_data = std::fs::read(&pdf_path).unwrap();
        assert!(pdf_data.starts_with(b"%PDF-"));
        assert!(pdf_data.len() > 200);
        let _ = std::fs::remove_file(json_path);
        let _ = std::fs::remove_file(pdf_path);
    }

    #[test]
    fn legacy_project_and_new_sheets_round_trip_and_export_together() {
        let legacy = include_str!("../examples/maison-deux-pieces.json");
        let project: Project = serde_json::from_str(legacy).unwrap();
        let mut app = EasyDraft::default();
        app.set_project(project);
        let old_wall_count = app.plan.walls.len();
        assert!(old_wall_count > 1);
        assert_eq!(app.sheets[0].name, "Rez-de-chaussée");

        app.add_sheet("1er étage".into());
        assert!(app.plan.walls.is_empty());
        app.snapshot();
        app.plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 2.0, y: 0.0 },
            thickness_m: 0.15,
        });
        app.switch_sheet(0);
        assert_eq!(app.plan.walls.len(), old_wall_count);
        app.undo();
        assert_eq!(app.active_sheet, 1);
        assert!(app.plan.walls.is_empty());
        app.redo();
        assert_eq!(app.plan.walls.len(), 1);
        app.add_sheet("Annexe".into());

        let json = serde_json::to_string(&app.project()).unwrap();
        let mut reopened = EasyDraft::default();
        reopened.set_project(serde_json::from_str(&json).unwrap());
        assert_eq!(reopened.active_sheet, 2);
        assert!(reopened.plan.walls.is_empty());
        assert_eq!(reopened.sheets[0].plan.walls.len(), old_wall_count);
        assert_eq!(reopened.sheets[1].plan.walls.len(), 1);

        let file = std::env::temp_dir().join(format!("easydraft-sheets-{}.pdf", std::process::id()));
        reopened.pdf_path = file.to_string_lossy().into_owned();
        reopened.export_pdf();
        let pdf = std::fs::read(&file).unwrap();
        let _ = std::fs::remove_file(file);
        let pdf = String::from_utf8_lossy(&pdf);
        assert!(pdf.contains("/Count 3"));
        let ground_floor = pdf.find(r"(EasyDraft - Rez-de-chauss\351e)").unwrap();
        let first_floor = pdf.find(r"(EasyDraft - 1er \351tage)").unwrap();
        let annexe = pdf.find("(EasyDraft - Annexe)").unwrap();
        assert!(ground_floor < first_floor && first_floor < annexe);
        assert!(pdf[first_floor..annexe].contains("1 murs"));
        assert!(pdf[annexe..].contains("0 murs"));
    }

    #[test]
    fn project_settings_round_trip_and_legacy_files_use_defaults() {
        let legacy = include_str!("../examples/maison-deux-pieces.json");
        let mut app = EasyDraft::default();
        app.set_project(serde_json::from_str(legacy).unwrap());
        assert_eq!(app.settings, ProjectSettings::default());
        app.add_sheet("Cave".into());
        app.plan.add_lighting_circuit().unwrap();
        app.settings = ProjectSettings {
            grid: false,
            show_dimensions: false,
            show_architecture: false,
            show_electrical: false,
            show_rooms: false,
            show_underlay: false,
            snap_endpoints: false,
            snap_m: 0.2,
            zoom: 1.5,
            pan: [12.0, -40.0],
            circuit_visibility: CircuitVisibility::Active,
            active_circuit_index: 1,
            new_wall_thickness_cm: 24.0,
            ..ProjectSettings::default()
        };
        let settings = app.settings;
        let json = serde_json::to_string(&app.project()).unwrap();
        let mut reopened = EasyDraft::default();
        reopened.set_project(serde_json::from_str(&json).unwrap());
        assert_eq!(reopened.active_sheet, 1);
        assert_eq!(reopened.settings, settings);
        reopened.switch_sheet(0);
        assert_eq!(reopened.settings.circuit_visibility, CircuitVisibility::Active);
        assert_eq!(reopened.settings.active_circuit_index, 0);
    }

    #[test]
    fn clicking_and_typing_in_sheet_name_dialog_renames_active_sheet() {
        let mut app = EasyDraft::default();
        let ctx = egui::Context::default();
        let frame = |app: &mut EasyDraft, events| {
            let screen_rect = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
            let _ = ctx.run(egui::RawInput { screen_rect: Some(screen_rect), events, ..Default::default() }, |ctx| {
                app.show_sheet_name_dialog(ctx);
            });
        };
        app.add_sheet("1er étage".into());
        app.open_sheet_name_dialog(true);
        for _ in 0..3 {
            frame(&mut app, Vec::new());
        }
        let textbox = ctx.memory(|mem| mem.focused()).unwrap();
        let pos = ctx.read_response(textbox).unwrap().rect.center();
        let click = |pressed| egui::Event::PointerButton {
            pos, button: egui::PointerButton::Primary, pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![egui::Event::PointerMoved(pos), click(true)]);
        frame(&mut app, vec![click(false)]);
        assert!(app.sheet_name_dialog.is_some());
        let key = |key, modifiers| egui::Event::Key {
            key, physical_key: None, pressed: true, repeat: false, modifiers,
        };
        frame(&mut app, vec![
            key(egui::Key::A, egui::Modifiers::COMMAND),
            egui::Event::Text("Étage nord".into()),
        ]);
        assert_eq!(app.sheet_name_dialog.as_ref().unwrap().name, "Étage nord");
        frame(&mut app, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(app.sheet_name_dialog.is_none());
        assert_eq!(app.sheets[1].name, "Étage nord");
        app.undo();
        assert_eq!(app.sheets[1].name, "1er étage");
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
        Box::new(|cc| {
            // Presentation claire et consistante : fond blanc, accent bleu pour
            // les commandes actives et titres visuels discrets.
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals = egui::Visuals::light();
            style.visuals.selection.bg_fill = Color32::from_rgb(46, 108, 224);
            style.visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
            style.spacing.item_spacing = Vec2::new(6.0, 6.0);
            cc.egui_ctx.set_style(style);
            Ok(Box::new(EasyDraft::default()))
        }),
    )
}
