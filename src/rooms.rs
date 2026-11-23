//! Detection automatique des pièces par graphe planaire de murs.
//! Les points d'intersection et les jonctions en T sont pris en compte.
//! Les surfaces sont des estimations 2D entre faces interieures des murs.

use crate::model::{Point, Wall};

const EPS: f32 = 0.001;

// Un mur peut sembler raccorde sur le dessin alors que son extremite
// s'arrete a quelques millimetres de l'axe d'une autre cloison. Ces
// micro-ecarts (< 2 cm) ne doivent pas faire disparaitre une piece.
//
// Ne pas augmenter EPS : cela fusionnerait aussi des intersections ou
// des noeuds non lies. Les raccords sont construits UNIQUEMENT pour le
// graphe de detection des pieces et ne changent pas le plan sauvegarde.
const ROOM_JOIN_TOLERANCE_M: f32 = 0.02;
const ROOM_JOIN_MAX_PARALLEL_COS: f32 = 0.45;
const MIN_AREA_M2: f32 = 0.05;
const MAX_WALLS: usize = 2000;

#[derive(Debug, Clone)]
pub struct DetectedRoom {
    /// Polygone ferme par les axes des murs (dernier sommet non repete).
    #[allow(dead_code)] // Contour brut preserve pour les futurs outils de selection et cotation.
    pub polygon: Vec<Point>,
    /// Polygone approxime des faces interieures des murs.
    pub inner_polygon: Vec<Point>,
    pub center: Point,
    pub area_m2: f32,
    /// Cotes interieures du rectangle ; sinon encombrement selon X et Y.
    pub width_m: f32,
    pub height_m: f32,
    pub rectangular: bool,
}

impl DetectedRoom {
    pub fn contains(&self, point: Point) -> bool {
        point_inside(&self.inner_polygon, point)
    }
}

#[derive(Clone, Copy)]
struct Edge {
    a: usize,
    b: usize,
    thickness: f32,
}

#[derive(Clone, Copy)]
struct HalfEdge {
    from: usize,
    to: usize,
    edge_index: usize,
}

fn cross(a: Point, b: Point) -> f32 {
    a.x * b.y - a.y * b.x
}
fn delta(a: Point, b: Point) -> Point {
    Point { x: a.x - b.x, y: a.y - b.y }
}
fn add(a: Point, b: Point) -> Point {
    Point { x: a.x + b.x, y: a.y + b.y }
}
fn mul(a: Point, t: f32) -> Point {
    Point { x: a.x * t, y: a.y * t }
}
fn interpolate(a: Point, b: Point, t: f32) -> Point {
    add(a, mul(delta(b, a), t))
}
fn signed_area(points: &[Point]) -> f32 {
    if points.len() < 3 { return 0.0; }
    points.iter().enumerate().map(|(i, a)| {
        cross(*a, points[(i + 1) % points.len()])
    }).sum::<f32>() * 0.5
}
fn centroid(points: &[Point]) -> Option<Point> {
    let area = signed_area(points);
    if area.abs() < 0.000001 { return None; }
    let mut x = 0.0;
    let mut y = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let c = cross(a, b);
        x += (a.x + b.x) * c;
        y += (a.y + b.y) * c;
    }
    Some(Point { x: x / (6.0 * area), y: y / (6.0 * area) })
}

/// Ray casting. Les points exactement sur le contour ne servent pas au nommage.
pub fn point_inside(poly: &[Point], p: Point) -> bool {
    if poly.len() < 3 || !p.x.is_finite() || !p.y.is_finite() { return false; }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[j];
        if (a.y > p.y) != (b.y > p.y) {
            let x = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if p.x < x { inside = !inside; }
        }
        j = i;
    }
    inside
}
fn projected_on(segment: &Wall, p: Point) -> Option<f32> {
    let d = delta(segment.end, segment.start);
    let sq = d.x * d.x + d.y * d.y;
    if sq <= EPS * EPS { return None; }
    let t = ((p.x - segment.start.x) * d.x + (p.y - segment.start.y) * d.y) / sq;
    let closest = interpolate(segment.start, segment.end, t);
    if t >= -EPS && t <= 1.0 + EPS && p.distance_to(closest) <= EPS {
        Some(t.clamp(0.0, 1.0))
    } else {
        None
    }
}
fn intersection(a: &Wall, b: &Wall) -> Option<(f32, f32, Point)> {
    let p = a.start;
    let q = b.start;
    let r = delta(a.end, p);
    let s = delta(b.end, q);
    let den = cross(r, s);
    if den.abs() <= 0.00000001 { return None; }
    let qp = delta(q, p);
    let t = cross(qp, s) / den;
    let u = cross(qp, r) / den;
    if (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u) {
        let t = t.clamp(0.0, 1.0);
        let u = u.clamp(0.0, 1.0);
        Some((t, u, interpolate(a.start, a.end, t)))
    } else {
        None
    }
}
fn node_index(nodes: &mut Vec<Point>, point: Point) -> usize {
    if let Some(i) = nodes.iter().position(|p| p.distance_to(point) < EPS) { return i; }
    nodes.push(point);
    nodes.len() - 1
}
fn inner_polygon(poly: &[Point], thickness: &[f32]) -> Vec<Point> {
    let mut result = Vec::with_capacity(poly.len());
    let count = poly.len();
    for i in 0..count {
        let previous = (i + count - 1) % count;
        let p = poly[i];
        let prev_dir = delta(poly[i], poly[previous]);
        let next_dir = delta(poly[(i + 1) % count], p);
        let prev_len = (prev_dir.x * prev_dir.x + prev_dir.y * prev_dir.y).sqrt();
        let next_len = (next_dir.x * next_dir.x + next_dir.y * next_dir.y).sqrt();
        if prev_len < EPS || next_len < EPS { return Vec::new(); }
        let d1 = mul(prev_dir, 1.0 / prev_len);
        let d2 = mul(next_dir, 1.0 / next_len);
        let n1 = Point { x: -d1.y, y: d1.x };
        let n2 = Point { x: -d2.y, y: d2.x };
        let a = add(p, mul(n1, thickness[previous] * 0.5));
        let b = add(p, mul(n2, thickness[i] * 0.5));
        let divisor = cross(d1, d2);
        let interior = if divisor.abs() < 0.000001 {
            mul(add(a, b), 0.5)
        } else {
            add(a, mul(d1, cross(delta(b, a), d2) / divisor))
        };
        if !interior.x.is_finite() || !interior.y.is_finite() {
            return Vec::new();
        }
        result.push(interior);
    }
    result
}
fn is_rectangle(poly: &[Point]) -> bool {
    if poly.len() != 4 { return false; }
    (0..4).all(|i| {
        let a = delta(poly[(i + 1) % 4], poly[i]);
        let b = delta(poly[(i + 2) % 4], poly[(i + 1) % 4]);
        let la = (a.x * a.x + a.y * a.y).sqrt();
        let lb = (b.x * b.x + b.y * b.y).sqrt();
        la > EPS && lb > EPS && ((a.x*b.x+a.y*b.y)/(la*lb)).abs() < 0.01
    })
}
fn dimension_bounds(poly: &[Point]) -> (f32, f32) {
    let minx = poly.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let maxx = poly.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let miny = poly.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let maxy = poly.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    (maxx - minx, maxy - miny)
}

/// Retourne de tres courts segments virtuels qui relient une extremite
/// de mur a une cloison presque touchee, a condition que les deux murs
/// se rencontrent avec un angle suffisant (raccord en T).
///
/// Seuls les espaces <= 2 cm sont corriges. Une ouverture de passage
/// libre classique (35 cm ou plus) n'est JAMAIS refermee ici.
/// Les portes, fenetres, passages et murs originaux ne sont pas modifies.
fn tiny_room_junctions(walls: &[Wall]) -> Vec<Wall> {
    let mut joins = Vec::new();
    for (i, wall) in walls.iter().enumerate() {
        let len = wall.length_m();
        if len < 0.05 || !len.is_finite() { continue; }
        let direction = delta(wall.end, wall.start);
        for endpoint in [wall.start, wall.end] {
            let mut nearest: Option<(f32, Point, f32)> = None;
            for (j, other) in walls.iter().enumerate() {
                if i == j { continue; }
                let other_direction = delta(other.end, other.start);
                let sq = other_direction.x * other_direction.x
                    + other_direction.y * other_direction.y;
                if sq < 0.05_f32.powi(2) || !sq.is_finite() { continue; }
                let t = (delta(endpoint, other.start).x * other_direction.x
                    + delta(endpoint, other.start).y * other_direction.y) / sq;
                // Seules les jonctions en T SUR un mur sont raccordees.
                // Les murs paralleles rapproches restent deux murs.
                if t <= 0.01 || t >= 0.99 { continue; }
                let projected = interpolate(other.start, other.end, t);
                let gap = endpoint.distance_to(projected);
                if gap <= EPS || gap > ROOM_JOIN_TOLERANCE_M || !gap.is_finite() {
                    continue;
                }
                let dot = direction.x * other_direction.x
                    + direction.y * other_direction.y;
                let parallel_cos = (dot / (len * sq.sqrt())).abs();
                if parallel_cos > ROOM_JOIN_MAX_PARALLEL_COS { continue; }
                if nearest.as_ref().is_none_or(|current| gap < current.0) {
                    nearest = Some((
                        gap,
                        projected,
                        (wall.thickness_m.max(0.0) + other.thickness_m.max(0.0)) * 0.5,
                    ));
                }
            }
            if let Some((_, connected_at, thickness_m)) = nearest {
                joins.push(Wall {
                    start: endpoint,
                    end: connected_at,
                    thickness_m,
                });
            }
        }
    }
    joins
}

/// Detection des pieces avec correction topologique de micro-ecarts de
/// dessin. Ne modifie jamais les coordonnees des murs du document.
pub fn detect_rooms(walls: &[Wall]) -> Vec<DetectedRoom> {
    if walls.is_empty() || walls.len() > MAX_WALLS {
        return Vec::new();
    }
    let joins = tiny_room_junctions(walls);
    if joins.is_empty() {
        return detect_rooms_strict(walls);
    }
    let mut topology = Vec::with_capacity((walls.len() + joins.len()).min(MAX_WALLS));
    topology.extend_from_slice(walls);
    topology.extend(joins.into_iter().take(MAX_WALLS - walls.len()));
    detect_rooms_strict(&topology)
}

/// Graphe planaire strict, utilise apres construction des jonctions virtuelles.
fn detect_rooms_strict(walls: &[Wall]) -> Vec<DetectedRoom> {
    if walls.is_empty() || walls.len() > MAX_WALLS {
        return Vec::new();
    }
    let mut cuts: Vec<Vec<(f32, Point)>> = walls.iter().map(|wall| {
        vec![(0.0, wall.start), (1.0, wall.end)]
    }).collect();

    // Inclut les croix et les raccords en T, meme si un mur n'a pas ete coupe a la main.
    for i in 0..walls.len() {
        for j in (i + 1)..walls.len() {
            if let Some((t, u, p)) = intersection(&walls[i], &walls[j]) {
                cuts[i].push((t, p));
                cuts[j].push((u, p));
            }
            for p in [walls[j].start, walls[j].end] {
                if let Some(t) = projected_on(&walls[i], p) { cuts[i].push((t, p)); }
            }
            for p in [walls[i].start, walls[i].end] {
                if let Some(t) = projected_on(&walls[j], p) { cuts[j].push((t, p)); }
            }
        }
    }

    let mut nodes: Vec<Point> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    for (i, cuts) in cuts.iter_mut().enumerate() {
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        cuts.dedup_by(|a, b| a.1.distance_to(b.1) <= EPS);
        for pair in cuts.windows(2) {
            if pair[0].1.distance_to(pair[1].1) < EPS { continue; }
            let a = node_index(&mut nodes, pair[0].1);
            let b = node_index(&mut nodes, pair[1].1);
            if a == b { continue; }
            let thickness = walls[i].thickness_m.max(0.0);
            if let Some(existing) = edges.iter_mut().find(|edge|
                (edge.a == a && edge.b == b) || (edge.a == b && edge.b == a)
            ) {
                existing.thickness = existing.thickness.max(thickness);
            } else {
                edges.push(Edge { a, b, thickness });
            }
        }
    }
    if edges.is_empty() { return Vec::new(); }

    let mut half_edges = Vec::with_capacity(edges.len() * 2);
    let mut neighbors = vec![Vec::<usize>::new(); nodes.len()];
    for (edge_index, edge) in edges.iter().enumerate() {
        let fwd = half_edges.len();
        half_edges.push(HalfEdge { from: edge.a, to: edge.b, edge_index });
        half_edges.push(HalfEdge { from: edge.b, to: edge.a, edge_index });
        neighbors[edge.a].push(fwd);
        neighbors[edge.b].push(fwd + 1);
    }
    for (node_index, adj) in neighbors.iter_mut().enumerate() {
        adj.sort_by(|&a, &b| {
            let point = nodes[node_index];
            let pa = nodes[half_edges[a].to];
            let pb = nodes[half_edges[b].to];
            let aa = (pa.y - point.y).atan2(pa.x - point.x);
            let ab = (pb.y - point.y).atan2(pb.x - point.x);
            aa.total_cmp(&ab)
        });
    }

    let mut visited = vec![false; half_edges.len()];
    let mut rooms = Vec::new();
    for start in 0..half_edges.len() {
        if visited[start] { continue; }
        let mut path = Vec::new();
        let mut thickness = Vec::new();
        let mut current = start;
        let mut closed = false;
        for _ in 0..=half_edges.len() {
            if visited[current] { break; }
            visited[current] = true;
            let edge = half_edges[current];
            path.push(nodes[edge.from]);
            thickness.push(edges[edge.edge_index].thickness);
            let outgoing = &neighbors[edge.to];
            let reverse = current ^ 1;
            let Some(position) = outgoing.iter().position(|&id| id == reverse) else {
                break;
            };
            // Traverse en gardant la region a gauche : premier rayon horaire
            // a partir du rayon de retour.
            let next = outgoing[(position + outgoing.len() - 1) % outgoing.len()];
            if next == start {
                closed = true;
                break;
            }
            current = next;
        }
        if !closed || path.len() < 3 || signed_area(&path) <= MIN_AREA_M2 {
            continue;
        }
        let interior = inner_polygon(&path, &thickness);
        let area = signed_area(&interior);
        if area <= MIN_AREA_M2 || !area.is_finite() { continue; }
        // Pour les concavites, un centroide peut sortir du polygone ; on
        // place alors l'etiquette a l'interieur en suivant un triangle valide.
        let mut label = centroid(&interior).unwrap_or(interior[0]);
        if !point_inside(&interior, label) {
            let mut found = false;
            for vertex in &interior {
                let candidate = interpolate(label, *vertex, 0.8);
                if point_inside(&interior, candidate) {
                    label = candidate;
                    found = true;
                    break;
                }
            }
            if !found { continue; }
        }
        let rectangular = is_rectangle(&path) && is_rectangle(&interior);
        let (width_m, height_m) = if rectangular {
            (interior[0].distance_to(interior[1]), interior[1].distance_to(interior[2]))
        } else {
            dimension_bounds(&interior)
        };
        rooms.push(DetectedRoom {
            polygon: path,
            inner_polygon: interior,
            center: label,
            area_m2: area,
            width_m,
            height_m,
            rectangular,
        });
    }
    rooms.sort_by(|a, b| {
        a.center.y.total_cmp(&b.center.y)
            .then(a.center.x.total_cmp(&b.center.x))
    });
    rooms
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wall(x1: f32, y1: f32, x2: f32, y2: f32, thick: f32) -> Wall {
        Wall {
            start: Point { x: x1, y: y1 },
            end: Point { x: x2, y: y2 },
            thickness_m: thick,
        }
    }
    fn square() -> Vec<Wall> {
        vec![
            wall(0.0, 0.0, 4.0, 0.0, 0.2),
            wall(4.0, 0.0, 4.0, 3.0, 0.2),
            wall(4.0, 3.0, 0.0, 3.0, 0.2),
            wall(0.0, 3.0, 0.0, 0.0, 0.2),
        ]
    }
    #[test]
    fn rectangle_with_thick_walls_uses_inside_dimensions() {
        let rooms = detect_rooms(&square());
        assert_eq!(rooms.len(), 1);
        let room = &rooms[0];
        assert!(room.rectangular);
        assert!((room.width_m - 3.8).abs() < 0.001);
        assert!((room.height_m - 2.8).abs() < 0.001);
        assert!((room.area_m2 - 10.64).abs() < 0.001);
        assert!(room.contains(Point { x: 2.0, y: 1.5 }));
    }
    #[test]
    fn open_room_not_detected() {
        let mut walls = square();
        walls.pop();
        assert!(detect_rooms(&walls).is_empty());
    }
    #[test]
    fn shared_wall_splits_two_rooms_with_t_junctions() {
        let mut walls = square();
        walls.push(wall(2.0, 0.0, 2.0, 3.0, 0.2));
        let rooms = detect_rooms(&walls);
        assert_eq!(rooms.len(), 2);
        for room in &rooms {
            assert!((room.area_m2 - 5.04).abs() < 0.01);
        }
    }
    #[test]
    fn crossing_partition_splits_four_rooms() {
        let mut walls = square();
        walls.push(wall(2.0, 0.0, 2.0, 3.0, 0.2));
        walls.push(wall(0.0, 1.5, 4.0, 1.5, 0.2));
        let rooms = detect_rooms(&walls);
        assert_eq!(rooms.len(), 4);
    }
    #[test]
    fn repo_example_has_two_named_rooms_and_one_lighting_link() {
        let plan: crate::model::Plan =
            serde_json::from_str(include_str!("../examples/maison-deux-pieces.json")).unwrap();
        let rooms = detect_rooms(&plan.walls);
        assert_eq!(rooms.len(), 2);
        let names: Vec<&str> = rooms.iter().map(|room| plan.room_name(room)).collect();
        assert!(names.contains(&"Salon"));
        assert!(names.contains(&"Cuisine"));
        for room in &rooms {
            assert!((room.area_m2 - 10.64).abs() < 0.01);
        }
        assert_eq!(plan.lighting_connections.len(), 1);
        assert!(plan.lighting_connection_is_valid(&plan.lighting_connections[0]));
    }

    #[test]
    fn removing_a_wall_reopens_room() {
        let mut plan = crate::model::Plan::default();
        plan.walls = square();
        assert_eq!(detect_rooms(&plan.walls).len(), 1);
        plan.remove_wall(0).unwrap();
        assert!(detect_rooms(&plan.walls).is_empty());
    }

    #[test]
    fn rectangular_detection_independent_of_wall_directions() {
        let mut walls = square();
        walls[0] = wall(4.0, 0.0, 0.0, 0.0, 0.2);
        walls.swap(1, 3);
        assert_eq!(detect_rooms(&walls).len(), 1);
    }
}
