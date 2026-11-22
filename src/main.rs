mod model;

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Sense, Stroke, Vec2};
use model::{ElectricalKind, ElectricalSymbol, Plan, Point, Wall};

const GRID_METERS: f32 = 0.5;
const PIXELS_PER_METER: f32 = 80.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Wall,
    Outlet,
    Switch,
    Light,
    Pan,
}

struct EasyDraft {
    plan: Plan,
    undo: Vec<Plan>,
    tool: Tool,
    pending_wall: Option<Point>,
    zoom: f32,
    pan: Vec2,
    grid: bool,
    file_path: String,
    status: String,
}

impl Default for EasyDraft {
    fn default() -> Self {
        Self {
            plan: Plan::default(),
            undo: Vec::new(),
            tool: Tool::Wall,
            pending_wall: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            grid: true,
            file_path: "mon-plan.json".into(),
            status: "Clique une fois pour commencer un mur, puis une seconde fois pour le terminer.".into(),
        }
    }
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
                self.status = format!("Ouvert : {}", self.file_path);
            }
            Err(e) => self.status = format!("Erreur : {e}"),
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.pending_wall = None;
    }

    fn place(&mut self, position: Point) {
        match self.tool {
            Tool::Wall => {
                if let Some(start) = self.pending_wall.take() {
                    let distance = start.distance_to(position);
                    if distance > 0.001 {
                        self.snapshot();
                        self.plan.walls.push(Wall { start, end: position });
                        self.status = format!("Mur de {distance:.2} m ajoute");
                    }
                } else {
                    self.pending_wall = Some(position);
                    self.status = "Choisis le deuxieme point du mur".into();
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
                self.place(to_world(pos).snapped(GRID_METERS));
            }
        }

        for wall in &self.plan.walls {
            let a = to_screen(wall.start);
            let b = to_screen(wall.end);
            painter.line_segment([a, b], Stroke::new(7.0, Color32::from_rgb(45, 53, 65)));
            painter.text(
                a + (b - a) * 0.5 + Vec2::new(0.0, -14.0),
                Align2::CENTER_CENTER,
                format!("{:.2} m", wall.start.distance_to(wall.end)),
                FontId::proportional(13.0),
                Color32::DARK_GRAY,
            );
        }

        for symbol in &self.plan.symbols {
            let pos = to_screen(symbol.position);
            painter.circle_filled(pos, 13.0, Color32::WHITE);
            painter.circle_stroke(pos, 13.0, Stroke::new(2.0, Color32::from_rgb(20, 106, 190)));
            let label = match symbol.kind {
                ElectricalKind::Outlet => "P",
                ElectricalKind::Switch => "I",
                ElectricalKind::Light => "X",
            };
            painter.text(pos, Align2::CENTER_CENTER, label, FontId::proportional(15.0), Color32::from_rgb(20, 106, 190));
        }

        if let Some(start) = self.pending_wall {
            if let Some(cursor) = response.hover_pos() {
                let end = to_world(cursor).snapped(GRID_METERS);
                painter.line_segment([to_screen(start), to_screen(end)], Stroke::new(3.0, Color32::from_rgb(40, 130, 215)));
            }
        }

        painter.text(
            rect.left_top() + Vec2::new(12.0, 12.0),
            Align2::LEFT_TOP,
            format!("Grille : 0,5 m | Zoom : {:.0} %", self.zoom * 100.0),
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
                if ui.button("Nouveau").clicked() {
                    self.snapshot();
                    self.plan = Plan::default();
                    self.pending_wall = None;
                    self.status = "Nouveau plan".into();
                }
                if ui.button("Annuler").clicked() { self.undo(); }
                if ui.button("Enregistrer").clicked() { self.save(); }
                if ui.button("Ouvrir").clicked() { self.load(); }
            });
        });

        egui::SidePanel::left("tools").show(ctx, |ui| {
            ui.heading("Outils");
            ui.separator();
            if ui.selectable_label(self.tool == Tool::Wall, "Mur").clicked() { self.select_tool(Tool::Wall); }
            if ui.selectable_label(self.tool == Tool::Outlet, "Prise").clicked() { self.select_tool(Tool::Outlet); }
            if ui.selectable_label(self.tool == Tool::Switch, "Interrupteur").clicked() { self.select_tool(Tool::Switch); }
            if ui.selectable_label(self.tool == Tool::Light, "Point lumineux").clicked() { self.select_tool(Tool::Light); }
            if ui.selectable_label(self.tool == Tool::Pan, "Deplacer la vue").clicked() { self.select_tool(Tool::Pan); }
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
            ui.label(format!("Murs : {}", self.plan.walls.len()));
            ui.label(format!("Symboles : {}", self.plan.symbols.len()));
            ui.small("Symboles indicatifs, non certifies RGIE.");
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });

        egui::CentralPanel::default().show(ctx, |ui| self.canvas(ui));
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
