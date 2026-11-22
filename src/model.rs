use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn distance_to(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn snapped(self, step: f32) -> Self {
        Self {
            x: (self.x / step).round() * step,
            y: (self.y / step).round() * step,
        }
    }
}

pub const DEFAULT_WALL_THICKNESS_M: f32 = 0.15;

fn default_wall_thickness_m() -> f32 {
    DEFAULT_WALL_THICKNESS_M
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub start: Point,
    pub end: Point,
    // Epaisseur en metres. Le defaut maintient la compatibilite avec les anciens plans.
    #[serde(default = "default_wall_thickness_m")]
    pub thickness_m: f32,
}

impl Wall {
    pub fn length_m(&self) -> f32 {
        self.start.distance_to(self.end)
    }

    /// Position d'un point projeté sur l'axe du mur, mesurée depuis son départ.
    pub fn projected_distance_m(&self, point: Point) -> f32 {
        let length = self.length_m();
        if length <= f32::EPSILON {
            return 0.0;
        }
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let projection = ((point.x - self.start.x) * dx
            + (point.y - self.start.y) * dy) / length;
        projection.clamp(0.0, length)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpeningKind {
    Door,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    /// Index du mur porteur dans Plan::walls.
    pub wall_index: usize,
    /// Distance du centre depuis le début du mur, en mètres.
    pub center_m: f32,
    pub width_m: f32,
    pub kind: OpeningKind,
    #[serde(default)]
    pub hinge_right: bool,
    #[serde(default)]
    pub swing_flipped: bool,
}

impl Opening {
    pub fn start_m(&self) -> f32 {
        self.center_m - self.width_m / 2.0
    }

    pub fn end_m(&self) -> f32 {
        self.center_m + self.width_m / 2.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ElectricalKind {
    Outlet,
    Switch,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectricalSymbol {
    pub position: Point,
    pub kind: ElectricalKind,
}

/// Relation fonctionnelle entre un interrupteur et un luminaire.
 /// Les indices referencent la liste Plan::symbols ; ils restent coherents tant qu'on
 /// n'offre pas de suppression ou reordonnancement individuel des symboles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightingConnection {
    pub switch_index: usize,
    pub light_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stair {
    /// Debut de l'escalier, en metres.
    pub start: Point,
    /// Fin de l'escalier, dans le sens de la montee.
    pub end: Point,
    /// Largeur perpendiculaire a la montee, en metres.
    pub width_m: f32,
    /// Nombre de marches representees en plan.
    pub steps: u16,
}

impl Stair {
    pub fn length_m(&self) -> f32 {
        self.start.distance_to(self.end)
    }

    pub fn is_valid(&self) -> bool {
        self.length_m() > 0.05
            && self.width_m.is_finite()
            && self.width_m > 0.0
            && (2..=60).contains(&self.steps)
    }

    /// Les quatre coins du rectangle de l'escalier, selon son orientation.
    pub fn corners(&self) -> [Point; 4] {
        let len = self.length_m().max(0.0001);
        let nx = -(self.end.y - self.start.y) / len * self.width_m / 2.0;
        let ny = (self.end.x - self.start.x) / len * self.width_m / 2.0;
        [
            Point { x: self.start.x + nx, y: self.start.y + ny },
            Point { x: self.end.x + nx, y: self.end.y + ny },
            Point { x: self.end.x - nx, y: self.end.y - ny },
            Point { x: self.start.x - nx, y: self.start.y - ny },
        ]
    }
}

/// Metadonnees du circuit d'eclairage. Zero/chaine vide signifie "non renseigne"
/// pour ne jamais inventer les caracteristiques de l'installation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LightingCircuit {
    pub letter: String,
    pub breaker_amp: u16,
    pub breaker_poles: u8,
    pub breaker_curve: String,
    pub upstream_rdc: String,
    pub cable_type: String,
    pub conductor_count: u8,
    pub section_mm2: f32,
    pub installation_method: String,
}

impl Default for LightingCircuit {
    fn default() -> Self {
        Self {
            letter: "A".into(),
            breaker_amp: 0,
            breaker_poles: 0,
            breaker_curve: String::new(),
            upstream_rdc: String::new(),
            cable_type: String::new(),
            conductor_count: 0,
            section_mm2: 0.0,
            installation_method: String::new(),
        }
    }
}

impl LightingCircuit {
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.letter.trim().is_empty() { missing.push("repere circuit"); }
        if self.breaker_amp == 0 { missing.push("calibre protection"); }
        if self.breaker_poles == 0 { missing.push("nombre de poles"); }
        if self.breaker_curve.trim().is_empty() { missing.push("courbe/type protection"); }
        if self.upstream_rdc.trim().is_empty() { missing.push("differentiel en amont"); }
        if self.cable_type.trim().is_empty() { missing.push("type de cable"); }
        if self.conductor_count == 0 { missing.push("nombre de conducteurs"); }
        if !self.section_mm2.is_finite() || self.section_mm2 <= 0.0 { missing.push("section du cable"); }
        if self.installation_method.trim().is_empty() { missing.push("mode de pose"); }
        missing
    }

    pub fn protection_label(&self) -> String {
        let mut parts = vec!["Disjoncteur".to_owned()];
        if self.breaker_poles > 0 { parts.push(format!("{}P", self.breaker_poles)); }
        if !self.breaker_curve.trim().is_empty() { parts.push(self.breaker_curve.trim().to_owned()); }
        if self.breaker_amp > 0 { parts.push(format!("{} A", self.breaker_amp)); }
        parts.join(" ")
    }

    pub fn cable_label(&self) -> String {
        if self.cable_type.trim().is_empty() || self.conductor_count == 0
            || !self.section_mm2.is_finite() || self.section_mm2 <= 0.0 {
            return "Canalisation : a renseigner".to_owned();
        }
        format!("{} {}G{} mm2", self.cable_type.trim(), self.conductor_count, self.section_mm2)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub walls: Vec<Wall>,
    pub symbols: Vec<ElectricalSymbol>,
    /// La valeur par défaut garde la compatibilité avec les plans JSON anciens.
    #[serde(default)]
    pub openings: Vec<Opening>,
    #[serde(default)]
    pub stairs: Vec<Stair>,
    /// Liaisons logiques servant aux deux vues du projet.
    /// L'absence de ce champ dans un ancien JSON donne une liste vide.
    #[serde(default)]
    pub lighting_connections: Vec<LightingConnection>,
    #[serde(default)]
    pub lighting_circuit: LightingCircuit,
}

impl Plan {
    /// Lie un interrupteur a un point lumineux (ce n'est pas un chemin de cable).
    pub fn connect_switch_to_light(
        &mut self,
        switch_index: usize,
        light_index: usize,
    ) -> Result<(), &'static str> {
        if !matches!(self.symbols.get(switch_index), Some(s) if s.kind == ElectricalKind::Switch) {
            return Err("Choisis un interrupteur valide");
        }
        if !matches!(self.symbols.get(light_index), Some(s) if s.kind == ElectricalKind::Light) {
            return Err("Choisis un point lumineux valide");
        }
        if self.lighting_connections.iter().any(|c| c.light_index == light_index) {
            return Err("Ce point lumineux est deja lie a un interrupteur");
        }
        self.lighting_connections.push(LightingConnection {
            switch_index,
            light_index,
        });
        Ok(())
    }

    pub fn lighting_connection_is_valid(&self, link: &LightingConnection) -> bool {
        matches!(self.symbols.get(link.switch_index), Some(s) if s.kind == ElectricalKind::Switch)
            && matches!(self.symbols.get(link.light_index), Some(s) if s.kind == ElectricalKind::Light)
    }

    /// Numero visible du symbole parmi ceux du meme type, en commençant a 1.
    pub fn symbol_number(&self, index: usize) -> Option<usize> {
        let target = self.symbols.get(index)?.kind;
        Some(self.symbols[..=index].iter().filter(|s| s.kind == target).count())
    }

    /// Regroupe les points lumineux qui partagent le meme interrupteur.
    /// Cette vue a une unique definition, reutilisee par l'interface et l'export.
    pub fn lighting_groups(&self) -> Vec<(usize, Vec<usize>)> {
        let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
        let mut seen_lights = std::collections::HashSet::new();
        for link in &self.lighting_connections {
            if !self.lighting_connection_is_valid(link) || !seen_lights.insert(link.light_index) {
                continue;
            }
            if let Some((_, lights)) = groups.iter_mut().find(|(i, _)| *i == link.switch_index) {
                lights.push(link.light_index);
            } else {
                groups.push((link.switch_index, vec![link.light_index]));
            }
        }
        groups
    }

    pub fn unlinked_switches(&self) -> usize {
        self.symbols.iter().enumerate().filter(|(index, symbol)| {
            symbol.kind == ElectricalKind::Switch
                && !self.lighting_connections.iter().any(|link| {
                    link.switch_index == *index && self.lighting_connection_is_valid(link)
                })
        }).count()
    }

    pub fn unlinked_lights(&self) -> usize {
        self.symbols.iter().enumerate().filter(|(index, symbol)| {
            symbol.kind == ElectricalKind::Light
                && !self.lighting_connections.iter().any(|link| {
                    link.light_index == *index && self.lighting_connection_is_valid(link)
                })
        }).count()
    }

    /// Ajoute une demonstration fonctionnelle dans les deux vues, sans effacer le projet.
    pub fn add_demo_lighting(&mut self) {
        let offset = self.symbols.iter().map(|s| s.position.x).fold(0.0_f32, f32::max) + 1.0;
        let switch_index = self.symbols.len();
        self.symbols.push(ElectricalSymbol {
            position: Point { x: offset, y: 0.0 },
            kind: ElectricalKind::Switch,
        });
        let light_index = self.symbols.len();
        self.symbols.push(ElectricalSymbol {
            position: Point { x: offset + 2.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        self.lighting_connections.push(LightingConnection { switch_index, light_index });
    }

    /// Ajoute une ouverture centrée au clic sur un mur, sans dépasser ni chevaucher une autre ouverture.
    pub fn add_opening(
        &mut self,
        wall_index: usize,
        clicked_at: Point,
        width_m: f32,
        kind: OpeningKind,
        hinge_right: bool,
        swing_flipped: bool,
        snap_m: f32,
    ) -> Result<(), &'static str> {
        let wall = self.walls.get(wall_index).ok_or("Mur introuvable")?;
        let wall_length = wall.length_m();
        const MARGIN: f32 = 0.05;
        if !width_m.is_finite() || width_m <= 0.0 || !snap_m.is_finite() || snap_m <= 0.0 {
            return Err("Largeur ou pas de grille invalide");
        }
        if wall_length < width_m + 2.0 * MARGIN {
            return Err("Le mur est trop court pour cette ouverture");
        }
        let projected = wall.projected_distance_m(clicked_at);
        let center = (projected / snap_m).round() * snap_m;
        let center = center.clamp(width_m / 2.0 + MARGIN, wall_length - width_m / 2.0 - MARGIN);
        let proposed_start = center - width_m / 2.0;
        let proposed_end = center + width_m / 2.0;
        if self.openings.iter().any(|o| {
            o.wall_index == wall_index
                && proposed_start < o.end_m() + MARGIN
                && proposed_end > o.start_m() - MARGIN
        }) {
            return Err("Cette ouverture chevauche une autre porte ou fenêtre");
        }
        self.openings.push(Opening {
            wall_index,
            center_m: center,
            width_m,
            kind,
            hinge_right,
            swing_flipped,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_to_half_meter() {
        assert_eq!(
            Point { x: 1.26, y: -0.24 }.snapped(0.5),
            Point { x: 1.5, y: 0.0 }
        );
    }

    #[test]
    fn wall_length_in_meters() {
        assert_eq!(
            Point { x: 0.0, y: 0.0 }.distance_to(Point { x: 3.0, y: 4.0 }),
            5.0
        );
    }

    #[test]
    fn old_wall_json_uses_default_thickness() {
        let old = r#"{"start":{"x":0.0,"y":0.0},"end":{"x":3.0,"y":0.0}}"#;
        let wall: Wall = serde_json::from_str(old).unwrap();
        assert_eq!(wall.thickness_m, DEFAULT_WALL_THICKNESS_M);
        assert_eq!(wall.length_m(), 3.0);
    }

    #[test]
    fn wall_thickness_round_trip() {
        let wall = Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.30,
        };
        let restored: Wall = serde_json::from_str(&serde_json::to_string(&wall).unwrap()).unwrap();
        assert_eq!(restored.thickness_m, 0.30);
    }

    #[test]
    fn door_is_placed_on_wall_axis() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 1.0, y: 2.0 },
            end: Point { x: 5.0, y: 2.0 },
            thickness_m: 0.2,
        });
        plan.add_opening(0, Point { x: 3.27, y: 2.06 }, 0.9, OpeningKind::Door, false, false, 0.05).unwrap();
        assert_eq!(plan.openings[0].center_m, 2.25);
        assert_eq!(plan.openings[0].kind, OpeningKind::Door);
    }

    #[test]
    fn openings_cannot_overlap_or_leave_wall() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 3.0, y: 0.0 },
            thickness_m: 0.15,
        });
        plan.add_opening(0, Point { x: 1.5, y: 0.0 }, 1.2, OpeningKind::Window, false, false, 0.05).unwrap();
        assert!(plan.add_opening(0, Point { x: 1.7, y: 0.0 }, 0.9, OpeningKind::Door, false, false, 0.05).is_err());
        assert!(plan.add_opening(0, Point { x: 0.0, y: 0.0 }, 3.0, OpeningKind::Door, false, false, 0.05).is_err());
        assert_eq!(plan.openings.len(), 1);
    }

    #[test]
    fn old_plan_json_has_no_openings() {
        let old = r#"{"walls":[],"symbols":[]}"#;
        let plan: Plan = serde_json::from_str(old).unwrap();
        assert!(plan.openings.is_empty());
    }

    #[test]
    fn openings_survive_json_round_trip() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 3.0, y: 0.0 },
            thickness_m: 0.15,
        });
        plan.add_opening(0, Point { x: 1.5, y: 0.0 }, 0.9, OpeningKind::Door, true, true, 0.05).unwrap();
        let restored: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(restored.openings.len(), 1);
        assert!(restored.openings[0].hinge_right);
        assert!(restored.openings[0].swing_flipped);
    }

    #[test]
    fn old_plan_json_has_no_stairs() {
        let old = r#"{"walls":[],"symbols":[]}"#;
        let plan: Plan = serde_json::from_str(old).unwrap();
        assert!(plan.stairs.is_empty());
    }

    #[test]
    fn stair_geometry_and_json_round_trip() {
        let stair = Stair {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 3.0, y: 0.0 },
            width_m: 1.2,
            steps: 12,
        };
        assert!(stair.is_valid());
        assert_eq!(stair.length_m(), 3.0);
        let corners = stair.corners();
        assert_eq!(corners[0], Point { x: 0.0, y: 0.6 });
        assert_eq!(corners[3], Point { x: 0.0, y: -0.6 });
        let mut plan = Plan::default();
        plan.stairs.push(stair);
        let encoded = serde_json::to_string(&plan).unwrap();
        let decoded: Plan = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.stairs.len(), 1);
        assert_eq!(decoded.stairs[0].steps, 12);
    }

    #[test]
    fn demo_lighting_has_functional_connection() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        assert_eq!(plan.symbols.len(), 2);
        assert_eq!(plan.lighting_connections.len(), 1);
        assert!(plan.lighting_connection_is_valid(&plan.lighting_connections[0]));
        assert_eq!(plan.symbol_number(0), Some(1));
        assert_eq!(plan.symbol_number(1), Some(1));
        assert_eq!(plan.unlinked_lights(), 0);
    }

    #[test]
    fn refuses_wrong_element_types_and_duplicate_light() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        assert!(plan.connect_switch_to_light(0, 1).is_err());
        assert!(plan.connect_switch_to_light(1, 0).is_err());
        assert!(plan.connect_switch_to_light(3, 1).is_err());
        let unlinked = plan.symbols.len();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 0.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        assert_eq!(plan.unlinked_lights(), 1);
        plan.connect_switch_to_light(0, unlinked).unwrap();
        assert_eq!(plan.unlinked_lights(), 0);
    }

    #[test]
    fn saved_lighting_connections_restore_correctly() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        let restored: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(restored.lighting_connections, plan.lighting_connections);
        assert!(restored.lighting_connection_is_valid(&restored.lighting_connections[0]));
    }

    #[test]
    fn old_json_without_connections_still_loads() {
        let old = r#"{"walls":[],"symbols":[{"position":{"x":0.0,"y":0.0},"kind":"Light"}]}"#;
        let loaded: Plan = serde_json::from_str(old).unwrap();
        assert!(loaded.lighting_connections.is_empty());
        assert_eq!(loaded.unlinked_lights(), 1);
    }

    #[test]
    fn circuit_metadata_stays_unspecified_until_user_fills_it() {
        let circuit = LightingCircuit::default();
        assert_eq!(circuit.letter, "A");
        assert_eq!(circuit.breaker_amp, 0);
        assert!(!circuit.missing_fields().is_empty());
        assert!(circuit.cable_label().contains("renseigner"));
    }

    #[test]
    fn groups_share_one_switch_and_keep_all_lamps() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 1.0, y: 4.0 },
            kind: ElectricalKind::Light,
        });
        plan.connect_switch_to_light(0, 2).unwrap();
        assert_eq!(plan.lighting_groups(), vec![(0, vec![1, 2])]);
        assert_eq!(plan.unlinked_switches(), 0);
        let restored: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(restored.lighting_groups(), vec![(0, vec![1, 2])]);
    }

    #[test]
    fn old_plan_preserves_default_circuit() {
        let plan: Plan = serde_json::from_str(r#"{"walls":[],"symbols":[]}"#).unwrap();
        assert_eq!(plan.lighting_circuit.letter, "A");
        assert_eq!(plan.lighting_circuit.breaker_amp, 0);
    }

    #[test]
    fn plan_json_round_trip() {
        let mut plan = Plan::default();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 1.0, y: 2.0 },
            kind: ElectricalKind::Outlet,
        });
        let recovered: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(recovered.symbols.len(), 1);
    }
}
