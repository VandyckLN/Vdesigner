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
    std::fs::write(&path, &text).map_err(|e| PaletteIoError::Io(e.to_string()))?;

    if palette.gerar.contains(&Generated::Css) {
        std::fs::write(dir.join(CSS_FILE_NAME), to_css_vars(palette)?)
            .map_err(|e| PaletteIoError::Io(e.to_string()))?;
    }
    if palette.gerar.contains(&Generated::Tailwind) {
        std::fs::write(dir.join(TAILWIND_FILE_NAME), to_tailwind(palette)?)
            .map_err(|e| PaletteIoError::Io(e.to_string()))?;
    }

    Ok(text)
}
