use std::{fs, io, path::Path};

/// Moves the library's storage file from the legacy `~/Library/Application Support` location
/// to the XDG path resolved by `use_xdg_layout`.
///
/// Behavior:
/// 1. If `<legacy>/<primary_filename>` exists and `<xdg>/<primary_filename>` does not, rename it.
/// 2. Otherwise, if `<legacy>/persist.json` exists and `<xdg>/persist.json` does not, rename it.
/// 3. Otherwise, do nothing.
///
/// Never overwrites an existing file at the destination. The `xdg_dir == legacy_dir` case is
/// a no-op (defensive guard; the paths should not coincide in practice).
pub(crate) fn migrate_legacy_to_xdg(
    xdg_dir: &Path,
    legacy_dir: &Path,
    primary_filename: &str,
) -> io::Result<()> {
    if xdg_dir == legacy_dir {
        return Ok(());
    }
    fs::create_dir_all(xdg_dir)?;

    let primary_from = legacy_dir.join(primary_filename);
    let primary_to = xdg_dir.join(primary_filename);
    if primary_from.exists() {
        if !primary_to.exists() {
            fs::rename(&primary_from, &primary_to)?;
        }
        return Ok(());
    }

    let legacy_from = legacy_dir.join("persist.json");
    let legacy_to = xdg_dir.join("persist.json");
    if legacy_from.exists() && !legacy_to.exists() {
        fs::rename(&legacy_from, &legacy_to)?;
    }

    Ok(())
}
