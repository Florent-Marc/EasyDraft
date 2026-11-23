//! Export PDF vectoriel A4 paysage, sans dependance supplementaire.
//! Le plan est adapte a la page (il n'est pas imprime a une echelle architecturale fixe).

use std::fmt::Write as _;

use crate::model::{BackgroundPlan, ElectricalKind, Opening, OpeningKind, Plan, Point, Stair, Wall};
use crate::rgie_symbols::{self, Primitive};

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
        if let Some(bg) = plan.background.as_ref() {
            if let Ok((mut width_px, mut height_px)) = image::image_dimensions(&bg.path) {
                if bg.rotation_quarters % 2 == 1 {
                    std::mem::swap(&mut width_px, &mut height_px);
                }
                if width_px > 0 && bg.width_m.is_finite() && bg.width_m > 0.0 {
                    let height_m = bg.width_m * height_px as f32 / width_px as f32;
                    include(bg.origin, 0.0);
                    include(Point {
                        x: bg.origin.x + bg.width_m,
                        y: bg.origin.y + height_m,
                    }, 0.0);
                }
            }
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
        let mut safe = String::new();
        for character in text.chars() {
            match character {
                '\\' | '(' | ')' => {
                    safe.push('\\');
                    safe.push(character);
                }
                ' '..='~' => safe.push(character),
                '\u{a0}'..='\u{ff}' => {
                    let _ = write!(safe, "\\{:03o}", character as u32);
                }
                'Œ' => safe.push_str("\\214"),
                'œ' => safe.push_str("\\234"),
                '’' => safe.push_str("\\222"),
                '–' => safe.push_str("\\226"),
                _ if character.is_whitespace() => safe.push(' '),
                _ => safe.push('?'),
            }
        }
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

    // Reserve au futur export des symboles vectoriels circulaires.
    #[allow(dead_code)]
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
                OpeningKind::Passage => {
                    // Passage libre : seulement les deux tableaux de l'ouverture.
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
        for (index, symbol) in plan.symbols.iter().enumerate() {
            let (x, y) = self.xy(symbol.position);
            let marks = rgie_symbols::device_primitives(
                symbol.kind, plan.symbol_appearance(index)
            );
            draw_pdf_primitives(&mut self.commands, x, y, 0.8, &marks, BLUE);
            let reference = match symbol.kind {
                ElectricalKind::Outlet | ElectricalKind::Light => plan.point_ref(index),
                ElectricalKind::Switch => {
                    let refs = plan.switch_point_refs(index);
                    if refs.is_empty() { None } else { Some(refs.join(",")) }
                }
            };
            if let Some(reference) = reference {
                // PDF en points : l'etiquette reste lisible independamment de l'echelle du plan.
                self.text_pt(x - 13.0, y - 24.0, 8.0, &reference, BLUE);
            }
        }
    }
}

/// Helvetica Type1 emet du texte ASCII dans l'export PDF prototype.
fn ascii_pdf_text(input: &str) -> String {
    input.chars().map(|c| match c {
        'à'|'á'|'â'|'ä'|'À'|'Á'|'Â'|'Ä' => 'a',
        'é'|'è'|'ê'|'ë'|'É'|'È'|'Ê'|'Ë' => 'e',
        'î'|'ï'|'ì'|'í'|'Î'|'Ï' => 'i',
        'ô'|'ö'|'ò'|'ó'|'Ô'|'Ö' => 'o',
        'ù'|'ú'|'û'|'ü'|'Ù'|'Ú'|'Û'|'Ü' => 'u',
        'ç'|'Ç' => 'c',
        _ if c.is_ascii() => c,
        _ => '?',
    }).collect()
}

/// JPEG embarque dans le PDF, avec l'opacite precomposee sur du blanc
/// (pas de transparence PDF ambiguë).
struct PdfBackgroundImage {
    jpeg: Vec<u8>,
    width: u32,
    height: u32,
}

fn pdf_background(bg: &BackgroundPlan) -> Result<PdfBackgroundImage, String> {
    let image = image::open(&bg.path)
        .map_err(|error| format!("Fond de plan introuvable ou illisible : {error}"))?;
    let rgb = image.to_rgb8();
    let mut rgb = match bg.rotation_quarters % 4 {
        1 => image::imageops::rotate90(&rgb),
        2 => image::imageops::rotate180(&rgb),
        3 => image::imageops::rotate270(&rgb),
        _ => rgb,
    };
    let opacity = bg.opacity.clamp(0.0, 1.0);
    for px in rgb.pixels_mut() {
        for channel in &mut px.0 {
            *channel = (*channel as f32 * opacity + 255.0 * (1.0 - opacity)).round() as u8;
        }
    }
    let (width, height) = rgb.dimensions();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
        .encode(rgb.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .map_err(|error| format!("Encodage du fond de plan PDF impossible : {error}"))?;
    Ok(PdfBackgroundImage { jpeg, width, height })
}

#[cfg(test)]
pub(crate) fn export_pdf(path: &str, plan: &Plan, show_dimensions: bool) -> Result<(), String> {
    export_plan_sheets(path, [("Plan 2D", plan)], show_dimensions)
}

/// Exporte une page A4 paysage par feuille, ajustée au dessin de la feuille.
pub fn export_plan_sheets<'a>(
    path: &str,
    sheets: impl IntoIterator<Item = (&'a str, &'a Plan)>,
    show_dimensions: bool,
) -> Result<(), String> {
    let pages = sheets.into_iter().map(|(name, plan)| {
        render_plan_page(name, plan, show_dimensions)
            .map_err(|error| format!("{name} : {error}"))
    }).collect::<Result<Vec<_>, _>>()?;
    let bytes = build_pdf_document(pages.iter().map(|(stream, image)| {
        (stream.as_str(), image.as_ref())
    }));
    std::fs::write(path, bytes).map_err(|err| format!("Export PDF impossible : {err}"))
}

fn render_plan_page(
    name: &str, plan: &Plan, show_dimensions: bool,
) -> Result<(String, Option<PdfBackgroundImage>), String> {
    let image = plan.background.as_ref().map(pdf_background).transpose()?;
    let mut pdf = Canvas::new(plan);
    // Dessiner le fond de plan raster AVANT les symboles et la geometrie
    // vectorielle ; l'opacity est precomposee dans le JPEG embarque.
    if let (Some(bg), Some(raster)) = (plan.background.as_ref(), image.as_ref()) {
        let (x, top_y) = pdf.xy(bg.origin);
        let w = bg.width_m * pdf.scale;
        let h = w * raster.height as f32 / raster.width.max(1) as f32;
        let _ = writeln!(pdf.commands,
            "q {:.3} 0 0 {:.3} {:.3} {:.3} cm /Bg Do Q",
            w, h, x, top_y - h
        );
    }
    let _ = writeln!(pdf.commands, "0 J 0 j"); // extremites nettes et angles miter
    for (index, wall) in plan.walls.iter().enumerate() {
        pdf.wall(index, wall, &plan.openings, show_dimensions);
    }
    for stair in &plan.stairs {
        pdf.stair(stair);
    }
    // Les pieces sont recalculees depuis les murs, comme dans l'editeur 2D.
    // Noms et surfaces proviennent de la sauvegarde et de la geometrie partagees.
    for room in crate::rooms::detect_rooms(&plan.walls) {
        let title = plan.room_name(&room);
        if !title.is_empty() {
            pdf.centered_text(
                Point { x: room.center.x, y: room.center.y - 0.18 },
                11.0,
                &ascii_pdf_text(title),
                BLUE,
            );
        }
        pdf.centered_text(
            room.center,
            9.0,
            &format!("{:.2} m2", room.area_m2),
            BLUE,
        );
        if room.rectangular {
            pdf.centered_text(
                Point { x: room.center.x, y: room.center.y + 0.17 },
                8.0,
                &format!("{:.2} x {:.2} m", room.width_m, room.height_m),
                DIM,
            );
        }
    }
    pdf.symbols(plan);

    // Le texte du titre est ajoute en coordonnees de page, sans transformation du plan.
    let mut title: String = name.chars().take(60).collect();
    if name.chars().count() > 60 {
        title.push_str("...");
    }
    let title_size = if title.chars().count() > 38 { 11.0 } else { 16.0 };
    pdf.text_pt(MARGIN, PAGE_H - 34.0, title_size, &format!("EasyDraft - {title}"), DARK);
    let mut details = format!("Adresse : {}", truncate_pdf(&plan.installation_info.address, 74));
    let revision = plan.installation_info.revision.trim();
    if !revision.is_empty() {
        let _ = write!(details, " | Révision : {}", truncate_pdf(revision, 8));
    }
    pdf.text_pt(MARGIN, PAGE_H - 53.0, 9.0, &details, DIM);
    pdf.text_pt(
        MARGIN, 25.0, 9.0,
        &format!(
            "{} murs  |  {} portes  |  {} fenetres  |  {} passages  |  {} escaliers",
            plan.walls.len(),
            plan.openings.iter().filter(|o| o.kind == OpeningKind::Door).count(),
            plan.openings.iter().filter(|o| o.kind == OpeningKind::Window).count(),
            plan.openings.iter().filter(|o| o.kind == OpeningKind::Passage).count(),
            plan.stairs.len()
        ),
        DIM,
    );
    Ok((pdf.commands, image))
}


/// PDF vectoriel du schema unifilaire de travail, distinct du plan architectural.
/// Plusieurs pages A4 paysage sont generees pour ne pas ecraser les circuits charges.
/// Les valeurs absentes restent explicitement "a renseigner".
/// Entree unifilaire dessinee : soit une commande vers une lampe,
/// soit une prise autonome affectee au circuit.
enum CircuitEntry {
    Light(usize),
    Socket(usize),
}

// Tableau vectoriel de travail. Une feuille precede les details des circuits.
enum BoardRow { Differential(usize), Circuit(usize), MissingDdr }

fn board_pdf_pages(plan: &Plan) -> Vec<String> {
    let mut rows = Vec::new();
    for d in 0..plan.differentials.len() {
        rows.push(BoardRow::Differential(d));
        for i in plan.circuits_for_differential(d) { rows.push(BoardRow::Circuit(i)); }
    }
    let unlinked = plan.circuits_without_differential();
    if !unlinked.is_empty() {
        rows.push(BoardRow::MissingDdr);
        for i in unlinked { rows.push(BoardRow::Circuit(i)); }
    }
    if rows.is_empty() { rows.push(BoardRow::MissingDdr); }
    rows.chunks(12).map(|chunk| {
        let mut out = String::new();
        raw_pdf_text(&mut out, 32.0, 558.0, 17.0,
            "EasyDraft - TABLEAU ELECTRIQUE - SYNOPTIQUE", DARK);
        raw_pdf_text(&mut out, 32.0, 533.0, 10.0,
            &format!("Alimentation : {}", truncate_pdf(&plan.installation_info.supply, 63)), DARK);
        raw_pdf_text(&mut out, 32.0, 516.0, 10.0,
            &format!("Cable d'entree : {}", truncate_pdf(&plan.installation_info.incoming_cable, 63)), DARK);
        raw_pdf_text(&mut out, 32.0, 500.0, 10.0,
            &format!("Tableau : {}", truncate_pdf(&plan.installation_info.panel_name, 55)), DARK);
        pdf_box(&mut out, 35.0, 446.0, 140.0, 30.0, DARK, 1.0);
        raw_pdf_text(&mut out, 50.0, 458.0, 10.0, "Alimentation", DARK);
        let mut y = 431.0;
        for entry in chunk {
            match entry {
                BoardRow::Differential(i) => {
                    let d = &plan.differentials[*i];
                    raw_pdf_line(&mut out, 106.0, y, 210.0, y, DARK, 1.1);
                    pdf_box(&mut out, 212.0, y - 11.0, 135.0, 25.0, DARK, 1.0);
                    raw_pdf_text(&mut out, 219.0, y - 2.0, 10.0,
                        &format!("DDR {}", d.reference), BLUE);
                    raw_pdf_text(&mut out, 359.0, y - 2.0, 10.0,
                        &truncate_pdf(&d.description(), 69), DARK);
                }
                BoardRow::MissingDdr => {
                    raw_pdf_text(&mut out, 209.0, y - 2.0, 11.0,
                        "DDR NON ATTRIBUE - branche non reliee", (0.7, 0.35, 0.1));
                }
                BoardRow::Circuit(i) => {
                    if let Some(c) = plan.circuit(*i) {
                        let assigned = plan.differential_for_circuit(*i).is_some();
                        let col = if assigned { DARK } else { (0.7, 0.35, 0.1) };
                        raw_pdf_text(&mut out, 211.0, y - 2.0, 9.0,
                            &format!("Via {}", plan.differential_for_circuit(*i)
                                .map(|d| d.reference.as_str()).unwrap_or("?")), col);
                        if assigned {
                            raw_pdf_line(&mut out, 277.0, y, 319.0, y, DARK, 1.1);
                        }
                        draw_pdf_primitives(
                            &mut out, 336.0, y, 0.72,
                            &rgie_symbols::horizontal_protection_primitives(
                                rgie_symbols::protection_from_text(&c.protection_kind)
                            ), col
                        );
                        raw_pdf_line(&mut out, 355.0, y, 398.0, y, col, 1.1);
                        raw_pdf_text(&mut out, 411.0, y - 2.0, 10.0,
                            &format!("Circuit {} / {}", c.letter,
                                truncate_pdf(&c.protection_label(), 25)), DARK);
                        raw_pdf_text(&mut out, 635.0, y - 2.0, 9.0,
                            &truncate_pdf(&c.cable_label(), 27), DIM);
                    }
                }
            }
            y -= 29.0;
        }
        raw_pdf_text(&mut out, 32.0, 45.0, 9.0,
            "Topologie declaree, non verifiee sur place. Pas un schema de cablage.", DIM);
        out
    }).collect()
}

pub fn export_unifilar_pdf(path: &str, plan: &Plan) -> Result<(), String> {
    // Une ou plusieurs pages par circuit, avec au plus six appareils par page.
    // Pas de faux symbole de conformite ni de schema de raccordement.
    let mut page_specs = Vec::new();
    for circuit_index in 0..plan.circuit_count() {
        let count = plan.active_lighting_connections_for_circuit(circuit_index).len()
            + plan.outlets_on_circuit(circuit_index).len();
        for chunk_index in 0..count.max(1).div_ceil(6) {
            let start = chunk_index * 6;
            page_specs.push((circuit_index, start, (start + 6).min(count)));
        }
    }
    let mut pages = board_pdf_pages(plan);
    let overview_count = pages.len();
    let pages_count = overview_count + page_specs.len();
    let color = DARK;
    for (index, page) in pages.iter_mut().enumerate() {
        raw_pdf_text(page, 720.0, 560.0, 9.0,
            &format!("Page {}/{}", index + 1, pages_count), DARK);
    }

    for (page_index, (circuit_index, start, end)) in page_specs.into_iter().enumerate() {
        let circuit = plan.circuit(circuit_index)
            .ok_or("Index de circuit invalide pendant l'export")?;
        let links = plan.active_lighting_connections_for_circuit(circuit_index);
        let outlets = plan.outlets_on_circuit(circuit_index);
        let mut items: Vec<CircuitEntry> = (0..links.len()).map(CircuitEntry::Light).collect();
        items.extend(outlets.iter().copied().map(CircuitEntry::Socket));
        items.sort_by_key(|item| match item {
            CircuitEntry::Light(index) => links[*index].light_index,
            CircuitEntry::Socket(index) => *index,
        });

        let mut pdf = String::new();
        raw_pdf_text(
            &mut pdf, 35.0, 563.0, 17.0,
            "EasyDraft - SCHEMA UNIFILAIRE (PROTOTYPE)", color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 546.0, 10.0,
            "Dossier de travail / NON CERTIFIE / controle RGIE requis le cas echeant",
            (0.68, 0.25, 0.10),
        );
        raw_pdf_text(
            &mut pdf, 35.0, 529.0, 10.0,
            &format!("Adresse : {}", truncate_pdf(&plan.installation_info.address, 82)),
            color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 512.0, 11.0,
            &format!(
                "Circuit {} | Protection : {}",
                pdf_safe_label(&circuit.letter),
                if circuit.breaker_amp == 0 { "a renseigner".to_owned() }
                else { circuit.protection_label() }
            ),
            color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 496.0, 10.0,
            &format!("Canalisation : {}", circuit.cable_label()),
            color,
        );
        raw_pdf_text(
            &mut pdf, 35.0, 480.0, 9.0,
            &format!(
                "Pose : {}  |  DDR amont : {}",
                truncate_pdf(&circuit.installation_method, 28),
                truncate_pdf(&plan.differential_description_for_circuit(circuit_index), 32),
            ),
            color,
        );
        raw_pdf_text(
            &mut pdf, 715.0, 563.0, 9.0,
            &format!("Page {}/{}", page_index + overview_count + 1, pages_count), color,
        );
        let missing_count = plan.circuit_missing_fields(circuit_index).len();
        if missing_count > 0 {
            raw_pdf_text(
                &mut pdf, 35.0, 462.0, 9.0,
                &format!("ATTENTION : {missing_count} donnee(s) de circuit non renseignee(s)"),
                (0.67, 0.31, 0.08),
            );
        }

        // Tableau -> DDR lie -> protection du circuit. Sans DDR, PAS de
        // liaison vers le tableau (on montre le manque explicitement).
        pdf_box(&mut pdf, 40.0, 390.0, 112.0, 42.0, color, 1.0);
        raw_pdf_text(&mut pdf, 43.0, 405.0, 10.0,
            &truncate_pdf(&plan.installation_info.panel_name, 16), color);
        if let Some(device) = plan.differential_for_circuit(circuit_index) {
            raw_pdf_line(&mut pdf, 152.0, 411.0, 177.0, 411.0, color, 1.3);
            pdf_box(&mut pdf, 177.0, 395.0, 73.0, 32.0, color, 1.0);
            raw_pdf_text(&mut pdf, 182.0, 407.0, 10.0,
                &format!("DDR {}", device.reference), color);
        } else {
            pdf_box(&mut pdf, 177.0, 395.0, 73.0, 32.0, (0.7, 0.35, 0.1), 1.0);
            raw_pdf_text(&mut pdf, 182.0, 407.0, 10.0,
                "DDR ?", (0.7, 0.35, 0.1));
        }
        raw_pdf_line(&mut pdf, 250.0, 411.0, 263.0, 411.0, color, 1.3);
        draw_pdf_primitives(
            &mut pdf, 283.0, 411.0, 0.75,
            &rgie_symbols::horizontal_protection_primitives(
                rgie_symbols::protection_from_text(&circuit.protection_kind)
            ),
            color,
        );
        raw_pdf_line(&mut pdf, 302.0, 411.0, 310.0, 411.0, color, 1.3);
        raw_pdf_text(&mut pdf, 173.0, 377.0, 9.0,
            "DDR attribue / protection divisionnaire declares (a verifier)", DIM);

        let first_y = 329.0;
        let last_y = if end > start {
            first_y - (end - start - 1) as f32 * 46.0
        } else { first_y };
        raw_pdf_line(&mut pdf, 310.0, 411.0, 310.0, last_y, color, 1.6);

        for (index, entry) in items[start..end].iter().enumerate() {
            let y = first_y - index as f32 * 46.0;
            raw_pdf_line(&mut pdf, 310.0, y, 431.0, y, color, 1.6);
            match entry {
                CircuitEntry::Light(link_index) => {
                    let link = links[*link_index];
                    let appearance = plan.symbol_appearance(link.switch_index);
                    let radius = rgie_symbols::schematic_switch_radius(appearance) * 0.83;
                    raw_pdf_line(&mut pdf, 431.0, y, 451.0 - radius, y, color, 1.6);
                    draw_pdf_primitives(
                        &mut pdf, 451.0, y, 0.83,
                        &rgie_symbols::schematic_device_primitives(
                            ElectricalKind::Switch,
                            appearance,
                        ),
                        color,
                    );
                    raw_pdf_line(&mut pdf, 451.0 + radius, y, 670.0, y, color, 1.6);
                    draw_pdf_primitives(
                        &mut pdf, 670.0, y, 1.02,
                        &rgie_symbols::schematic_device_primitives(
                            ElectricalKind::Light,
                            plan.symbol_appearance(link.light_index),
                        ),
                        color,
                    );
                    let i_num = plan.symbol_number(link.switch_index).unwrap_or(link.switch_index + 1);
                    let l_num = plan.symbol_number(link.light_index).unwrap_or(link.light_index + 1);
                    raw_pdf_text(&mut pdf, 424.0, y - 21.0, 10.0, &format!("I{i_num}"), color);
                    raw_pdf_text(&mut pdf, 664.0, y - 21.0, 10.0, &format!("L{l_num}"), color);
                    raw_pdf_text(&mut pdf, 718.0, y, 11.0,
                        &plan.point_ref(link.light_index).unwrap_or_else(|| "?".into()),
                        BLUE);
                }
                CircuitEntry::Socket(symbol_index) => {
                    raw_pdf_line(&mut pdf, 431.0, y, 670.0 - 19.0 * 0.85, y, color, 1.6);
                    draw_pdf_primitives(
                        &mut pdf, 670.0, y, 0.85,
                        &rgie_symbols::schematic_device_primitives(
                            ElectricalKind::Outlet,
                            plan.symbol_appearance(*symbol_index),
                        ),
                        color,
                    );
                    let p_num = plan.symbol_number(*symbol_index).unwrap_or(*symbol_index + 1);
                    raw_pdf_text(&mut pdf, 664.0, y - 21.0, 10.0, &format!("P{p_num}"), color);
                    raw_pdf_text(&mut pdf, 718.0, y, 11.0,
                        &plan.point_ref(*symbol_index).unwrap_or_else(|| "?".into()),
                        BLUE);
                }
            }
        }
        if items.is_empty() {
            raw_pdf_text(
                &mut pdf, 335.0, 326.0, 11.0,
                "Aucun appareil attribue au circuit", DIM,
            );
        }
        let info = &plan.installation_info;
        raw_pdf_text(&mut pdf, 35.0, 67.0, 9.0,
            &format!("Tension : {} | Courant : {} | Revision : {} | Date : {}",
                truncate_pdf(&info.voltage, 12),
                truncate_pdf(&info.current_kind, 16),
                truncate_pdf(&info.revision, 9),
                truncate_pdf(&info.drawing_date, 14)),
            color);
        raw_pdf_text(&mut pdf, 35.0, 51.0, 9.0,
            &format!("Responsable : {} ({})",
                truncate_pdf(&info.executor_name, 43),
                truncate_pdf(&info.executor_role, 24)),
            color);
        raw_pdf_text(&mut pdf, 35.0, 35.0, 9.0,
            &format!("Non affectes : {} lampe(s), {} interrupteur(s), {} prise(s)",
                plan.unlinked_lights(), plan.unlinked_switches(), plan.unassigned_outlets()),
            color);
        raw_pdf_text(&mut pdf, 35.0, 19.0, 9.0,
            "Symboles de base du RGIE 2.23. Circuit fonctionnel a verifier avant tout usage officiel.",
            DIM);
        pages.push(pdf);
    }

    std::fs::write(path, build_pdf_pages(&pages))
        .map_err(|e| format!("Export du schema unifilaire impossible : {e}"))
}

fn truncate_pdf(value: &str, limit: usize) -> String {
    let text = pdf_safe_label(value);
    let mut chars = text.chars();
    let part = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() { format!("{part}...") } else { part }
}

fn pdf_safe_label(value: &str) -> &str {
    if value.trim().is_empty() { "a renseigner" } else { value.trim() }
}

/// Traduit le catalogue graphique commun en traits PDF vectoriels.
/// Le code PDF et l'ecran utilisent exactement les memes primitives locales.
/// Coordonnees des symboles : Y positif vers le bas, converti ici en Y PDF.
fn draw_pdf_primitives(
    out: &mut String, cx: f32, cy: f32, size: f32,
    shapes: &[Primitive], color: Rgb,
) {
    let at = |p: (f32, f32)| (cx + p.0 * size, cy - p.1 * size);
    for primitive in shapes {
        match *primitive {
            Primitive::Line(a, b) => {
                let (x1, y1) = at(a);
                let (x2, y2) = at(b);
                raw_pdf_line(out, x1, y1, x2, y2, color, 1.35);
            }
            Primitive::Arc(center, radius, start, end) => {
                let points = rgie_symbols::arc_points(center, radius, start, end);
                for pair in points.windows(2) {
                    let (x1, y1) = at(pair[0]);
                    let (x2, y2) = at(pair[1]);
                    raw_pdf_line(out, x1, y1, x2, y2, color, 1.35);
                }
            }
            Primitive::Circle(center, radius) => {
                let (x, y) = at(center);
                raw_pdf_circle(out, x, y, radius * size, color);
            }
            Primitive::Dot(center, radius) => {
                let (x, y) = at(center);
                // Un disque via quatre courbes de Bezier remplies.
                let r = radius * size;
                let k = r * 0.55228475;
                let _ = writeln!(
                    out,
                    "{:.3} {:.3} {:.3} rg {:.3} {:.3} m \
                     {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c \
                     {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c \
                     {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c \
                     {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c f",
                    color.0, color.1, color.2,
                    x+r, y,
                    x+r, y+k, x+k, y+r, x, y+r,
                    x-k, y+r, x-r, y+k, x-r, y,
                    x-r, y-k, x-k, y-r, x, y-r,
                    x+k, y-r, x+r, y-k, x+r, y,
                );
            }
        }
    }
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
    let ascii = ascii_pdf_text(text);
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
    build_pdf_document(pages.iter().map(|stream| (stream.as_str(), None)))
}

fn build_pdf_document<'a>(
    pages: impl IntoIterator<Item = (&'a str, Option<&'a PdfBackgroundImage>)>,
) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        Vec::new(), // Arbre des pages, renseigné une fois les pages numérotées.
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
    ];
    let mut page_refs = Vec::new();
    for (stream, image) in pages {
        let page_id = objects.len() + 1;
        let content_id = page_id + 1;
        let resources = if image.is_some() {
            format!("/Font << /F1 3 0 R >> /XObject << /Bg {} 0 R >>", page_id + 2)
        } else {
            "/Font << /F1 3 0 R >>".into()
        };
        page_refs.push(format!("{page_id} 0 R"));
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.0} {:.0}] /Resources << {} >> /Contents {} 0 R >>",
            PAGE_W, PAGE_H, resources, content_id
        ).into_bytes());
        objects.push(format!("<< /Length {} >>\nstream\n{}endstream", stream.len(), stream).into_bytes());
        if let Some(background) = image {
            let mut object = format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                background.width, background.height, background.jpeg.len(),
            ).into_bytes();
            object.extend_from_slice(&background.jpeg);
            object.extend_from_slice(b"\nendstream");
            objects.push(object);
        }
    }
    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>", page_refs.join(" "), page_refs.len(),
    ).into_bytes();

    let mut bytes = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, obj) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(obj);
        bytes.extend_from_slice(b"\nendobj\n");
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
        assert!(pdf.windows(b"/Count 3".len()).any(|chunk| chunk == b"/Count 3"));
        assert!(pdf.windows(b"A7".len()).any(|chunk| chunk == b"A7"));
    }

    #[test]
    fn unifilar_pdf_includes_assigned_outlets_and_dossier_metadata() {
        let mut plan = Plan::default();
        plan.installation_info.address = "Maison test Belgique".into();
        plan.installation_info.voltage = "230 V".into();
        plan.symbols.push(crate::model::ElectricalSymbol {
            position: Point { x: 2.0, y: 1.0 },
            kind: ElectricalKind::Outlet,
        });
        plan.assign_outlet_to_circuit(0, 0).unwrap();
        let target = std::env::temp_dir().join(format!(
            "easydraft-prise-rgie-{}.pdf", std::process::id()
        ));
        export_unifilar_pdf(&target.to_string_lossy(), &plan).unwrap();
        let bytes = std::fs::read(&target).unwrap();
        let _ = std::fs::remove_file(target);
        assert!(bytes.windows(b"Maison test Belgique".len()).any(|w| w == b"Maison test Belgique"));
        assert!(bytes.windows(b"230 V".len()).any(|w| w == b"230 V"));
        assert!(bytes.windows(b"(A1)".len()).any(|w| w == b"(A1)"));
        assert!(bytes.starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn unifilar_pdf_separates_circuit_a_and_b() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        let other = plan.add_lighting_circuit().unwrap();
        let sw = plan.symbols.len();
        plan.symbols.push(crate::model::ElectricalSymbol {
            position: Point { x: 5.0, y: 0.0 },
            kind: ElectricalKind::Switch,
        });
        let lamp = plan.symbols.len();
        plan.symbols.push(crate::model::ElectricalSymbol {
            position: Point { x: 6.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        plan.connect_switch_to_light_in_circuit(sw, lamp, other).unwrap();
        assert_eq!(plan.lighting_point_ref(lamp).as_deref(), Some("B1"));
        let filename = std::env::temp_dir().join(format!(
            "easydraft-multicircuit-{}.pdf",
            std::process::id()
        ));
        export_unifilar_pdf(&filename.to_string_lossy(), &plan).unwrap();
        let pdf = std::fs::read(&filename).unwrap();
        let _ = std::fs::remove_file(filename);
        assert!(pdf.windows(b"/Count 3".len()).any(|x| x == b"/Count 3"));
        assert!(pdf.windows(b"Circuit A".len()).any(|x| x == b"Circuit A"));
        assert!(pdf.windows(b"Circuit B".len()).any(|x| x == b"Circuit B"));
    }

    #[test]
    fn pdf_symbol_catalog_reuses_rgie_shapes_and_selected_variants() {
        let mut generic = String::new();
        draw_pdf_primitives(
            &mut generic, 100.0, 100.0, 1.0,
            &rgie_symbols::device_primitives(
                ElectricalKind::Outlet, crate::model::ElectricalAppearance::General
            ),
            DARK,
        );
        let mut with_earth = String::new();
        draw_pdf_primitives(
            &mut with_earth, 100.0, 100.0, 1.0,
            &rgie_symbols::device_primitives(
                ElectricalKind::Outlet, crate::model::ElectricalAppearance::OutletEarth
            ),
            DARK,
        );
        assert_ne!(generic, with_earth);
        assert!(generic.contains(" RG "));
        assert!(with_earth.len() > generic.len());
    }

    #[test]
    fn board_sample_export_contains_overview_and_three_circuit_pages() {
        let plan: Plan = serde_json::from_str(
            include_str!("../examples/tableau-deux-ddr-trois-circuits.json")
        ).unwrap();
        let path = std::env::temp_dir().join(format!(
            "easydraft-ddr-tableau-{}.pdf", std::process::id()
        ));
        export_unifilar_pdf(&path.to_string_lossy(), &plan).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.windows(b"/Count 4".len()).any(|a| a == b"/Count 4"));
        assert!(bytes.windows(b"DDR D1".len()).any(|a| a == b"DDR D1"));
        assert!(bytes.windows(b"DDR D2".len()).any(|a| a == b"DDR D2"));
        assert!(bytes.windows(b"Circuit C".len()).any(|a| a == b"Circuit C"));
    }

    #[test]
    fn board_overview_pdf_differentiates_connected_and_unassigned_departures() {
        let mut plan = Plan::default();
        plan.add_lighting_circuit().unwrap();
        let d = plan.add_differential().unwrap();
        plan.set_circuit_differential(0, Some(d)).unwrap();
        let overview = board_pdf_pages(&plan).join("\n");
        assert!(overview.contains("DDR D1"));
        assert!(overview.contains("Via D1"));
        assert!(overview.contains("DDR NON ATTRIBUE"));
        assert!(overview.contains("Via ?"));
        plan.set_circuit_differential(1, Some(d)).unwrap();
        let connected = board_pdf_pages(&plan).join("\n");
        assert!(!connected.contains("DDR NON ATTRIBUE"));
    }

    #[test]
    fn export_plan_embeds_background_image_with_vector_walls() {
        let filename = format!("easydraft-bg-test-{}", std::process::id());
        let picture = std::env::temp_dir().join(format!("{filename}.png"));
        let output = std::env::temp_dir().join(format!("{filename}.pdf"));
        image::RgbImage::from_pixel(80, 50, image::Rgb([80, 140, 190]))
            .save(&picture).unwrap();
        let mut plan = Plan::default();
        plan.background = Some(BackgroundPlan {
            path: picture.to_string_lossy().into_owned(),
            width_m: 10.0,
            origin: Point { x: -5.0, y: -3.125 },
            opacity: 0.4,
            rotation_quarters: 1,
        });
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 3.0, y: 0.0 },
            thickness_m: 0.15,
        });
        let other = Plan::default();
        export_plan_sheets(output.to_str().unwrap(), [("Cave", &plan), ("Grenier", &other)], true)
            .unwrap();
        let pdf = std::fs::read(&output).unwrap();
        assert_eq!(pdf.windows(15).filter(|piece| *piece == b"/Subtype /Image").count(), 1);
        assert_eq!(pdf.windows(6).filter(|piece| *piece == b"/Bg Do").count(), 1);
        assert!(pdf.windows(b"/Count 2".len()).any(|piece| piece == b"/Count 2"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        let _ = std::fs::remove_file(&output);
        let _ = std::fs::remove_file(&picture);
    }

    #[test]
    fn plan_page_omits_blank_revision_and_places_counts_at_left_margin() {
        let mut plan = Plan::default();
        plan.installation_info.revision = "  ".into();
        let (page, _) = render_plan_page("Cave", &plan, true).unwrap();
        assert!(!page.contains(r"R\351vision"));
        assert!(!page.contains("Ajuste a la page"));
        assert!(page.contains(&format!("{MARGIN:.3} 25.000 Td (0 murs")));

        plan.installation_info.revision = "B".into();
        let (page, _) = render_plan_page("Cave", &plan, true).unwrap();
        assert!(page.contains(r"R\351vision : B"));
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
        let pdf = build_pdf_pages(&[canvas.commands]);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        assert!(pdf.windows(b"/MediaBox".len()).any(|w| w == b"/MediaBox"));
        assert!(pdf.windows(b"startxref".len()).any(|w| w == b"startxref"));
    }
}
