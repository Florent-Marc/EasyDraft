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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub walls: Vec<Wall>,
    pub symbols: Vec<ElectricalSymbol>,
    /// La valeur par défaut garde la compatibilité avec les plans JSON anciens.
    #[serde(default)]
    pub openings: Vec<Opening>,
}

impl Plan {
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
