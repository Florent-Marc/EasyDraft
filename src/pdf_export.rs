//! Export PDF vectoriel A4 paysage, sans dependance supplementaire.
//! Le plan est adapte a la page (il n'est pas imprime a une echelle architecturale fixe).

use std::fmt::Write as _;

use crate::model::{ElectricalKind, Opening, OpeningKind, Plan, Point, Stair, Wall};

const PAGE_W: f32 = 842.0; // A4 paysage en points PDF
const PAGE_H: f32 = 595.0;
const MARGIN: f32 = 32.0;
const TOP: f32 = 65.0;
const BOTTOM: f32 = 54.0;

type Rgb = (f32, f32, f32);
const DARK: Rgb = (0.18, 0.22, 0.27);
const BLUE: Rgb = (0.11, 0.45, 0.72);
const GLASS: Rgb = (0.12, 0.56, 0.75);
const DIM: Rgb = (0.31, 0.43, 0.60);

struct Canvas {
    commands: String,
    scale: f32,
    tx: f32,
    ty: f32,
}

impl Canvas {
    fn new(plan: &Plan) -> Self {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut include = |p: Point, padding: f32| {
            if p.x.is_finite() && p.y.is_finite() {
                min_x = min_x.min(p.x - padding);
                max_x = max_x.max(p.x + padding);
                min_y = min_y.min(p.y - padding);
                max_y = max_y.max(p.y + padding);
            }
        };
        for wall in &plan.walls {
            // Inclut egalement la place pour les lignes de cote.
            let padding = wall.thickness_m.max(0.0) / 2.0 + 0.5;
            include(wall.start, padding);
            include(wall.end, padding);
        }
        for stair in &plan.stairs {
            if stair.is_valid() {
                for p in stair.corners() {
                    include(p, 0.3);
                }
            }
        }
        for symbol in &plan.symbols {
            include(symbol.position, 0.25);
        }
        if !min_x.is_finite() {
            min_x = -1.0;
            max_x = 1.0;
            min_y = -1.0;
            max_y = 1.0;
        }
        let width = (max_x - min_x).max(1.0);
        let height = (max_y - min_y).max(1.0);
        let available_w = PAGE_W - MARGIN * 2.0;
        let available_h = PAGE_H - TOP - BOTTOM;
        let scale = (available_w / width).min(available_h / height);
        let tx = MARGIN + (available_w - width * scale) / 2.0 - min_x * scale;
        let ty = BOTTOM + (available_h - height * scale) / 2.0 + max_y * scale;
        Self { commands: String::new(), scale, tx, ty }
    }

    fn xy(&self, p: Point) -> (f32, f32) {
        (self.tx + p.x * self.scale, self.ty - p.y * self.scale)
    }

    fn line(&mut self, a: Point, b: Point, width_pt: f32, color: Rgb) {
        let (x1, y1) = self.xy(a);
        let (x2, y2) = self.xy(b);
        let _ = writeln!(
            self.commands,
            "{:.3} {:.3} {:.3} RG {:.3} w {:.3} {:.3} m {:.3} {:.3} l S",
            color.0, color.1, color.2,
            width_pt.max(0.2), x1, y1, x2, y2
        );
    }

    fn text_pt(&mut self, x: f32, y: f32, size: f32, text: &str, color: Rgb) {
        let safe = text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
        let _ = writeln!(
            self.commands,
            "BT /F1 {:.2} Tf {:.3} {:.3} {:.3} rg {:.3} {:.3} Td ({}) Tj ET",
            size, color.0, color.1, color.2, x, y, safe
        );
    }

    fn centered_text(&mut self, at: Point, size: f32, text: &str, color: Rgb) {
        let (x, y) = self.xy(at);
        // Approximation de la largeur en Helvetica, suffisante pour les cotes.
        let approximate_width = text.chars().count() as f32 * size * 0.49;
        self.text_pt(x - approximate_width * 0.5, y - size * 0.33, size, text, color);
    }

    fn circle(&mut self, center: Point, radius: f32, color: Rgb) {
        let (x, y) = self.xy(center);
        let r = radius * self.scale;
        let k = r * 0.55228475;
        let _ = writeln!(
            self.commands,
            "{:.3} {:.3} {:.3} RG 1 w {:.3} {:.3} m \
             {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c \
             {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c \
             {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c \
             {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c S",
            color.0, color.1, color.2,
            x+r, y,
            x+r, y+k, x+k, y+r, x, y+r,
            x-k, y+r, x-r, y+k, x-r, y,
            x-r, y-k, x-k, y-r, x, y-r,
            x+k, y-r, x+r, y-k, x+r, y
        );
    }

    fn wall_segment(&mut self, a: Point, b: Point, thickness_m: f32) {
        self.line(a, b, thickness_m.max(0.01) * self.scale, DARK);
    }

    fn wall(&mut self, index: usize, wall: &Wall, openings: &[Opening], show_dimensions: bool) {
        let length = wall.length_m();
        if !length.is_finite() || length < 0.001 {
            return;
        }
        let dx = (wall.end.x - wall.start.x) / length;
        let dy = (wall.end.y - wall.start.y) / length;
        let along = |distance: f32| Point {
            x: wall.start.x + dx * distance,
            y: wall.start.y + dy * distance,
        };
        let mut segments: Vec<&Opening> = openings.iter()
            .filter(|o| o.wall_index == index && o.width_m.is_finite() && o.width_m > 0.0
                && o.center_m.is_finite())
            .collect();
        segments.sort_by(|a, b| a.start_m().total_cmp(&b.start_m()));
        let mut cursor = 0.0_f32;
        for o in segments {
            let start = o.start_m().clamp(0.0, length);
            let end = o.end_m().clamp(0.0, length);
            if start > cursor {
                self.wall_segment(along(cursor), along(start), wall.thickness_m);
            }
            let half = wall.thickness_m / 2.0;
            let normal_x = -dy * half;
            let normal_y = dx * half;
            for edge in [start, end] {
                let p = along(edge);
                self.line(
                    Point { x: p.x - normal_x, y: p.y - normal_y },
                    Point { x: p.x + normal_x, y: p.y + normal_y },
                    1.2, DARK,
                );
            }
            match o.kind {
                OpeningKind::Window => {
                    for offset in [-0.3_f32, 0.0, 0.3] {
                        let shift = offset * wall.thickness_m;
                        let a = along(start);
                        let b = along(end);
                        self.line(
                            Point { x: a.x - dy * shift, y: a.y + dx * shift },
                            Point { x: b.x - dy * shift, y: b.y + dx * shift },
                            1.5, GLASS,
                        );
                    }
                }
                OpeningKind::Door => {
                    let hinge = if o.hinge_right { along(end) } else { along(start) };
                    let axial_sign = if o.hinge_right { -1.0 } else { 1.0 };
                    let swing_sign = if o.swing_flipped { -1.0 } else { 1.0 };
                    let radius = o.width_m;
                    let outer = Point {
                        x: hinge.x - dy * swing_sign * radius,
                        y: hinge.y + dx * swing_sign * radius,
                    };
                    self.line(hinge, outer, 1.5, BLUE);
                    let mut previous = Point {
                        x: hinge.x + dx * axial_sign * radius,
                        y: hinge.y + dy * axial_sign * radius,
                    };
                    for i in 1..=20 {
                        let angle = std::f32::consts::FRAC_PI_2 * i as f32 / 20.0;
                        let next = Point {
                            x: hinge.x + dx * axial_sign * radius * angle.cos()
                                - dy * swing_sign * radius * angle.sin(),
                            y: hinge.y + dy * axial_sign * radius * angle.cos()
                                + dx * swing_sign * radius * angle.sin(),
                        };
                        self.line(previous, next, 0.6, DIM);
                        previous = next;
                    }
                }
            }
            cursor = cursor.max(end);
        }
        if cursor < length {
            self.wall_segment(along(cursor), wall.end, wall.thickness_m);
        }
        if show_dimensions {
            let offset = wall.thickness_m.max(0.0) / 2.0 + 0.32;
            let shift = |p: Point| Point { x: p.x - dy * offset, y: p.y + dx * offset };
            let a = shift(wall.start);
            let b = shift(wall.end);
            self.line(a, b, 0.7, DIM);
            for end in [wall.start, wall.end] {
                let q = shift(end);
                self.line(end, q, 0.5, DIM);
                self.line(
                    Point { x: q.x - dx * 0.045 - dy * 0.045, y: q.y - dy * 0.045 + dx * 0.045 },
                    Point { x: q.x + dx * 0.045 + dy * 0.045, y: q.y + dy * 0.045 - dx * 0.045 },
                    0.9, DIM
                );
            }
            let mid = Point {
                x: (a.x + b.x) * 0.5 - dy * 0.13,
                y: (a.y + b.y) * 0.5 + dx * 0.13,
            };
            self.centered_text(mid, 9.0, &format!("{length:.2} m"), DIM);
        }
    }

    fn stair(&mut self, stair: &Stair) {
        if !stair.is_valid() { return; }
        let corners = stair.corners();
        for i in 0..4 {
            self.line(corners[i], corners[(i + 1) % 4], 1.4, DARK);
        }
        let left_start = corners[0];
        let left_end = corners[1];
        let right_end = corners[2];
        let right_start = corners[3];
        for i in 1..stair.steps {
            let t = i as f32 / stair.steps as f32;
            let interpolate = |a: Point, b: Point| Point {
                x: a.x + (b.x - a.x) * t,
                y: a.y + (b.y - a.y) * t,
            };
            self.line(
                interpolate(left_start, left_end),
                interpolate(right_start, right_end),
                0.75, DARK
            );
        }
        let mix = |t: f32| Point {
            x: stair.start.x + (stair.end.x - stair.start.x) * t,
            y: stair.start.y + (stair.end.y - stair.start.y) * t,
        };
        self.line(mix(0.2), mix(0.83), 1.0, BLUE);
        let end = mix(0.83);
        let len = stair.length_m();
        let dx = (stair.end.x - stair.start.x) / len;
        let dy = (stair.end.y - stair.start.y) / len;
        for sign in [-1.0_f32, 1.0] {
            self.line(
                end,
                Point {
                    x: end.x - dx * 0.18 + (-dy) * 0.1 * sign,
                    y: end.y - dy * 0.18 + dx * 0.1 * sign,
                },
                1.0, BLUE,
            );
        }
    }

    fn symbols(&mut self, plan: &Plan) {
        for s in &plan.symbols {
            self.circle(s.position, 0.13, BLUE);
            let text = match s.kind {
                ElectricalKind::Outlet => "P",
                ElectricalKind::Switch => "I",
                ElectricalKind::Light => "X",
            };
            self.centered_text(s.position, 10.0, text, BLUE);
        }
    }
}

fn build_pdf(stream: &str) -> Vec<u8> {
    let content = format!("<< /Length {} >>\nstream\n{}endstream", stream.len(), stream);
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.0} {:.0}] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            PAGE_W, PAGE_H
        ),
        content,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut bytes = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, object).as_bytes());
    }
    let xref_offset = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(), xref_offset
        ).as_bytes(),
    );
    bytes
}

/// Exporte en une page A4 paysage, mise a l'echelle pour tenir sur la page.
pub fn export_pdf(path: &str, plan: &Plan, show_dimensions: bool) -> Result<(), String> {
    let mut pdf = Canvas::new(plan);
    let _ = writeln!(pdf.commands, "0 J 0 j"); // extremites nettes et angles miter
    for (index, wall) in plan.walls.iter().enumerate() {
        pdf.wall(index, wall, &plan.openings, show_dimensions);
    }
    for stair in &plan.stairs {
        pdf.stair(stair);
    }
    pdf.symbols(plan);

    // Le texte du titre est ajoute en coordonnees de page, sans transformation du plan.
    pdf.text_pt(MARGIN, PAGE_H - 34.0, 18.0, "EasyDraft - Plan 2D", DARK);
    pdf.text_pt(MARGIN, 25.0, 9.0, "A4 paysage - Ajuste a la page (pas une echelle 1:50 ou 1:100)", DIM);
    pdf.text_pt(
        PAGE_W - 226.0, 25.0, 9.0,
        &format!(
            "{} murs  |  {} portes  |  {} fenetres  |  {} escaliers",
            plan.walls.len(),
            plan.openings.iter().filter(|o| o.kind == OpeningKind::Door).count(),
            plan.openings.iter().filter(|o| o.kind == OpeningKind::Window).count(),
            plan.stairs.len()
        ),
        DIM,
    );
    let bytes = build_pdf(&pdf.commands);
    std::fs::write(path, bytes).map_err(|err| format!("Export PDF impossible : {err}"))
}


/// PDF vectoriel du schema unifilaire de travail, distinct du plan architectural.
/// Plusieurs pages A4 paysage sont generees pour ne pas ecraser les circuits charges.
/// Les valeurs absentes restent explicitement "a renseigner".
pub fn export_unifilar_pdf(path: &str, plan: &Plan) -> Result<(), String> {
    let links = plan.active_lighting_connections();
    let pages_count = links.len().max(1).div_ceil(6);
    let circuit = &plan.lighting_circuit;
    let color = DARK;
    let mut pages = Vec::new();

    for page_index in 0..pages_count {
        let start = page_index * 6;
        let end = (start + 6).min(links.len());
        let mut pdf = String::new();
        raw_pdf_text(
            &mut pdf, 35.0, 559.0, 18.0,
            "EasyDraft - SCHEMA UNIFILAIRE (PROTOTYPE)", color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 540.0, 9.0,
            "Document de travail NON CONFORME / non valide RGIE", (0.68, 0.25, 0.10),
        );
        raw_pdf_text(
            &mut pdf, 35.0, 517.0, 12.0,
            &format!(
                "Circuit {}  |  Protection : {}",
                pdf_safe_label(&circuit.letter),
                if circuit.breaker_amp == 0 { "a renseigner".to_owned() }
                else { circuit.protection_label() }
            ),
            color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 499.0, 11.0,
            &format!("Canalisation : {}", circuit.cable_label()),
            color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 481.0, 10.0,
            &format!(
                "Pose : {}  |  DDR amont : {}",
                pdf_safe_label(&circuit.installation_method),
                pdf_safe_label(&circuit.upstream_rdc),
            ),
            color,
        );
        raw_pdf_text(
            &mut pdf, 760.0, 555.0, 10.0,
            &format!("{}/{}", page_index + 1, pages_count), color,
        );
        let missing_count = circuit.missing_fields().len();
        if missing_count > 0 {
            raw_pdf_text(
                &mut pdf, 35.0, 461.0, 10.0,
                &format!("ATTENTION : {missing_count} caracteristique(s) de circuit non renseignee(s)"),
                (0.67, 0.31, 0.08),
            );
        }

        // Le depart est dessine symboliquement. Le DDR non renseigne n'est pas invente.
        pdf_box(&mut pdf, 40.0, 390.0, 140.0, 42.0, color, 1.0);
        raw_pdf_text(&mut pdf, 55.0, 406.0, 11.0, "Tableau / depart", color);
        raw_pdf_line(&mut pdf, 180.0, 411.0, 207.0, 411.0, color, 1.6);
        raw_pdf_line(&mut pdf, 207.0, 411.0, 230.0, 430.0, color, 1.6);
        pdf_box(&mut pdf, 227.0, 427.0, 8.0, 8.0, color, 1.0);
        raw_pdf_line(&mut pdf, 248.0, 411.0, 310.0, 411.0, color, 1.6);
        raw_pdf_text(&mut pdf, 188.0, 379.0, 9.0, "Protection (representation simplifiee)", DIM);

        let first_y = 330.0;
        let last_y = if end > start {
            first_y - (end - start - 1) as f32 * 46.0
        } else {
            first_y
        };
        raw_pdf_line(&mut pdf, 310.0, 411.0, 310.0, last_y, color, 1.6);
        for (index, link) in links[start..end].iter().enumerate() {
            let y = first_y - index as f32 * 46.0;
            raw_pdf_line(&mut pdf, 310.0, y, 431.0, y, color, 1.6);
            // Interrupteur : contact ouvert.
            raw_pdf_line(&mut pdf, 431.0, y, 451.0, y + 13.0, BLUE, 1.5);
            raw_pdf_line(&mut pdf, 459.0, y, 478.0, y, color, 1.5);
            raw_pdf_line(&mut pdf, 478.0, y, 655.0, y, color, 1.6);
            raw_pdf_circle(&mut pdf, 670.0, y, 15.0, color);
            raw_pdf_line(&mut pdf, 660.0, y - 10.0, 680.0, y + 10.0, color, 1.5);
            raw_pdf_line(&mut pdf, 660.0, y + 10.0, 680.0, y - 10.0, color, 1.5);
            let i_number = plan.symbol_number(link.switch_index).unwrap_or(link.switch_index + 1);
            let l_number = plan.symbol_number(link.light_index).unwrap_or(link.light_index + 1);
            raw_pdf_text(&mut pdf, 425.0, y - 22.0, 10.0, &format!("I{i_number}"), color);
            raw_pdf_text(&mut pdf, 663.0, y - 22.0, 10.0, &format!("L{l_number}"), color);
            raw_pdf_text(
                &mut pdf, 330.0, y + 9.0, 9.0,
                &format!(
                    "{}.{}",
                    pdf_safe_label(&circuit.letter),
                    plan.lighting_point_number(link.light_index).unwrap_or(start + index + 1)
                ),
                DIM,
            );
        }
        if links.is_empty() {
            raw_pdf_text(
                &mut pdf, 335.0, 326.0, 11.0,
                "Aucune liaison interrupteur - lampe", DIM,
            );
        }
        raw_pdf_text(
            &mut pdf, 35.0, 58.0, 10.0,
            &format!(
                "{} lampe(s) sans liaison | {} interrupteur(s) sans liaison",
                plan.unlinked_lights(), plan.unlinked_switches()
            ),
            color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 38.0, 9.0,
            "Liaisons fonctionnelles, PAS un trajet de cable. A faire verifier avant toute utilisation technique.",
            DIM,
        );
        pages.push(pdf);
    }

    std::fs::write(path, build_pdf_pages(&pages))
        .map_err(|e| format!("Export du schema unifilaire impossible : {e}"))
}

fn pdf_safe_label(value: &str) -> &str {
    if value.trim().is_empty() { "a renseigner" } else { value.trim() }
}

fn raw_pdf_line(out: &mut String, x1: f32, y1: f32, x2: f32, y2: f32, rgb: Rgb, width: f32) {
    let _ = writeln!(
        out,
        "{:.3} {:.3} {:.3} RG {:.3} w {:.2} {:.2} m {:.2} {:.2} l S",
        rgb.0, rgb.1, rgb.2, width, x1, y1, x2, y2
    );
}

fn raw_pdf_text(out: &mut String, x: f32, y: f32, size: f32, text: &str, rgb: Rgb) {
    // Type1 Helvetica sans encodage Unicode. Les caracteres non ASCII sont remplaces.
    let ascii = text.chars().map(|ch| if ch.is_ascii() { ch } else { '?' }).collect::<String>();
    let escaped = ascii.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
    let _ = writeln!(
        out,
        "BT /F1 {:.1} Tf {:.3} {:.3} {:.3} rg {:.2} {:.2} Td ({}) Tj ET",
        size, rgb.0, rgb.1, rgb.2, x, y, escaped
    );
}

fn pdf_box(out: &mut String, x: f32, y: f32, w: f32, h: f32, rgb: Rgb, thickness: f32) {
    raw_pdf_line(out, x, y, x+w, y, rgb, thickness);
    raw_pdf_line(out, x+w, y, x+w, y+h, rgb, thickness);
    raw_pdf_line(out, x+w, y+h, x, y+h, rgb, thickness);
    raw_pdf_line(out, x, y+h, x, y, rgb, thickness);
}

fn raw_pdf_circle(out: &mut String, x: f32, y: f32, r: f32, rgb: Rgb) {
    let k = 0.55228475 * r;
    let _ = writeln!(
        out,
        "{:.3} {:.3} {:.3} RG 1.2 w \
         {:.2} {:.2} m \
         {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c \
         {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c \
         {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c \
         {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c S",
        rgb.0, rgb.1, rgb.2,
        x+r, y,
        x+r, y+k, x+k, y+r, x, y+r,
        x-k, y+r, x-r, y+k, x-r, y,
        x-r, y-k, x-k, y-r, x, y-r,
        x+k, y-r, x+r, y-k, x+r, y
    );
}

fn build_pdf_pages(pages: &[String]) -> Vec<u8> {
    let font_id = 3 + pages.len() * 2;
    let page_refs = (0..pages.len())
        .map(|index| format!("{} 0 R", 3 + index * 2))
        .collect::<Vec<_>>().join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", page_refs, pages.len()),
    ];
    for (index, stream) in pages.iter().enumerate() {
        let content_id = 4 + index * 2;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.0} {:.0}] /Resources << /Font << /F1 {} 0 R >> >> /Contents {} 0 R >>",
            PAGE_W, PAGE_H, font_id, content_id
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{}endstream", stream.len(), stream));
    }
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned());

    let mut bytes = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, obj) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index+1, obj).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(), xref
        ).as_bytes()
    );
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unifilar_pdf_spreads_seven_links_across_two_pages() {
        let mut plan = Plan::default();
        plan.symbols.push(crate::model::ElectricalSymbol {
            position: Point { x: 0.0, y: 0.0 },
            kind: ElectricalKind::Switch,
        });
        for i in 0..7 {
            let index = plan.symbols.len();
            plan.symbols.push(crate::model::ElectricalSymbol {
                position: Point { x: 1.0, y: i as f32 },
                kind: ElectricalKind::Light,
            });
            plan.connect_switch_to_light(0, index).unwrap();
        }
        let path = std::env::temp_dir().join(format!(
            "easydraft-unifilar-{}.pdf",
            std::process::id()
        ));
        export_unifilar_pdf(&path.to_string_lossy(), &plan).unwrap();
        let pdf = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert!(pdf.windows(b"/Count 2".len()).any(|chunk| chunk == b"/Count 2"));
        assert!(pdf.windows(b"A7".len()).any(|chunk| chunk == b"A7"));
    }

    #[test]
    fn exports_minimal_valid_pdf_structure() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
            thickness_m: 0.2,
        });
        plan.stairs.push(Stair {
            start: Point { x: 1.0, y: 1.0 },
            end: Point { x: 1.0, y: 3.0 },
            width_m: 0.9,
            steps: 10,
        });
        let mut canvas = Canvas::new(&plan);
        canvas.wall(0, &plan.walls[0], &[], true);
        canvas.stair(&plan.stairs[0]);
        let pdf = build_pdf(&canvas.commands);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert!(pdf.windows(b"/MediaBox".len()).any(|w| w == b"/MediaBox"));
        assert!(pdf.windows(b"startxref".len()).any(|w| w == b"startxref"));
    }
}
