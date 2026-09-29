//! Generation de `ui/src/bindings.ts` depuis les types Rust (`ts-rs`, PRD §7
//! "les types TS sont generes depuis Rust... dans ui/src/bindings.ts, fichier
//! genere, ne pas editer"). Regroupe manuellement plusieurs types dans un seul
//! fichier : le mecanisme `#[ts(export)]` automatique de `ts-rs` ecrit un
//! fichier par type et les runs de tests se marchent dessus s'ils partagent
//! le meme chemin de sortie (observe empiriquement, M3-1).
//!
//! Regenere a chaque `cargo test -p graphite` (donc `just check`).

use std::path::Path;

use ts_rs::TS;

use crate::hero_profiles::HeroProfilePayload;
use crate::home::{
    HomeSnapshotPayload, ImportStatusPayload, LastSessionPayload, PeriodKpis, ProfitCurvePoint,
};
use crate::import::{ImportProgressPayload, ImportSummaryPayload};
use crate::setup::WinamaxAccountPayload;
use crate::status::StatusSnapshotPayload;
use crate::watch::{HandsNewPayload, WatcherRunState};

#[test]
fn generate_ui_bindings() {
    let mut decls = vec![
        WinamaxAccountPayload::decl(),
        ImportProgressPayload::decl(),
        ImportSummaryPayload::decl(),
        HandsNewPayload::decl(),
        StatusSnapshotPayload::decl(),
        WatcherRunState::decl(),
        HeroProfilePayload::decl(),
        HomeSnapshotPayload::decl(),
        PeriodKpis::decl(),
        ProfitCurvePoint::decl(),
        LastSessionPayload::decl(),
        ImportStatusPayload::decl(),
    ];
    decls.sort();

    let mut content =
        String::from("// Ce fichier est genere par ts-rs (M3-1). Ne pas l'editer a la main.\n\n");
    for decl in decls {
        content.push_str("export ");
        content.push_str(&decl);
        content.push_str("\n\n");
    }

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/bindings.ts");
    std::fs::write(&path, content).expect("write ui/src/bindings.ts");
}
