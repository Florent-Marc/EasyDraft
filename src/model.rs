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
