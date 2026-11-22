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

/// Variante graphique RGIE choisie explicitement. Le symbole general ne
/// suppose ni mise a la terre, ni obturateurs, ni fonction de commande complexe.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ElectricalAppearance {
    General,
    SwitchBipolar,
    SwitchTwoWay,
    PushButton,
    OutletEarth,
    OutletChild,
    OutletEarthChild,
    WallLight,
}

impl ElectricalAppearance {
    pub fn valid_for(self, kind: ElectricalKind) -> bool {
        match self {
            Self::General => true,
            Self::SwitchBipolar | Self::SwitchTwoWay | Self::PushButton =>
                kind == ElectricalKind::Switch,
            Self::OutletEarth | Self::OutletChild | Self::OutletEarthChild =>
                kind == ElectricalKind::Outlet,
            Self::WallLight => kind == ElectricalKind::Light,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "Symbole general",
            Self::SwitchBipolar => "Interrupteur bipolaire",
            Self::SwitchTwoWay => "Interrupteur va-et-vient",
            Self::PushButton => "Bouton-poussoir",
            Self::OutletEarth => "Prise avec contact de terre",
            Self::OutletChild => "Prise avec protection enfant",
            Self::OutletEarthChild => "Prise avec terre et protection enfant",
            Self::WallLight => "Point d'attente d'eclairage en applique",
        }
    }

    pub fn choices(kind: ElectricalKind) -> &'static [Self] {
        match kind {
            ElectricalKind::Switch => &[
                Self::General, Self::SwitchBipolar, Self::SwitchTwoWay, Self::PushButton
            ],
            ElectricalKind::Outlet => &[
                Self::General, Self::OutletEarth, Self::OutletChild, Self::OutletEarthChild
            ],
            ElectricalKind::Light => &[Self::General, Self::WallLight],
        }
    }
}

/// Une entree par symbole personnalise. Les anciens JSON restent compatibles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolAppearanceRecord {
    pub symbol_index: usize,
    pub appearance: ElectricalAppearance,
}

/// Relation fonctionnelle entre un interrupteur et un luminaire.
 /// Les indices referencent la liste Plan::symbols ; ils restent coherents tant qu'on
 /// n'offre pas de suppression ou reordonnancement individuel des symboles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightingConnection {
    pub switch_index: usize,
    pub light_index: usize,
    /// 0 = circuit historique. L'absence du champ reste compatible
    /// avec les projets enregistres avant la gestion multi-circuits.
    #[serde(default)]
    pub circuit_index: usize,
}

/// Association explicite d'une prise placee sur le plan a un circuit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutletAssignment {
    pub symbol_index: usize,
    pub circuit_index: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct InstallationInfo {
    pub address: String,
    pub executor_name: String,
    pub executor_role: String,
    pub executor_vat: String,
    /// Valeurs libres afin de ne pas supposer 230 V, 400 V ou mono/triphase.
    pub voltage: String,
    pub current_kind: String,
    pub supply: String,
    /// Caracteristiques du cable d'entree au tableau principal.
    pub incoming_cable: String,
    pub panel_name: String,
    pub revision: String,
    pub drawing_date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewLevel {
    Missing,
    ToVerify,
}

#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub level: ReviewLevel,
    pub scope: String,
    pub message: String,
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
    pub protection_kind: String,
    pub breaker_amp: u16,
    pub breaker_poles: u8,
    pub breaker_curve: String,
    /// Ancienne description libre conservee pour la compatibilite JSON.
    pub upstream_rdc: String,
    /// Lien structure vers Plan::differentials. Ne pas deduire d'un vieux texte.
    #[serde(default)]
    pub upstream_differential: Option<usize>,
    pub cable_type: String,
    pub conductor_count: u8,
    pub section_mm2: f32,
    pub installation_method: String,
}

impl Default for LightingCircuit {
    fn default() -> Self {
        Self {
            letter: "A".into(),
            protection_kind: String::new(),
            breaker_amp: 0,
            breaker_poles: 0,
            breaker_curve: String::new(),
            upstream_rdc: String::new(),
            upstream_differential: None,
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
        if self.protection_kind.trim().is_empty() { missing.push("type de protection"); }
        if self.breaker_amp == 0 { missing.push("calibre protection"); }
        if self.breaker_poles == 0 { missing.push("nombre de poles"); }
        if self.breaker_curve.trim().is_empty() { missing.push("courbe/type protection"); }
        if self.cable_type.trim().is_empty() { missing.push("type de cable"); }
        if self.conductor_count == 0 { missing.push("nombre de conducteurs"); }
        if !self.section_mm2.is_finite() || self.section_mm2 <= 0.0 { missing.push("section du cable"); }
        if self.installation_method.trim().is_empty() { missing.push("mode de pose"); }
        missing
    }

    pub fn protection_label(&self) -> String {
        let kind = self.protection_kind.trim();
        let mut parts = vec![if kind.is_empty() {
            "Protection".to_owned()
        } else {
            kind.to_owned()
        }];
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

/// Caracteristiques RELEVEES d'une protection differentielle dans le tableau.
/// Les champs a zero / vides sont explicitement inconnus.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DifferentialDevice {
    pub reference: String,
    pub kind: String,
    pub rated_current_amp: u16,
    pub sensitivity_ma: u16,
    pub poles: u8,
    pub notes: String,
}

impl DifferentialDevice {
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.reference.trim().is_empty() { missing.push("repere"); }
        if self.kind.trim().is_empty() { missing.push("type du DDR"); }
        if self.rated_current_amp == 0 { missing.push("courant nominal"); }
        if self.sensitivity_ma == 0 { missing.push("sensibilite differentielle"); }
        if self.poles == 0 { missing.push("nombre de poles"); }
        missing
    }

    pub fn description(&self) -> String {
        format!(
            "{} - {} - {} A / {} mA - {}P",
            self.reference,
            if self.kind.trim().is_empty() { "type ?" } else { self.kind.trim() },
            if self.rated_current_amp == 0 { "?".to_string() } else { self.rated_current_amp.to_string() },
            if self.sensitivity_ma == 0 { "?".to_string() } else { self.sensitivity_ma.to_string() },
            if self.poles == 0 { "?".to_string() } else { self.poles.to_string() },
        )
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
    #[serde(default)]
    pub symbol_appearances: Vec<SymbolAppearanceRecord>,
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
    /// Circuits d'eclairage additionnels ; l'ancien circuit reste a l'indice zero.
    #[serde(default)]
    pub additional_lighting_circuits: Vec<LightingCircuit>,
    /// Protections differentielles du tableau ; elles peuvent alimenter plusieurs circuits.
    #[serde(default)]
    pub differentials: Vec<DifferentialDevice>,
    #[serde(default)]
    pub outlet_assignments: Vec<OutletAssignment>,
    #[serde(default)]
    pub installation_info: InstallationInfo,
    /// Les noms appartiennent a un point interieur, pas a un index de murs fragile.
    #[serde(default)]
    pub room_labels: Vec<RoomLabel>,
}

impl Plan {
    pub fn symbol_appearance(&self, symbol_index: usize) -> ElectricalAppearance {
        let Some(symbol) = self.symbols.get(symbol_index) else {
            return ElectricalAppearance::General;
        };
        self.symbol_appearances.iter()
            .find(|record| record.symbol_index == symbol_index
                && record.appearance.valid_for(symbol.kind))
            .map(|record| record.appearance)
            .unwrap_or(ElectricalAppearance::General)
    }

    pub fn set_symbol_appearance(
        &mut self, symbol_index: usize, appearance: ElectricalAppearance
    ) -> Result<(), &'static str> {
        let kind = self.symbols.get(symbol_index)
            .ok_or("Symbole electrique introuvable")?.kind;
        if !appearance.valid_for(kind) {
            return Err("Variante incompatible avec ce type d'appareil");
        }
        self.symbol_appearances.retain(|record| record.symbol_index != symbol_index);
        if appearance != ElectricalAppearance::General {
            self.symbol_appearances.push(SymbolAppearanceRecord {
                symbol_index, appearance,
            });
        }
        Ok(())
    }

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

    /// Deplace une ouverture le long de son mur, en verifiant ses limites.
    /// Aucun decalage geometrique des autres ouvertures n'est autorise.
    pub fn update_opening(
        &mut self,
        index: usize,
        center_m: f32,
        width_m: f32,
    ) -> Result<(), &'static str> {
        let opening = self.openings.get(index).ok_or("Ouverture introuvable")?;
        let wall = self.walls.get(opening.wall_index).ok_or("Mur de l'ouverture introuvable")?;
        let length = wall.length_m();
        const MARGIN: f32 = 0.05;
        if !center_m.is_finite() || !width_m.is_finite() || width_m <= 0.0 {
            return Err("Position ou largeur invalide");
        }
        let start = center_m - width_m / 2.0;
        let end = center_m + width_m / 2.0;
        if start < MARGIN - 0.0001 || end > length - MARGIN + 0.0001 {
            return Err("L'ouverture doit rester a l'interieur du mur (marge 5 cm)");
        }
        if self.openings.iter().enumerate().any(|(other_index, other)| {
            other_index != index
                && other.wall_index == opening.wall_index
                && start < other.end_m() + MARGIN - 0.0001
                && end > other.start_m() - MARGIN + 0.0001
        }) {
            return Err("L'ouverture chevaucherait une autre porte ou fenetre");
        }
        self.openings[index].center_m = center_m;
        self.openings[index].width_m = width_m;
        Ok(())
    }

    /// Deplacement d'une ouverture sans changement de largeur.
    pub fn move_opening(&mut self, index: usize, center_m: f32) -> Result<(), &'static str> {
        let width = self.openings.get(index).ok_or("Ouverture introuvable")?.width_m;
        self.update_opening(index, center_m, width)
    }

    pub fn remove_opening(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.openings.len() { return Err("Ouverture introuvable"); }
        self.openings.remove(index);
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
        self.symbol_appearances.retain(|record| record.symbol_index != index);
        for record in &mut self.symbol_appearances {
            if record.symbol_index > index { record.symbol_index -= 1; }
        }
        self.outlet_assignments.retain(|item| item.symbol_index != index);
        for assignment in &mut self.outlet_assignments {
            if assignment.symbol_index > index { assignment.symbol_index -= 1; }
        }
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

    pub fn circuit_count(&self) -> usize {
        1 + self.additional_lighting_circuits.len()
    }

    pub fn circuit(&self, index: usize) -> Option<&LightingCircuit> {
        if index == 0 { Some(&self.lighting_circuit) }
        else { self.additional_lighting_circuits.get(index - 1) }
    }

    pub fn add_differential(&mut self) -> Result<usize, &'static str> {
        if self.differentials.len() >= 24 {
            return Err("Maximum 24 protections differentielles dans ce prototype");
        }
        let reference = (1..=99).map(|n| format!("D{n}"))
            .find(|candidate| !self.differentials.iter().any(|d| {
                d.reference.eq_ignore_ascii_case(candidate)
            }))
            .ok_or("Aucun repere de differentiel libre")?;
        let index = self.differentials.len();
        self.differentials.push(DifferentialDevice {
            reference,
            ..DifferentialDevice::default()
        });
        Ok(index)
    }

    pub fn update_differential(
        &mut self, index: usize, updated: DifferentialDevice
    ) -> Result<(), &'static str> {
        if index >= self.differentials.len() { return Err("Differentiel introuvable"); }
        let reference = updated.reference.trim().to_ascii_uppercase();
        if reference.is_empty() || reference.len() > 12
            || !reference.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Repere differentiel invalide (1 a 12 lettres, chiffres, tiret)");
        }
        if self.differentials.iter().enumerate().any(|(other, d)| {
            other != index && d.reference.eq_ignore_ascii_case(reference)
        }) {
            return Err("Repere de differentiel deja utilise");
        }
        let mut device = updated;
        device.reference = reference;
        self.differentials[index] = device;
        Ok(())
    }

    pub fn set_circuit_differential(
        &mut self, circuit_index: usize, upstream_index: Option<usize>
    ) -> Result<(), &'static str> {
        if upstream_index.is_some_and(|i| i >= self.differentials.len()) {
            return Err("Le differentiel selectionne n'existe pas");
        }
        if circuit_index == 0 {
            self.lighting_circuit.upstream_differential = upstream_index;
        } else if let Some(circuit) = self.additional_lighting_circuits.get_mut(circuit_index - 1) {
            circuit.upstream_differential = upstream_index;
        } else {
            return Err("Circuit introuvable");
        }
        Ok(())
    }

    pub fn differential_for_circuit(
        &self, circuit_index: usize
    ) -> Option<&DifferentialDevice> {
        let index = self.circuit(circuit_index)?.upstream_differential?;
        self.differentials.get(index)
    }

    pub fn differential_description_for_circuit(&self, circuit_index: usize) -> String {
        if let Some(device) = self.differential_for_circuit(circuit_index) {
            device.description()
        } else {
            "DDR non attribue".into()
        }
    }

    /// Le champ descriptif ancien ne vaut jamais un raccordement de tableau.
    pub fn circuit_missing_fields(&self, circuit_index: usize) -> Vec<&'static str> {
        let Some(circuit) = self.circuit(circuit_index) else { return vec!["circuit introuvable"]; };
        let mut missing = circuit.missing_fields();
        if self.differential_for_circuit(circuit_index).is_none() {
            missing.push("differentiel en amont non attribue");
        }
        missing
    }

    pub fn circuits_for_differential(&self, differential_index: usize) -> Vec<usize> {
        (0..self.circuit_count()).filter(|&index| {
            self.circuit(index).is_some_and(|c| {
                c.upstream_differential == Some(differential_index)
            })
        }).collect()
    }

    pub fn circuits_without_differential(&self) -> Vec<usize> {
        (0..self.circuit_count()).filter(|&index| {
            self.differential_for_circuit(index).is_none()
        }).collect()
    }

    /// Un differentiel utilise ne peut etre supprime (indices reindexes sinon).
    pub fn remove_differential(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.differentials.len() { return Err("Differentiel introuvable"); }
        if self.circuits_for_differential(index).len() > 0 {
            return Err("Deconnecte d'abord les circuits de ce differentiel");
        }
        self.differentials.remove(index);
        if let Some(i) = self.lighting_circuit.upstream_differential {
            if i > index { self.lighting_circuit.upstream_differential = Some(i - 1); }
        }
        for circuit in &mut self.additional_lighting_circuits {
            if let Some(i) = circuit.upstream_differential {
                if i > index { circuit.upstream_differential = Some(i - 1); }
            }
        }
        Ok(())
    }

    pub fn add_lighting_circuit(&mut self) -> Result<usize, &'static str> {
        if self.circuit_count() >= 16 {
            return Err("Maximum de 16 circuits dans cette version");
        }
        // Genere un repere unique, sans modifier les reperes existants.
        let letter = (1..=26).map(|index| ((b'A' + index) as char).to_string())
            .find(|name| (0..self.circuit_count()).all(|i| {
                self.circuit(i).is_some_and(|c| !c.letter.eq_ignore_ascii_case(name))
            }))
            .ok_or("Aucun repere libre")?;
        let new_index = self.circuit_count();
        self.additional_lighting_circuits.push(LightingCircuit {
            letter,
            ..LightingCircuit::default()
        });
        Ok(new_index)
    }

    pub fn update_lighting_circuit(
        &mut self, index: usize, updated: LightingCircuit
    ) -> Result<(), &'static str> {
        if updated.upstream_differential.is_some_and(|i| i >= self.differentials.len()) {
            return Err("Differentiel en amont introuvable");
        }
        let letter = updated.letter.trim().to_ascii_uppercase();
        if letter.is_empty() || letter.chars().count() > 4
            || !letter.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("Repere de circuit: 1 a 4 lettres ou chiffres");
        }
        if (0..self.circuit_count()).any(|other| {
            other != index && self.circuit(other)
                .is_some_and(|c| c.letter.eq_ignore_ascii_case(letter))
        }) {
            return Err("Ce repere de circuit existe deja");
        }
        let mut edited = updated;
        edited.letter = letter;
        if index == 0 {
            self.lighting_circuit = edited;
        } else if let Some(circuit) = self.additional_lighting_circuits.get_mut(index - 1) {
            *circuit = edited;
        } else {
            return Err("Circuit introuvable");
        }
        Ok(())
    }

    /// Suppression reservee aux circuits sans liaison pour ne perdre aucun appareil.
    pub fn remove_lighting_circuit(&mut self, index: usize) -> Result<(), &'static str> {
        if index == 0 || index >= self.circuit_count() {
            return Err("Ce circuit ne peut pas etre supprime");
        }
        if self.lighting_connections.iter().any(|link| link.circuit_index == index)
            || self.outlet_assignments.iter().any(|item| item.circuit_index == index) {
            return Err("Retire d'abord les appareils et liaisons de ce circuit");
        }
        self.additional_lighting_circuits.remove(index - 1);
        for link in &mut self.lighting_connections {
            if link.circuit_index > index { link.circuit_index -= 1; }
        }
        for assignment in &mut self.outlet_assignments {
            if assignment.circuit_index > index { assignment.circuit_index -= 1; }
        }
        Ok(())
    }

    pub fn assign_outlet_to_circuit(
        &mut self, symbol_index: usize, circuit_index: usize
    ) -> Result<(), &'static str> {
        if self.circuit(circuit_index).is_none() {
            return Err("Circuit inexistant");
        }
        if !matches!(self.symbols.get(symbol_index), Some(s) if s.kind == ElectricalKind::Outlet) {
            return Err("Selectionne une prise valide");
        }
        if let Some(existing) = self.outlet_assignments.iter_mut()
            .find(|item| item.symbol_index == symbol_index)
        {
            existing.circuit_index = circuit_index;
        } else {
            self.outlet_assignments.push(OutletAssignment { symbol_index, circuit_index });
        }
        Ok(())
    }

    pub fn unassign_outlet(&mut self, symbol_index: usize) {
        self.outlet_assignments.retain(|item| item.symbol_index != symbol_index);
    }

    pub fn active_outlet_assignments(&self) -> Vec<&OutletAssignment> {
        let mut seen = std::collections::HashSet::new();
        self.outlet_assignments.iter().filter(|item| {
            matches!(self.symbols.get(item.symbol_index), Some(s) if s.kind == ElectricalKind::Outlet)
                && self.circuit(item.circuit_index).is_some()
                && seen.insert(item.symbol_index)
        }).collect()
    }

    pub fn outlets_on_circuit(&self, circuit_index: usize) -> Vec<usize> {
        self.active_outlet_assignments().into_iter()
            .filter(|item| item.circuit_index == circuit_index)
            .map(|item| item.symbol_index)
            .collect()
    }

    pub fn unassigned_outlets(&self) -> usize {
        let valid = self.active_outlet_assignments();
        self.symbols.iter().enumerate().filter(|(index, symbol)| {
            symbol.kind == ElectricalKind::Outlet
                && !valid.iter().any(|item| item.symbol_index == *index)
        }).count()
    }

    /// Ancien appel : toujours lie au circuit historique (index 0).
    pub fn connect_switch_to_light(
        &mut self,
        switch_index: usize,
        light_index: usize,
    ) -> Result<(), &'static str> {
        self.connect_switch_to_light_in_circuit(switch_index, light_index, 0)
    }

    /// Lie un interrupteur et une lampe sur le circuit choisi. Ce n'est
    /// ni un trace de cable ni une validation de cablage.
    pub fn connect_switch_to_light_in_circuit(
        &mut self,
        switch_index: usize,
        light_index: usize,
        circuit_index: usize,
    ) -> Result<(), &'static str> {
        if self.circuit(circuit_index).is_none() {
            return Err("Le circuit selectionne n'existe pas");
        }
        if !matches!(self.symbols.get(switch_index), Some(s) if s.kind == ElectricalKind::Switch) {
            return Err("Choisis un interrupteur valide");
        }
        if !matches!(self.symbols.get(light_index), Some(s) if s.kind == ElectricalKind::Light) {
            return Err("Choisis un point lumineux valide");
        }
        if self.lighting_connections.iter().any(|c| c.light_index == light_index) {
            return Err("Ce point lumineux est deja lie a un interrupteur");
        }
        if self.lighting_connections.iter().any(|c| {
            c.switch_index == switch_index && c.circuit_index != circuit_index
        }) {
            return Err("Un interrupteur ne peut pas commander plusieurs circuits ici");
        }
        self.lighting_connections.push(LightingConnection {
            switch_index,
            light_index,
            circuit_index,
        });
        Ok(())
    }

    /// Deplace l'ensemble des lampes d'un interrupteur vers un autre depart.
    /// Les points lumineux restent associes a leur commande.
    pub fn move_lighting_group(
        &mut self, switch_index: usize, target_circuit: usize
    ) -> Result<(), &'static str> {
        if self.circuit(target_circuit).is_none() { return Err("Circuit cible inexistant"); }
        if !matches!(self.symbols.get(switch_index), Some(s) if s.kind == ElectricalKind::Switch) {
            return Err("L'element selectionne n'est pas un interrupteur");
        }
        let mut found = false;
        for link in &mut self.lighting_connections {
            if link.switch_index == switch_index {
                link.circuit_index = target_circuit;
                found = true;
            }
        }
        if !found { return Err("Cet interrupteur n'a aucune lampe associee"); }
        Ok(())
    }

    pub fn lighting_connection_is_valid(&self, link: &LightingConnection) -> bool {
        self.circuit(link.circuit_index).is_some()
            && matches!(self.symbols.get(link.switch_index), Some(s) if s.kind == ElectricalKind::Switch)
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
        let mut switch_circuit = std::collections::HashMap::new();
        self.lighting_connections.iter()
            .filter(|link| {
                if !self.lighting_connection_is_valid(link) { return false; }
                if let Some(previous) = switch_circuit.get(&link.switch_index) {
                    if *previous != link.circuit_index { return false; }
                }
                if !seen_lights.insert(link.light_index) { return false; }
                switch_circuit.insert(link.switch_index, link.circuit_index);
                true
            })
            .collect()
    }

    /// Indique si l'entree fait partie des liaisons valides non dupliquees.
    pub fn is_active_lighting_connection(&self, index: usize) -> bool {
        let Some(candidate) = self.lighting_connections.get(index) else { return false; };
        self.active_lighting_connections().into_iter()
            .any(|link| std::ptr::eq(link, candidate))
    }

    /// Regroupe les points lumineux qui partagent le meme interrupteur.
    /// Cette vue a une unique definition, reutilisee par l'interface et l'export.
    /// Un groupe contient un interrupteur et les lampes qu'il commande dans ce circuit.
    pub fn lighting_groups_for_circuit(&self, circuit_index: usize) -> Vec<(usize, Vec<usize>)> {
        let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
        for link in self.active_lighting_connections_for_circuit(circuit_index) {
            if let Some((_, lights)) = groups.iter_mut().find(|(i, _)| *i == link.switch_index) {
                lights.push(link.light_index);
            } else {
                groups.push((link.switch_index, vec![link.light_index]));
            }
        }
        groups
    }

    /// Compatibilite avec les appels existants de groupement.
    pub fn lighting_groups(&self) -> Vec<(usize, Vec<usize>)> {
        self.lighting_groups_for_circuit(0)
    }

    /// Les reperes A1, A2, B1... sont numerotes parmi les appareils du
    /// circuit, d'apres l'ordre des symboles dans le projet. L'ordre physique
    /// reste a verifier lors de l'etablissement du dossier.
    pub fn point_number_on_circuit(
        &self, symbol_index: usize, circuit_index: usize
    ) -> Option<usize> {
        let mut points: Vec<usize> = self.active_lighting_connections_for_circuit(circuit_index)
            .iter().map(|link| link.light_index).collect();
        points.extend(self.outlets_on_circuit(circuit_index));
        points.sort_unstable();
        points.dedup();
        points.iter().position(|index| *index == symbol_index).map(|i| i + 1)
    }

    pub fn point_ref(&self, symbol_index: usize) -> Option<String> {
        let symbol = self.symbols.get(symbol_index)?;
        let circuit_index = match symbol.kind {
            ElectricalKind::Light => self.active_lighting_connections().into_iter()
                .find(|link| link.light_index == symbol_index)?.circuit_index,
            ElectricalKind::Outlet => self.active_outlet_assignments().into_iter()
                .find(|item| item.symbol_index == symbol_index)?.circuit_index,
            ElectricalKind::Switch => return None,
        };
        let letter = &self.circuit(circuit_index)?.letter;
        Some(format!(
            "{letter}{}",
            self.point_number_on_circuit(symbol_index, circuit_index)?
        ))
    }

    pub fn lighting_point_number(&self, light_index: usize) -> Option<usize> {
        let link = self.active_lighting_connections().into_iter()
            .find(|link| link.light_index == light_index)?;
        self.point_number_on_circuit(light_index, link.circuit_index)
    }

    pub fn lighting_point_ref(&self, light_index: usize) -> Option<String> {
        if self.symbols.get(light_index)?.kind != ElectricalKind::Light { return None; }
        self.point_ref(light_index)
    }

    /// Pour une commande unique on montre les references des lampes pilotees.
    pub fn switch_point_refs(&self, switch_index: usize) -> Vec<String> {
        self.active_lighting_connections().into_iter()
            .filter(|link| link.switch_index == switch_index)
            .filter_map(|link| self.point_ref(link.light_index))
            .collect()
    }

    pub fn active_lighting_connections_for_circuit(
        &self, circuit_index: usize
    ) -> Vec<&LightingConnection> {
        self.active_lighting_connections().into_iter()
            .filter(|link| link.circuit_index == circuit_index).collect()
    }

    pub fn unlinked_switches(&self) -> usize {
        let active = self.active_lighting_connections();
        self.symbols.iter().enumerate().filter(|(index, symbol)| {
            symbol.kind == ElectricalKind::Switch
                && !active.iter().any(|link| link.switch_index == *index)
        }).count()
    }

    pub fn unlinked_lights(&self) -> usize {
        let active = self.active_lighting_connections();
        self.symbols.iter().enumerate().filter(|(index, symbol)| {
            symbol.kind == ElectricalKind::Light
                && !active.iter().any(|link| link.light_index == *index)
        }).count()
    }

    /// Ajoute une demonstration fonctionnelle dans les deux vues, sans effacer le projet.
    pub fn add_demo_lighting(&mut self) {
        let _ = self.add_demo_lighting_in_circuit(0);
    }

    pub fn add_demo_lighting_in_circuit(
        &mut self, circuit_index: usize
    ) -> Result<(), &'static str> {
        if self.circuit(circuit_index).is_none() {
            return Err("Circuit de demonstration inexistant");
        }
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
        // L'ajout de symboles est sur un plan local valide ; la liaison ne peut echouer.
        self.connect_switch_to_light_in_circuit(switch_index, light_index, circuit_index)
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
    fn opening_can_slide_but_cannot_overlap_or_leave_wall() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
            thickness_m: 0.2,
        });
        plan.add_opening(0, Point { x: 1.0, y: 0.0 }, 0.9, OpeningKind::Door, false, false, 0.05).unwrap();
        plan.add_opening(0, Point { x: 4.0, y: 0.0 }, 1.2, OpeningKind::Window, false, false, 0.05).unwrap();
        plan.move_opening(0, 2.0).unwrap();
        assert_eq!(plan.openings[0].center_m, 2.0);
        assert!(plan.move_opening(0, 3.1).is_err());
        assert!(plan.move_opening(0, 0.1).is_err());
        assert_eq!(plan.openings[0].center_m, 2.0);
        plan.remove_opening(0).unwrap();
        assert_eq!(plan.openings.len(), 1);
        assert_eq!(plan.openings[0].kind, OpeningKind::Window);
    }

    #[test]
    fn moving_whole_wall_preserves_attached_opening_and_undo_data() {
        let mut plan = Plan::default();
        plan.walls.push(Wall {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
            thickness_m: 0.2,
        });
        plan.add_opening(0, Point { x: 2.0, y: 0.0 }, 0.9, OpeningKind::Door, false, false, 0.05).unwrap();
        let old_json = serde_json::to_string(&plan).unwrap();
        plan.update_wall(
            0,
            Point { x: 0.0, y: 2.0 },
            Point { x: 5.0, y: 2.0 },
            0.2,
        ).unwrap();
        assert_eq!(plan.openings[0].center_m, 2.0);
        let restored: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(restored.walls[0].start.y, 2.0);
        assert_eq!(restored.openings[0].center_m, 2.0);
        let undo: Plan = serde_json::from_str(&old_json).unwrap();
        assert_eq!(undo.walls[0].start.y, 0.0);
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
    fn different_circuits_have_independent_references_and_survive_json() {
        let mut p = Plan::default();
        p.add_demo_lighting();
        let second_circuit = p.add_lighting_circuit().unwrap();
        assert_eq!(second_circuit, 1);
        let sw = p.symbols.len();
        p.symbols.push(ElectricalSymbol {
            position: Point { x: 5.0, y: 0.0 },
            kind: ElectricalKind::Switch,
        });
        let lamp = p.symbols.len();
        p.symbols.push(ElectricalSymbol {
            position: Point { x: 6.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        p.connect_switch_to_light_in_circuit(sw, lamp, second_circuit).unwrap();
        assert_eq!(p.lighting_point_ref(1).as_deref(), Some("A1"));
        assert_eq!(p.lighting_point_ref(lamp).as_deref(), Some("B1"));
        assert_eq!(p.lighting_groups_for_circuit(0).len(), 1);
        assert_eq!(p.lighting_groups_for_circuit(1).len(), 1);
        let data = serde_json::to_string(&p).unwrap();
        let roundtrip: Plan = serde_json::from_str(&data).unwrap();
        assert_eq!(roundtrip.circuit_count(), 2);
        assert_eq!(roundtrip.lighting_point_ref(lamp).as_deref(), Some("B1"));
        assert!(roundtrip.connect_switch_to_light_in_circuit(sw, 1, 1).is_err());
        assert!(roundtrip.remove_lighting_circuit(1).is_err());
    }

    #[test]
    fn multi_circuit_example_loads_and_has_independent_labels() {
        let plan: Plan = serde_json::from_str(include_str!(
            "../examples/unifilaire-deux-circuits.json"
        )).unwrap();
        assert_eq!(plan.circuit_count(), 2);
        assert_eq!(plan.lighting_groups_for_circuit(0).len(), 1);
        assert_eq!(plan.lighting_groups_for_circuit(1).len(), 1);
        assert_eq!(plan.lighting_point_ref(1).as_deref(), Some("A1"));
        assert_eq!(plan.lighting_point_ref(2).as_deref(), Some("A2"));
        assert_eq!(plan.lighting_point_ref(4).as_deref(), Some("B1"));
        assert_eq!(plan.point_ref(5).as_deref(), Some("B2"));
        assert_eq!(plan.unassigned_outlets(), 0);
        assert_eq!(plan.unlinked_lights(), 0);
        assert_eq!(plan.unlinked_switches(), 0);
    }

    #[test]
    fn demo_can_be_inserted_in_selected_circuit() {
        let mut plan = Plan::default();
        let b = plan.add_lighting_circuit().unwrap();
        plan.add_demo_lighting_in_circuit(b).unwrap();
        assert_eq!(plan.lighting_point_ref(1).as_deref(), Some("B1"));
        assert!(plan.add_demo_lighting_in_circuit(22).is_err());
    }

    #[test]
    fn moving_a_switch_group_updates_all_its_lamps_and_references() {
        let mut p = Plan::default();
        p.add_demo_lighting();
        let extra_light = p.symbols.len();
        p.symbols.push(ElectricalSymbol {
            position: Point { x: 3.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        p.connect_switch_to_light(0, extra_light).unwrap();
        let dest = p.add_lighting_circuit().unwrap();
        p.move_lighting_group(0, dest).unwrap();
        assert_eq!(p.lighting_groups_for_circuit(0).len(), 0);
        assert_eq!(p.lighting_groups_for_circuit(1).len(), 1);
        assert_eq!(p.lighting_point_ref(1).as_deref(), Some("B1"));
        assert_eq!(p.lighting_point_ref(extra_light).as_deref(), Some("B2"));
    }

    #[test]
    fn duplicated_json_switch_across_circuits_only_keeps_one_group() {
        let mut p = Plan::default();
        p.add_demo_lighting();
        p.add_lighting_circuit().unwrap();
        let light = p.symbols.len();
        p.symbols.push(ElectricalSymbol {
            position: Point { x: 4.0, y: 0.0 },
            kind: ElectricalKind::Light,
        });
        p.lighting_connections.push(LightingConnection {
            switch_index: 0, light_index: light, circuit_index: 1,
        });
        assert_eq!(p.active_lighting_connections().len(), 1);
        assert_eq!(p.lighting_groups_for_circuit(1).len(), 0);
    }

    #[test]
    fn legacy_json_assumes_circuit_zero() {
        let data = r#"{"walls":[],"symbols":[{"position":{"x":0.0,"y":0.0},"kind":"Switch"},{"position":{"x":2.0,"y":0.0},"kind":"Light"}],"lighting_connections":[{"switch_index":0,"light_index":1}]}"#;
        let plan: Plan = serde_json::from_str(data).unwrap();
        assert_eq!(plan.circuit_count(), 1);
        assert_eq!(plan.lighting_connections[0].circuit_index, 0);
        assert_eq!(plan.lighting_point_ref(1).as_deref(), Some("A1"));
    }

    #[test]
    fn circuit_rename_checks_duplicates_and_empty() {
        let mut p = Plan::default();
        p.add_lighting_circuit().unwrap();
        let mut c = p.circuit(1).unwrap().clone();
        c.letter = "A".into();
        assert!(p.update_lighting_circuit(1, c.clone()).is_err());
        c.letter = "  ".into();
        assert!(p.update_lighting_circuit(1, c.clone()).is_err());
        c.letter = " C ".into();
        p.update_lighting_circuit(1, c).unwrap();
        assert_eq!(p.circuit(1).unwrap().letter, "C");
        p.remove_lighting_circuit(1).unwrap();
        assert_eq!(p.circuit_count(), 1);
    }

    #[test]
    fn assigned_outlets_have_shared_references_and_preserve_legacy() {
        let mut plan = Plan::default();
        plan.add_demo_lighting(); // symboles 0=I1, 1=L1
        let socket = plan.symbols.len();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 3.0, y: 1.0 },
            kind: ElectricalKind::Outlet,
        });
        assert_eq!(plan.unassigned_outlets(), 1);
        plan.assign_outlet_to_circuit(socket, 0).unwrap();
        assert_eq!(plan.lighting_point_ref(1).as_deref(), Some("A1"));
        assert_eq!(plan.point_ref(socket).as_deref(), Some("A2"));
        assert_eq!(plan.unassigned_outlets(), 0);
        let out = serde_json::to_string(&plan).unwrap();
        let restored: Plan = serde_json::from_str(&out).unwrap();
        assert_eq!(restored.point_ref(socket).as_deref(), Some("A2"));
        plan.remove_symbol(1).unwrap();
        assert_eq!(plan.point_ref(1).as_deref(), Some("A1"));
        assert_eq!(plan.outlet_assignments[0].symbol_index, 1);
        let old: Plan = serde_json::from_str(r#"{"walls":[],"symbols":[]}"#).unwrap();
        assert!(old.outlet_assignments.is_empty());
        assert!(old.installation_info.voltage.is_empty());
    }

    #[test]
    fn outlet_assignment_rejects_wrong_kind_and_missing_circuit() {
        let mut plan = Plan::default();
        plan.add_demo_lighting();
        assert!(plan.assign_outlet_to_circuit(0, 0).is_err());
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 0.0, y: 0.0 },
            kind: ElectricalKind::Outlet,
        });
        assert!(plan.assign_outlet_to_circuit(2, 4).is_err());
        plan.assign_outlet_to_circuit(2, 0).unwrap();
        plan.assign_outlet_to_circuit(2, 0).unwrap();
        assert_eq!(plan.outlet_assignments.len(), 1);
        plan.unassign_outlet(2);
        assert_eq!(plan.unassigned_outlets(), 1);
    }

    #[test]
    fn symbol_appearances_are_type_safe_and_backward_compatible() {
        let mut p = Plan::default();
        p.symbols.push(ElectricalSymbol {
            position: Point { x: 1.0, y: 1.0 }, kind: ElectricalKind::Outlet
        });
        p.symbols.push(ElectricalSymbol {
            position: Point { x: 2.0, y: 1.0 }, kind: ElectricalKind::Switch
        });
        assert_eq!(p.symbol_appearance(0), ElectricalAppearance::General);
        assert!(p.set_symbol_appearance(0, ElectricalAppearance::WallLight).is_err());
        p.set_symbol_appearance(0, ElectricalAppearance::OutletEarthChild).unwrap();
        p.set_symbol_appearance(1, ElectricalAppearance::SwitchTwoWay).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let decoded: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.symbol_appearance(0), ElectricalAppearance::OutletEarthChild);
        assert_eq!(decoded.symbol_appearance(1), ElectricalAppearance::SwitchTwoWay);
        p.remove_symbol(0).unwrap();
        assert_eq!(p.symbol_appearance(0), ElectricalAppearance::SwitchTwoWay);
        let old: Plan = serde_json::from_str(
            r#"{"walls":[],"symbols":[{"position":{"x":0.0,"y":0.0},"kind":"Outlet"}]}"#
        ).unwrap();
        assert_eq!(old.symbol_appearance(0), ElectricalAppearance::General);
    }

    #[test]
    fn appearance_catalog_has_only_compatible_choices() {
        for kind in [ElectricalKind::Outlet, ElectricalKind::Switch, ElectricalKind::Light] {
            assert!(ElectricalAppearance::choices(kind).iter().all(|style| style.valid_for(kind)));
            assert!(!ElectricalAppearance::choices(kind).is_empty());
        }
    }

    #[test]
    fn one_differential_feeds_multiple_circuits_and_roundtrips() {
        let mut p = Plan::default();
        let b = p.add_lighting_circuit().unwrap();
        let rcd = p.add_differential().unwrap();
        p.set_circuit_differential(0, Some(rcd)).unwrap();
        p.set_circuit_differential(b, Some(rcd)).unwrap();
        let mut updated = p.differentials[rcd].clone();
        updated.kind = "A".into();
        updated.rated_current_amp = 40;
        updated.sensitivity_ma = 30;
        updated.poles = 2;
        p.update_differential(rcd, updated).unwrap();
        assert_eq!(p.circuits_for_differential(rcd), vec![0, 1]);
        assert!(p.circuit_missing_fields(0).iter().all(|name| !name.contains("differentiel")));
        assert!(p.remove_differential(rcd).is_err());
        let saved = serde_json::to_string(&p).unwrap();
        let restored: Plan = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.circuits_for_differential(rcd), vec![0, 1]);
        assert_eq!(restored.differentials[0].sensitivity_ma, 30);
    }

    #[test]
    fn old_free_text_differential_is_not_treated_as_a_connection() {
        let old = r#"{"walls":[],"symbols":[],"lighting_circuit":{"letter":"A","upstream_rdc":"ancien DDR 30mA"}}"#;
        let p: Plan = serde_json::from_str(old).unwrap();
        assert!(p.differentials.is_empty());
        assert!(p.differential_for_circuit(0).is_none());
        assert!(p.circuit_missing_fields(0).iter().any(|name| name.contains("differentiel")));
        assert_eq!(p.lighting_circuit.upstream_rdc, "ancien DDR 30mA");
    }

    #[test]
    fn differential_deletion_reindexes_other_branches_safely() {
        let mut p = Plan::default();
        let first = p.add_differential().unwrap();
        let second = p.add_differential().unwrap();
        p.set_circuit_differential(0, Some(second)).unwrap();
        assert!(p.remove_differential(second).is_err());
        p.remove_differential(first).unwrap();
        assert_eq!(p.differential_for_circuit(0).unwrap().reference, "D2");
        assert_eq!(p.lighting_circuit.upstream_differential, Some(0));
        assert!(p.set_circuit_differential(0, Some(55)).is_err());
        p.set_circuit_differential(0, None).unwrap();
        assert_eq!(p.circuits_without_differential(), vec![0]);
    }

    #[test]
    fn differential_reference_must_be_unique() {
        let mut p = Plan::default();
        p.add_differential().unwrap();
        p.add_differential().unwrap();
        let mut edited = p.differentials[1].clone();
        edited.reference = "d1".into();
        assert!(p.update_differential(1, edited).is_err());
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
