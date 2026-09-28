use std::path::{Path, PathBuf};

/// Nom du dossier de donnees en mode portable (ADR-007).
const PORTABLE_DIR_NAME: &str = "graphite-data";
/// Sous-dossier applicatif dans `%LOCALAPPDATA%`.
const APPDATA_SUBDIR: &str = "Graphite";

/// Resout le dossier de donnees reel (ADR-007) a partir du dossier contenant
/// l'executable courant : `<exe_dir>/graphite-data/` s'il existe deja ou si
/// l'executable tourne depuis un lecteur amovible (cle USB), sinon
/// `%LOCALAPPDATA%\Graphite\`.
pub fn resolve_data_dir(exe_dir: &Path) -> PathBuf {
    let portable_dir_exists = exe_dir.join(PORTABLE_DIR_NAME).is_dir();
    let is_removable_drive = is_removable_drive(exe_dir);
    let local_appdata = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    pick_data_dir(
        exe_dir,
        portable_dir_exists,
        is_removable_drive,
        local_appdata.as_deref(),
    )
}

/// Logique pure de l'ADR-007, testable sans toucher au systeme de fichiers
/// ni au registre Windows.
fn pick_data_dir(
    exe_dir: &Path,
    portable_dir_exists: bool,
    is_removable_drive: bool,
    local_appdata: Option<&Path>,
) -> PathBuf {
    let portable = exe_dir.join(PORTABLE_DIR_NAME);
    if portable_dir_exists || is_removable_drive {
        return portable;
    }
    match local_appdata {
        Some(appdata) => appdata.join(APPDATA_SUBDIR),
        // Environnement degrade (variable absente) : le mode portable reste
        // un repli sur, cree au besoin a cote de l'executable.
        None => portable,
    }
}

#[cfg(windows)]
fn is_removable_drive(exe_dir: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_REMOVABLE;

    let Some(root) = exe_dir.ancestors().last() else {
        return false;
    };
    let wide: Vec<u16> = root
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` est une chaine UTF-16 valide, terminee par un NUL, et
    // reste vivante pour toute la duree de l'appel FFI.
    let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
    drive_type == DRIVE_REMOVABLE
}

#[cfg(not(windows))]
fn is_removable_drive(_exe_dir: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_existing_portable_dir() {
        let exe_dir = Path::new(r"D:\Graphite");
        let appdata = Path::new(r"C:\Users\fred\AppData\Local");
        let picked = pick_data_dir(exe_dir, true, false, Some(appdata));
        assert_eq!(picked, exe_dir.join(PORTABLE_DIR_NAME));
    }

    #[test]
    fn uses_portable_dir_on_removable_drive_even_without_existing_folder() {
        let exe_dir = Path::new(r"E:\Graphite");
        let picked = pick_data_dir(exe_dir, false, true, None);
        assert_eq!(picked, exe_dir.join(PORTABLE_DIR_NAME));
    }

    #[test]
    fn falls_back_to_local_appdata_on_a_fixed_drive() {
        let exe_dir = Path::new(r"C:\Program Files\Graphite");
        let appdata = Path::new(r"C:\Users\fred\AppData\Local");
        let picked = pick_data_dir(exe_dir, false, false, Some(appdata));
        assert_eq!(picked, appdata.join(APPDATA_SUBDIR));
    }

    #[test]
    fn falls_back_to_portable_dir_when_local_appdata_is_unavailable() {
        let exe_dir = Path::new(r"C:\Program Files\Graphite");
        let picked = pick_data_dir(exe_dir, false, false, None);
        assert_eq!(picked, exe_dir.join(PORTABLE_DIR_NAME));
    }
}
