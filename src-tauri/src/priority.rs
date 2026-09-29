//! Priorite processus (PRD §6.2) : la machine cible (Celeron N5095A) est
//! faible, Graphite ne doit jamais concurrencer le client Winamax pour le
//! CPU. Appliquee inconditionnellement au demarrage plutot que seulement
//! "pendant le jeu" : `gr-winmon` (detection de fenetre Win32) est encore
//! vide, et ajouter une detection en direct maintenant aurait elargi le
//! perimetre de M3-3 au-dela de ce qui a ete valide (decision Frederic,
//! 29/09).

#[cfg(windows)]
pub fn set_below_normal() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS,
    };
    // Safety: GetCurrentProcess renvoie un pseudo-handle qui ne peut pas
    // echouer ; SetPriorityClass sur notre propre processus ne touche a
    // aucun etat Rust.
    unsafe {
        SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
}

#[cfg(not(windows))]
pub fn set_below_normal() {}
