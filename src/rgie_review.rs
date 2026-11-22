//! Pre-controle documentaire du dossier electrique domestique belge.
//! Aucun resultat n'affirme la conformite reglementaire de l'installation.
use crate::model::{ElectricalKind, Plan, ReviewFinding, ReviewLevel};

fn missing(out: &mut Vec<ReviewFinding>, scope: impl Into<String>, message: impl Into<String>) {
    out.push(ReviewFinding {
        level: ReviewLevel::Missing,
        scope: scope.into(),
        message: message.into(),
    });
}
fn verify(out: &mut Vec<ReviewFinding>, scope: impl Into<String>, message: impl Into<String>) {
    out.push(ReviewFinding {
        level: ReviewLevel::ToVerify,
        scope: scope.into(),
        message: message.into(),
    });
}

/// Verification documentaire et de concordance des elements modelises.
/// Cette liste n'est pas une interpretation exhaustive du Livre 1 du RGIE.
pub fn review(plan: &Plan) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let info = &plan.installation_info;
    let fields = [
        ("Adresse du lieu", &info.address),
        ("Responsable de l'execution : nom", &info.executor_name),
        ("Responsable de l'execution : qualite", &info.executor_role),
        ("Tension nominale", &info.voltage),
        ("Nature du courant", &info.current_kind),
        ("Tableau de distribution", &info.panel_name),
    ];
    for (field, value) in fields {
        if value.trim().is_empty() {
            missing(&mut findings, "Identification du dossier", format!("{field} non renseigne"));
        }
    }
    if info.drawing_date.trim().is_empty() {
        missing(&mut findings, "Identification du dossier", "Date du schema non renseignee");
    }
    if info.revision.trim().is_empty() {
        missing(&mut findings, "Identification du dossier", "Revision non renseignee");
    }
    for circuit_index in 0..plan.circuit_count() {
        if let Some(circuit) = plan.circuit(circuit_index) {
            let scope = format!("Circuit {}", circuit.letter);
            for field in circuit.missing_fields() {
                missing(&mut findings, &scope, format!("{field} non renseigne"));
            }
            let lights = plan.active_lighting_connections_for_circuit(circuit_index).len();
            let outlets = plan.outlets_on_circuit(circuit_index).len();
            if lights + outlets == 0 {
                missing(&mut findings, &scope, "Aucun point lumineux ou prise attribue");
            }
            if lights > 0 && outlets > 0 {
                verify(&mut findings, &scope,
                    "Circuit avec prises et eclairage : verifier les conditions des circuits mixtes selon le RGIE");
            }
            // Les valeurs numeriques saisies ne sont pas un calcul de dimensionnement.
            if circuit.breaker_amp > 0 && circuit.section_mm2 > 0.0 {
                verify(
                    &mut findings, &scope,
                    "Verifier le choix de protection et la section selon installation, pose et conditions reelles",
                );
            }
        }
    }

    let invalid_lights = plan.lighting_connections.len()
        .saturating_sub(plan.active_lighting_connections().len());
    if invalid_lights > 0 {
        missing(
            &mut findings, "Concordance plan/unifilaire",
            format!("{invalid_lights} liaison(s) d'eclairage invalide(s) ou en doublon"),
        );
    }
    let invalid_outlets = plan.outlet_assignments.len()
        .saturating_sub(plan.active_outlet_assignments().len());
    if invalid_outlets > 0 {
        missing(
            &mut findings, "Concordance plan/unifilaire",
            format!("{invalid_outlets} affectation(s) de prise invalide(s) ou en doublon"),
        );
    }
    let without_light = plan.unlinked_lights();
    let without_switch = plan.unlinked_switches();
    let without_outlet = plan.unassigned_outlets();
    if without_light > 0 {
        missing(&mut findings, "Concordance plan/unifilaire",
            format!("{without_light} luminaire(s) sans commande representee"));
    }
    if without_switch > 0 {
        missing(&mut findings, "Concordance plan/unifilaire",
            format!("{without_switch} interrupteur(s) sans point commande"));
    }
    if without_outlet > 0 {
        missing(&mut findings, "Concordance plan/unifilaire",
            format!("{without_outlet} prise(s) sans circuit attribue"));
    }
    if plan.symbols.iter().all(|s| s.kind != ElectricalKind::Outlet) {
        verify(&mut findings, "Concordance plan/unifilaire",
            "Verifier si l'installation comporte des prises absentes du projet");
    }

    verify(
        &mut findings, "Limite du logiciel",
        "Symboles de travail : verifier les conventions du chapitre 2.13 et la legende",
    );
    verify(
        &mut findings, "Limite du logiciel",
        "Les boites, appareils fixes, sources, dispositifs amont et trajets de conducteurs ne sont pas tous modelises",
    );
    verify(
        &mut findings, "Verification externe",
        "La valeur des champs ne garantit pas la section, les protections, la mise a la terre, les volumes ou le dimensionnement reel",
    );
    verify(
        &mut findings, "Verification externe",
        "Les reperes sont numerotes selon les symboles du fichier : verifier leur ordre physique depuis chaque protection",
    );
    verify(
        &mut findings, "Verification externe",
        "Une verification par l'organisme agree reste necessaire lorsque le RGIE l'exige",
    );
    findings
}

/// Rapport visible dans l'application, sans certification.
pub fn show(ui: &mut eframe::egui::Ui, plan: &Plan) {
    ui.heading("Preparation du dossier RGIE - controle de completude");
    ui.label("Reference : RGIE belge Livre 1, sous-sections 3.1.2.1 et 3.1.2.2.");
    ui.colored_label(
        eframe::egui::Color32::from_rgb(185, 93, 21),
        "Ce controle detecte des informations manquantes. Il ne certifie pas la conformite de l'installation.",
    );
    let findings = review(plan);
    let missing = findings.iter().filter(|f| f.level == ReviewLevel::Missing).count();
    let verify = findings.iter().filter(|f| f.level == ReviewLevel::ToVerify).count();
    ui.heading(format!("{missing} element(s) incomplet(s) - {verify} point(s) a verifier"));
    ui.label("Corrige les informations dans le panneau de gauche, puis reviens ici.");
    ui.separator();
    eframe::egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for finding in &findings {
                ui.horizontal_wrapped(|ui| {
                    let color = match finding.level {
                        ReviewLevel::Missing => eframe::egui::Color32::from_rgb(178, 63, 45),
                        ReviewLevel::ToVerify => eframe::egui::Color32::from_rgb(164, 116, 26),
                    };
                    let label = match finding.level {
                        ReviewLevel::Missing => "INCOMPLET",
                        ReviewLevel::ToVerify => "A VERIFIER",
                    };
                    ui.colored_label(color, label);
                    ui.strong(&finding.scope);
                    ui.label(&finding.message);
                });
                ui.separator();
            }
            ui.label("Source officielle : https://economie.fgov.be/fr/publications/reglement-general-sur-les");
            ui.small("Conserver les schemas et le rapport du controle dans le dossier de l'installation.");
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElectricalSymbol, Point};

    #[test]
    fn empty_dossier_is_incomplete_and_never_claims_conformity() {
        let findings = review(&Plan::default());
        assert!(findings.iter().any(|f| f.message.contains("Adresse")));
        assert!(findings.iter().any(|f| f.level == ReviewLevel::ToVerify));
        assert!(findings.iter().any(|f| f.message.contains("Aucun point")));
    }

    #[test]
    fn catches_outlet_without_circuit_and_recovers_after_assignment() {
        let mut plan = Plan::default();
        plan.symbols.push(ElectricalSymbol {
            position: Point { x: 1.0, y: 1.0 },
            kind: ElectricalKind::Outlet,
        });
        assert!(review(&plan).iter().any(|f| f.message.contains("prise(s) sans circuit")));
        plan.assign_outlet_to_circuit(0, 0).unwrap();
        assert!(!review(&plan).iter().any(|f| f.message.contains("prise(s) sans circuit")));
    }

    #[test]
    fn sample_flags_mixed_circuit_without_falsely_rejecting_it() {
        let plan: Plan = serde_json::from_str(
            include_str!("../examples/unifilaire-deux-circuits.json")
        ).unwrap();
        assert_eq!(plan.unassigned_outlets(), 0);
        assert!(review(&plan).iter().any(|finding| {
            finding.message.contains("circuits mixtes")
                && finding.level == ReviewLevel::ToVerify
        }));
    }

    #[test]
    fn complete_fields_do_not_remove_external_review_warnings() {
        let mut plan = Plan::default();
        plan.installation_info.address = "Adresse de test".into();
        plan.installation_info.executor_name = "Responsable test".into();
        plan.installation_info.executor_role = "Installateur".into();
        plan.installation_info.voltage = "230 V".into();
        plan.installation_info.current_kind = "AC".into();
        plan.installation_info.panel_name = "Tableau principal".into();
        plan.installation_info.drawing_date = "2026-11-22".into();
        plan.installation_info.revision = "01".into();
        let findings = review(&plan);
        assert!(findings.iter().any(|f| f.level == ReviewLevel::ToVerify));
        assert!(!findings.iter().any(|f| f.message.contains("Adresse")));
    }
}
