//! Resultat chiffre d'une main pour un joueur (PRD §9.1/§13.4, M6-3) :
//! jetons nets gagnes/perdus, independant du calcul d'equite (§10.6,
//! `gr-equity`). Identite comptable : gains (`hand.pots[].winners`) moins
//! jetons mises (`hand.actions`, montants deja incrementaux y compris pour
//! les relances — `gr-parser-winamax::parse_action_line` normalise
//! `raises X to Y` en un `amount` incremental, pas le total `to_amount`) ;
//! l'egalite Σ gains = Σ mises sur toute la main est deja garantie a
//! l'analyse (PAR-11), donc valable sans ajustement particulier meme sur
//! une mise/relance jamais suivie (le joueur la recupere en remportant le
//! pot qui la contient, jamais via une ligne "Uncalled bet" separee —
//! jamais observee dans le corpus, `docs/formats/winamax.md` §7).
//!
//! Colonnes `hand_players.net_chips`/`net_bb`, prevues depuis `0001_init.sql`
//! (M2-1) mais jamais calculees avant cette story (report differe depuis
//! M2-2, jamais repris par M4).

use gr_core::{ActionKind, HandRecord};

/// Jetons nets gagnes (positif) ou perdus (negatif) par `pseudo` sur `hand`.
/// `0` si le joueur n'a pas participe (aucune action, aucun pot).
#[must_use]
pub fn compute_net_chips(hand: &HandRecord, pseudo: &str) -> i64 {
    let contributed: i64 = hand
        .actions
        .iter()
        .filter(|a| a.pseudo == pseudo)
        .filter(|a| {
            matches!(
                a.kind,
                ActionKind::PostAnte
                    | ActionKind::PostSmallBlind
                    | ActionKind::PostBigBlind
                    | ActionKind::Call
                    | ActionKind::Bet
                    | ActionKind::Raise
            )
        })
        .filter_map(|a| a.amount)
        .map(gr_core::Chips::amount)
        .sum();

    let won: i64 = hand
        .pots
        .iter()
        .flat_map(|pot| &pot.winners)
        .filter(|(winner, _)| winner == pseudo)
        .map(|(_, amount)| amount.amount())
        .sum();

    won - contributed
}

/// `compute_net_chips` normalise en bb (`hand.bb`). `None` si `bb == 0`
/// (division par zero evitee, R-NOPANIC — main sans blindes valides,
/// jamais observee mais pas exclue par le type).
#[must_use]
pub fn compute_net_bb(hand: &HandRecord, pseudo: &str) -> Option<f64> {
    let bb = hand.bb.amount();
    if bb == 0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let net_bb = compute_net_chips(hand, pseudo) as f64 / bb as f64;
    Some(net_bb)
}
