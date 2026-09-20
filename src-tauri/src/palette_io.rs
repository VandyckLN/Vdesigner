//! Reads and writes the palette in the folder the user picked. The folder is
//! the source of truth, so this module never keeps a second copy anywhere.

use std::path::Path;
use thiserror::Error;
use vdesigner_core::{
    palette_from_json, palette_to_json, to_css_vars, to_tailwind, CoreError, Generated, Palette,
    CSS_FILE_NAME, PALETTE_FILE_NAME, TAILWIND_FILE_NAME,
};

#[derive(Debug, Error)]
pub enum PaletteIoError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("falha ao acessar a pasta: {0}")]
    Io(String),

    #[error("o arquivo mudou no disco desde que foi aberto; recarregue antes de gravar")]
    ChangedOnDisk,
}

pub struct LoadedPalette {
    pub palette: Palette,
    /// Exactly the text read from disk. `save` compares against it to notice
    /// an edit made outside the app — a merge conflict resolved by hand, for
    /// instance — instead of silently overwriting it.
    pub on_disk: String,
}

pub fn load(dir: &Path) -> Result<Option<LoadedPalette>, PaletteIoError> {
    let path = dir.join(PALETTE_FILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| PaletteIoError::Io(e.to_string()))?;
    let palette = palette_from_json(&text)?;
    Ok(Some(LoadedPalette {
        palette,
        on_disk: text,
    }))
}

/// Writes `contents` to `path` atomically: the bytes land in a temporary file
/// in the same directory (so it is on the same volume, which is what makes
/// the following rename atomic) and `std::fs::rename` swaps it into place.
/// A crash or full disk can only ever leave the temp file truncated — the
/// destination is either the old, complete file or the new, complete file,
/// never something in between. Without this dance, a direct `fs::write`
/// truncates the destination up front, so a failure partway through would
/// destroy the user's palette with no way back.
fn write_atomically(path: &Path, contents: &str) -> Result<(), PaletteIoError> {
    let dir = path
        .parent()
        .ok_or_else(|| PaletteIoError::Io("caminho do arquivo não tem pasta pai".to_string()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| PaletteIoError::Io("caminho do arquivo é inválido".to_string()))?
        .to_string_lossy();
    let tmp_path = dir.join(format!("{file_name}.{}.tmp", std::process::id()));

    if let Err(e) = std::fs::write(&tmp_path, contents) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(PaletteIoError::Io(e.to_string()));
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(PaletteIoError::Io(e.to_string()));
    }
    Ok(())
}

/// Returns the text written, which the caller keeps as the next
/// `expected_on_disk`.
pub fn save(
    dir: &Path,
    palette: &Palette,
    expected_on_disk: Option<&str>,
) -> Result<String, PaletteIoError> {
    let path = dir.join(PALETTE_FILE_NAME);

    let current = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(PaletteIoError::Io(e.to_string())),
    };
    if current.as_deref() != expected_on_disk {
        return Err(PaletteIoError::ChangedOnDisk);
    }

    let text = palette_to_json(palette)?;

    // Write the derived files first and the palette JSON last. These files
    // are regenerated from the palette on every save, so a plain
    // (truncating) write is fine for them — if one fails, the next
    // successful save just rewrites it. The palette JSON is the only source
    // of truth, so it must go last: if a derived-file write fails after the
    // palette JSON had already been written, `save` would return `Err` while
    // the palette on disk had already changed, leaving the caller's
    // `expected_on_disk` stale and the next save spuriously rejected with
    // `ChangedOnDisk`. Writing the palette JSON last (and atomically, via
    // `write_atomically`) means any failure above leaves it untouched, so
    // `expected_on_disk` stays valid and at worst the derived files are
    // momentarily stale.
    if palette.gerar.contains(&Generated::Css) {
        std::fs::write(dir.join(CSS_FILE_NAME), to_css_vars(palette)?)
            .map_err(|e| PaletteIoError::Io(e.to_string()))?;
    }
    if palette.gerar.contains(&Generated::Tailwind) {
        std::fs::write(dir.join(TAILWIND_FILE_NAME), to_tailwind(palette)?)
            .map_err(|e| PaletteIoError::Io(e.to_string()))?;
    }

    write_atomically(&path, &text)?;

    Ok(text)
}
