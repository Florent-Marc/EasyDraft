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
        format!("{} {} x {} mm2", self.cable_type.trim(), self.conductor_count, self.section_mm2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomLabel {
    /// Position ancree a l'interieur de la piece.
    pub position: Point,
    pub name: String,
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
    /// Les noms appartiennent a un point interieur, pas a un index de murs fragile.
    #[serde(default)]
    pub room_labels: Vec<RoomLabel>,
}

impl Plan {
    pub fn room_name<'a>(&'a self, room: &crate::rooms::DetectedRoom) -> &'a str {
        self.room_labels.iter()
            .find(|label| room.contains(label.position))
            .map(|label| label.name.as_str())
            .unwrap_or("")
    }

    /// Modifie le nom de la piece contenant le point indique.
    /// Une chaine vide retire le nom, sans supprimer la piece geometrique.
    pub fn set_room_name(&mut self, at: Point, name: &str)
        -> Result<(), &'static str>
    {
        let rooms = crate::rooms::detect_rooms(&self.walls);
        let room = rooms.iter().find(|room| room.contains(at))
            .ok_or("Aucune piece fermee a cet endroit")?;
        if name.chars().count() > 60 {
            return Err("Nom limite a 60 caracteres");
        }
        let title = name.trim();
        // Eviter deux labels concurrents si des pieces ont ete fusionnees.
        let mut first = None;
        for (index, label) in self.room_labels.iter().enumerate() {
            if room.contains(label.position) {
                first = Some(index);
                break;
            }
        }
        if title.is_empty() {
            self.room_labels.retain(|label| !room.contains(label.position));
        } else if let Some(index) = first {
            self.room_labels[index].name = title.to_owned();
            self.room_labels[index].position = room.center;
            let anchor = room.center;
            let mut has_first = false;
            self.room_labels.retain(|label| {
                if label.position == anchor {
                    if has_first { false } else { has_first = true; true }
                } else { !room.contains(label.position) }
            });
        } else {
            self.room_labels.push(RoomLabel {
                position: room.center,
                name: title.to_owned(),
            });
        }
        Ok(())
    }

    /// Change les cotes d'un mur sans casser ses ouvertures existantes.
    /// Les ouvertures restent mesurees depuis le point de depart du mur.
    pub fn update_wall(&mut self, index: usize, start: Point, end: Point, thickness_m: f32)
        -> Result<(), &'static str>
    {
        let wall = self.walls.get(index).ok_or("Mur introuvable")?;
        let valid = [start.x, start.y, end.x, end.y, thickness_m]
            .into_iter().all(f32::is_finite);
        if !valid || !(0.05..=0.60).contains(&thickness_m) {
            return Err("Coordonnees ou epaisseur invalides (5 a 60 cm)");
        }
        let length = start.distance_to(end);
        if length < 0.05 {
            return Err("Un mur doit mesurer au moins 5 cm");
        }
        if self.openings.iter().any(|opening| {
            opening.wall_index == index
                && (opening.start_m() < 0.049
                    || opening.end_m() + 0.049 > length)
        }) {
            return Err("Une porte ou fenetre depasserait du mur");
        }
        let _ = wall;
        self.walls[index] = Wall { start, end, thickness_m };
        Ok(())
    }

    /// Supprime un mur et ses ouvertures, puis reajuste les references des autres murs.
    pub fn remove_wall(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.walls.len() {
            return Err("Mur introuvable");
        }
        self.walls.remove(index);
        self.openings.retain(|opening| opening.wall_index != index);
        for opening in &mut self.openings {
            if opening.wall_index > index {
                opening.wall_index -= 1;
            }
        }
        Ok(())
    }

    /// Supprime un symbole electrique et reindexe les liaisons survivantes.
    pub fn remove_symbol(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.symbols.len() {
            return Err("Symbole introuvable");
        }
        self.symbols.remove(index);
        self.lighting_connections.retain(|link|
            link.switch_index != index && link.light_index != index
        );
        for link in &mut self.lighting_connections {
            if link.switch_index > index { link.switch_index -= 1; }
            if link.light_index > index { link.light_index -= 1; }
        }
        Ok(())
    }

    pub fn remove_stair(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.stairs.len() {
            return Err("Escalier introuvable");
        }
        self.stairs.remove(index);
        Ok(())
    }

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

    /// Vue sans doublons des connexions valides. Un fichier JSON mal forme ne
    /// doit pas produire deux branches pour une meme lampe.
    pub fn active_lighting_connections(&self) -> Vec<&LightingConnection> {
        let mut seen_lights = std::collections::HashSet::new();
        self.lighting_connections.iter()
            .filter(|link| {
                self.lighting_connection_is_valid(link) && seen_lights.insert(link.light_index)
            })
            .collect()
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

    /// Numero d'ordre partage par le plan 2D, l'unifilaire et le PDF.
    pub fn lighting_point_number(&self, light_index: usize) -> Option<usize> {
        self.active_lighting_connections()
            .iter()
            .position(|link| link.light_index == light_index)
            .map(|index| index + 1)
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
        assert_eq!(plan.active_lighting_connections().len(), 2);
        assert_eq!(plan.unlinked_switches(), 0);
        assert_eq!(plan.lighting_point_number(1), Some(1));
        assert_eq!(plan.lighting_point_number(2), Some(2));
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
    fn removing_wall_cleans_openings_and_shifts_indices() {
        let mut plan = Plan::default();
        for x in [0.0, 4.0, 8.0] {
            plan.walls.push(Wall {
                start: Point { x, y: 0.0 },
                end: Point { x: x + 3.0, y: 0.0 },
                thickness_m: 0.15,
            });
        }
        for wall_index in [0, 1, 2] {
            plan.add_opening(
                wall_index,
                Point { x: wall_index as f32 * 4.0 + 1.5, y: 0.0 },
                0.9, OpeningKind::Door, false, false, 0.05,
            ).unwrap();
        }
        plan.remove_wall(1).unwrap();
        assert_eq!(plan.walls.len(), 2);
        assert_eq!(plan.openings.len(), 2);
        assert_eq!(plan.openings[0].wall_index, 0);
        assert_eq!(plan.openings[1].wall_index, 1);
        assert!(plan.remove_wall(4).is_err());
    }

    #[test]
    fn resizing_wall_preserves_or_rejects_openings() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.15,
        });
        plan.add_opening(
            0, Point { x: 3.0, y: 0.0 }, 0.9,
            OpeningKind::Window, false, false, 0.05
        ).unwrap();
        let start = Point { x: 0.0, y: 0.0 };
        assert!(plan.update_wall(0, start, Point { x: 2.0, y: 0.0 }, 0.20).is_err());
        assert_eq!(plan.walls[0].length_m(), 4.0);
        plan.update_wall(0, start, Point { x: 5.0, y: 0.0 }, 0.20).unwrap();
        assert_eq!(plan.walls[0].thickness_m, 0.20);
        assert_eq!(plan.openings.len(), 1);
    }

    #[test]
    fn deleting_switch_cleans_and_reindexes_lighting_links() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        let switch2 = plan.symbols.len();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 10.0, y: 0.0 },
            kind: ElectricalKind::Switch,
        });
        let light2 = plan.symbols.len();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 11.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        plan.connect_switch_to_light(switch2, light2).unwrap();
        plan.remove_symbol(0).unwrap();
        assert_eq!(plan.lighting_connections.len(), 1);
        assert_eq!(plan.lighting_connections[0].switch_index, 1);
        assert_eq!(plan.lighting_connections[0].light_index, 2);
        assert!(plan.lighting_connection_is_valid(&plan.lighting_connections[0]));
        assert!(plan.remove_symbol(500).is_err());
        let saved: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(saved.lighting_connections, plan.lighting_connections);
    }

    #[test]
    fn removing_stair_keeps_other_elements() {
        let mut plan = Plan::default();
        plan.stairs.push(Stair {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 2.0, y: 0.0 },
            width_m: 1.0, steps: 10,
        });
        plan.remove_stair(0).unwrap();
        assert!(plan.stairs.is_empty());
        assert!(plan.remove_stair(0).is_err());
    }

    #[test]
    fn room_name_survives_json_and_prior_plans_remain_compatible() {
        let mut plan = Plan::default();
        let coords = [(0.0, 0.0, 4.0, 0.0), (4.0, 0.0, 4.0, 3.0),
            (4.0, 3.0, 0.0, 3.0), (0.0, 3.0, 0.0, 0.0)];
        for (x1, y1, x2, y2) in coords {
            plan.walls.push(Wall {
                start: Point { x: x1, y: y1 },
                end: Point { x: x2, y: y2 },
                thickness_m: 0.2,
            });
        }
        plan.set_room_name(Point { x: 1.0, y: 1.0 }, "Salon").unwrap();
        let rooms = crate::rooms::detect_rooms(&plan.walls);
        assert_eq!(plan.room_name(&rooms[0]), "Salon");
        let restored: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(restored.room_name(&rooms[0]), "Salon");
        let old: Plan = serde_json::from_str(r#"{"walls":[],"symbols":[]}"#).unwrap();
        assert!(old.room_labels.is_empty());
        assert!(plan.set_room_name(Point { x: 8.0, y: 8.0 }, "Cuisine").is_err());
        plan.set_room_name(Point { x: 1.0, y: 1.0 }, "").unwrap();
        assert_eq!(plan.room_name(&rooms[0]), "");
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
