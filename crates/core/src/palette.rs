//! The palette file format. Pure: turns text into data and data into text,
//! and never opens a file — the Tauri layer owns the disk.

use crate::color::{parse_hex, Color};
use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Bumped whenever the shape changes. Reading a file from the future without
/// noticing it is from the future corrupts it silently on the next write.
pub const PALETTE_FORMAT_VERSION: u32 = 1;

pub const PALETTE_FILE_NAME: &str = "vdesigner-cores.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Generated {
    Css,
    Tailwind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Swatch {
    pub nome: String,
    pub hex: String,
    /// Whether this colour expands into the 50-900 scale in generated output.
    /// Not every colour earns ten rungs; a border colour needs one.
    #[serde(default)]
    pub rampa: bool,
}

impl Swatch {
    pub fn color(&self) -> Result<Color, CoreError> {
        parse_hex(&self.hex)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradientRef {
    pub nome: String,
    /// Names, not values. Editing a colour updates the gradient; copies would
    /// drift apart on the first edit.
    pub de: String,
    pub para: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub versao: u32,
    pub nome: String,
    #[serde(default)]
    pub gerar: Vec<Generated>,
    #[serde(default)]
    pub cores: Vec<Swatch>,
    #[serde(default)]
    pub degrades: Vec<GradientRef>,
}

pub fn palette_from_json(text: &str) -> Result<Palette, CoreError> {
    let palette: Palette = serde_json::from_str(text).map_err(|e| {
        CoreError::InvalidParameter(std::format!("arquivo de paleta ilegível: {e}"))
    })?;

    if palette.versao > PALETTE_FORMAT_VERSION {
        return Err(CoreError::InvalidParameter(std::format!(
            "este arquivo usa a versão {} do formato e esta build entende até a {}; \
             atualize o Vdesigner em vez de gravar por cima",
            palette.versao,
            PALETTE_FORMAT_VERSION
        )));
    }

    validate_palette(&palette)?;
    Ok(palette)
}

pub fn palette_to_json(palette: &Palette) -> Result<String, CoreError> {
    validate_palette(palette)?;
    serde_json::to_string_pretty(palette)
        .map_err(|e| CoreError::InvalidParameter(std::format!("falha ao gerar o JSON: {e}")))
}

pub fn validate_palette(palette: &Palette) -> Result<(), CoreError> {
    let mut seen = HashSet::new();
    for swatch in &palette.cores {
        validate_name(&swatch.nome)?;
        if !seen.insert(swatch.nome.as_str()) {
            return Err(CoreError::InvalidParameter(std::format!(
                "a cor `{}` aparece duas vezes",
                swatch.nome
            )));
        }
        swatch.color()?;
    }

    for gradient in &palette.degrades {
        validate_name(&gradient.nome)?;
        for endpoint in [&gradient.de, &gradient.para] {
            if !seen.contains(endpoint.as_str()) {
                return Err(CoreError::InvalidParameter(std::format!(
                    "o degradê `{}` aponta para a cor `{endpoint}`, que não existe na paleta",
                    gradient.nome
                )));
            }
        }
    }

    Ok(())
}

/// Names become CSS custom property names. A space or an accent produces
/// broken CSS, and the breakage would only surface in somebody else's browser.
fn validate_name(name: &str) -> Result<(), CoreError> {
    let shaped = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-');

    if shaped {
        Ok(())
    } else {
        Err(CoreError::InvalidParameter(std::format!(
            "`{name}` não serve como nome: use apenas letras minúsculas sem acento, \
             números e hífen, sem hífen nas pontas"
        )))
    }
}
