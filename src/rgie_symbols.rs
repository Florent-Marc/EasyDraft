//! Bibliotheque de signes graphiques de base du RGIE Livre 1, tableau 2.23.
//! Les formes sont definies une seule fois pour l'affichage et les deux PDFs.
//! Source officielle (SPF Economie): https://economie.fgov.be/sites/default/files/Files/Energy/Symboles-fr.pdf
//! Ce module ne garantit pas qu'un projet electrique est conforme au RGIE.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use crate::model::{ElectricalAppearance, ElectricalKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Line((f32, f32), (f32, f32)),
    Circle((f32, f32), f32),
    /// Arc entre deux angles en radians (Y positif vers le bas).
    Arc((f32, f32), f32, f32, f32),
    Dot((f32, f32), f32),
}

fn line(from: (f32, f32), to: (f32, f32)) -> Primitive {
    Primitive::Line(from, to)
}

/// Formes RGIE : voir parties E, F et G du tableau 2.23.
/// Origine au centre, largeur nominale ~40 unites.
pub fn device_primitives(kind: ElectricalKind, appearance: ElectricalAppearance) -> Vec<Primitive> {
    let style = if appearance.valid_for(kind) {
        appearance
    } else {
        ElectricalAppearance::General
    };
    match kind {
        ElectricalKind::Light => {
            let mut marks = vec![
                line((-11.0, -11.0), (11.0, 11.0)),
                line((-11.0, 11.0), (11.0, -11.0)),
            ];
            if style == ElectricalAppearance::WallLight {
                marks.push(line((17.0, -15.0), (17.0, 15.0)));
            }
            marks
        }
        ElectricalKind::Switch => {
            if style == ElectricalAppearance::PushButton {
                return vec![
                    Primitive::Circle((0.0, 0.0), 11.0),
                    Primitive::Circle((0.0, 0.0), 4.2),
                ];
            }
            let mut marks = vec![
                Primitive::Circle((-2.0, 7.0), 5.8),
                line((0.0, 1.5), (8.0, -14.0)),
            ];
            match style {
                ElectricalAppearance::SwitchBipolar => {
                    marks.push(line((5.0, -14.0), (15.0, -10.0)));
                    marks.push(line((6.5, -17.5), (16.5, -13.5)));
                }
                ElectricalAppearance::SwitchTwoWay => {
                    marks.push(line((8.0, -14.0), (15.0, -11.0)));
                    marks.push(line((-4.0, 12.8), (-9.0, 23.0)));
                    marks.push(line((-9.0, 23.0), (-16.0, 20.0)));
                }
                ElectricalAppearance::SwitchPull => {
                    marks.push(line((0.0, 12.0), (0.0, 25.0)));
                    marks.push(Primitive::Circle((0.0, 28.0), 3.0));
                }
                _ => {}
            }
            marks
        }
        ElectricalKind::Outlet => {
            let mut marks = vec![
                // Canalisation et demi-cercle du socle de prise, partie F.
                line((-23.0, -10.0), (0.0, -10.0)),
                line((0.0, -10.0), (0.0, 0.0)),
                Primitive::Arc((0.0, 10.0), 10.0, std::f32::consts::PI, std::f32::consts::TAU),
            ];
            if style == ElectricalAppearance::OutletTriple {
                for y in [8.0, 16.0] {
                    marks.push(Primitive::Arc((0.0, 10.0 + y), 10.0,
                        std::f32::consts::PI, std::f32::consts::TAU));
                }
                marks.push(line((0.0, 0.0), (0.0, 16.0)));
            }
            if matches!(style, ElectricalAppearance::OutletEarth |
                ElectricalAppearance::OutletEarthChild) {
                marks.push(line((-12.5, -0.3), (12.5, -0.3)));
            }
            if matches!(style, ElectricalAppearance::OutletChild |
                ElectricalAppearance::OutletEarthChild) {
                marks.push(line((-13.5, 10.0), (-7.0, 10.0)));
                marks.push(line((7.0, 10.0), (13.5, 10.0)));
            }
            marks
        }
    }
}

/// Signes pour une branche horizontale (amont à gauche).
pub fn schematic_device_primitives(
    kind: ElectricalKind, appearance: ElectricalAppearance,
) -> Vec<Primitive> {
    match kind {
        ElectricalKind::Light => device_primitives(kind, appearance),
        ElectricalKind::Switch => device_primitives(kind, appearance).into_iter().map(|mark| {
            if appearance == ElectricalAppearance::PushButton {
                return mark;
            }
            let center = |(x, y): (f32, f32)| (x + 2.0, y - 7.0);
            match mark {
                Primitive::Line(a, b) => Primitive::Line(center(a), center(b)),
                Primitive::Circle(at, radius) => Primitive::Circle(center(at), radius),
                other => other,
            }
        }).collect(),
        ElectricalKind::Outlet => {
            let rows: &[f32] = if appearance == ElectricalAppearance::OutletTriple {
                &[-12.0, 0.0, 12.0]
            } else {
                &[0.0]
            };
            let mut marks = vec![line((0.0, rows[0] - 14.0), (0.0, rows[rows.len() - 1] + 12.0))];
            for &y in rows {
                marks.push(Primitive::Arc((0.0, y - 2.0), 10.0, 0.0, std::f32::consts::PI));
                marks.push(line((-14.0, y - 2.0), (-10.0, y - 2.0)));
                marks.push(line((10.0, y - 2.0), (14.0, y - 2.0)));
                marks.push(line((-10.0, y + 7.0), (10.0, y + 7.0)));
            }
            if matches!(appearance, ElectricalAppearance::OutletEarth | ElectricalAppearance::OutletEarthChild) {
                marks.push(line((-7.0, 12.0), (7.0, 12.0)));
            }
            if matches!(appearance, ElectricalAppearance::OutletChild | ElectricalAppearance::OutletEarthChild) {
                marks.push(line((-7.0, -6.0), (7.0, -6.0)));
            }
            marks
        }
    }
}

/// Distance entre le centre d'une commande et son bord de raccord.
pub fn schematic_switch_radius(appearance: ElectricalAppearance) -> f32 {
    if appearance == ElectricalAppearance::PushButton { 11.0 } else { 5.8 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionMark {
    Breaker,
    Fuse,
    Unspecified,
}

pub fn protection_from_text(value: &str) -> ProtectionMark {
    let lower = value.to_lowercase();
    if lower.contains("disjoncteur") || lower.contains("interrupteur automatique") {
        ProtectionMark::Breaker
    } else if lower.contains("fusible") || lower.contains("coupe-circuit") {
        ProtectionMark::Fuse
    } else {
        ProtectionMark::Unspecified
    }
}

/// Representation en orientation verticale telle que dans le tableau 2.23, partie D.
/// La protection est signalee comme inconnue si le type n'est pas renseigne.
pub fn protection_primitives(kind: ProtectionMark) -> Vec<Primitive> {
    match kind {
        ProtectionMark::Breaker => vec![
            line((0.0, -19.0), (0.0, -8.0)),
            line((0.0, 8.0), (0.0, 20.0)),
            line((0.0, 8.0), (-10.0, -7.0)),
            Primitive::Dot((-10.0, -8.0), 3.0),
        ],
        ProtectionMark::Fuse => vec![
            line((0.0, -21.0), (0.0, -9.0)),
            line((0.0, 9.0), (0.0, 21.0)),
            line((-7.0, -9.0), (7.0, -9.0)),
            line((7.0, -9.0), (7.0, 9.0)),
            line((7.0, 9.0), (-7.0, 9.0)),
            line((-7.0, 9.0), (-7.0, -9.0)),
        ],
        ProtectionMark::Unspecified => vec![
            line((-12.0, -12.0), (12.0, -12.0)),
            line((12.0, -12.0), (12.0, 12.0)),
            line((12.0, 12.0), (-12.0, 12.0)),
            line((-12.0, 12.0), (-12.0, -12.0)),
        ],
    }
}

/// Orientation d'un signe de protection sur un depart de gauche a droite.
/// Les signes du tableau 2.23 sont souvent imprimes verticalement.
pub fn horizontal_protection_primitives(kind: ProtectionMark) -> Vec<Primitive> {
    let rotate = |p: (f32, f32)| (-p.1, p.0);
    protection_primitives(kind).into_iter().map(|primitive| match primitive {
        Primitive::Line(a, b) => Primitive::Line(rotate(a), rotate(b)),
        Primitive::Circle(c, r) => Primitive::Circle(rotate(c), r),
        Primitive::Dot(c, r) => Primitive::Dot(rotate(c), r),
        Primitive::Arc(c, r, a, b) => Primitive::Arc(
            rotate(c), r, a + std::f32::consts::FRAC_PI_2,
            b + std::f32::consts::FRAC_PI_2,
        ),
    }).collect()
}

pub fn arc_points(center: (f32, f32), r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
    let steps = 22;
    (0..=steps).map(|n| {
        let angle = from + (to - from) * (n as f32 / steps as f32);
        (center.0 + r * angle.cos(), center.1 + r * angle.sin())
    }).collect()
}

/// Affichage ecran du meme catalogue utilise par le PDF.
pub fn draw_egui(
    painter: &egui::Painter,
    origin: Pos2,
    primitives: &[Primitive],
    color: Color32,
    scale: f32,
) {
    let scaled = |p: (f32, f32)| origin + Vec2::new(p.0 * scale, p.1 * scale);
    let stroke = Stroke::new(1.75 * scale.max(0.5), color);
    for primitive in primitives {
        match *primitive {
            Primitive::Line(a, b) => {
                painter.line_segment([scaled(a), scaled(b)], stroke);
            }
            Primitive::Circle(center, radius) => {
                painter.circle_stroke(scaled(center), radius * scale, stroke);
            }
            Primitive::Arc(center, radius, start, end) => {
                let points: Vec<Pos2> = arc_points(center, radius, start, end)
                    .into_iter().map(scaled).collect();
                painter.add(egui::Shape::line(points, stroke));
            }
            Primitive::Dot(center, radius) => {
                painter.circle_filled(scaled(center), radius * scale, color);
            }
        }
    }
}

/// Formes toujours partagees : une seule definition des traits pour les PDFs.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn symbols_cover_each_device_kind_with_distinct_shapes() {
        for kind in [ElectricalKind::Switch, ElectricalKind::Outlet, ElectricalKind::Light] {
            assert!(!device_primitives(kind, ElectricalAppearance::General).is_empty());
        }
        assert_ne!(
            device_primitives(ElectricalKind::Outlet, ElectricalAppearance::OutletEarthChild),
            device_primitives(ElectricalKind::Outlet, ElectricalAppearance::General)
        );
        assert_ne!(
            device_primitives(ElectricalKind::Switch, ElectricalAppearance::PushButton),
            device_primitives(ElectricalKind::Switch, ElectricalAppearance::General)
        );
    }
    #[test]
    fn no_unrecorded_protection_symbol_is_invented() {
        assert_eq!(protection_from_text(""), ProtectionMark::Unspecified);
        assert_eq!(protection_from_text("Fusible gG"), ProtectionMark::Fuse);
        assert_eq!(protection_from_text("Disjoncteur"), ProtectionMark::Breaker);
    }
    #[test]
    fn breaker_and_fuse_can_be_oriented_on_horizontal_departure() {
        for mark in [ProtectionMark::Breaker, ProtectionMark::Fuse] {
            let originals = protection_primitives(mark);
            let horizontal = horizontal_protection_primitives(mark);
            assert_eq!(originals.len(), horizontal.len());
            assert_ne!(originals, horizontal);
        }
    }

    #[test]
    fn arcs_have_stable_endpoints() {
        let arc = arc_points((0.0, 10.0), 10.0, std::f32::consts::PI, std::f32::consts::TAU);
        assert!((arc[0].0 + 10.0).abs() < 0.001);
        assert!((arc[22].0 - 10.0).abs() < 0.001);
    }
}
