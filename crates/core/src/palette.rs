//! The palette file format. Pure: turns text into data and data into text,
//! and never opens a file — the Tauri layer owns the disk.

use crate::color::{format, gradient, parse_hex, ramp, Color, ColorFormat, RAMP_STEPS};
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
    let mut colour_names = HashSet::new();
    for swatch in &palette.cores {
        validate_name(&swatch.nome)?;
        if !seen.insert(swatch.nome.as_str()) {
            return Err(CoreError::InvalidParameter(std::format!(
                "a cor `{}` aparece duas vezes",
                swatch.nome
            )));
        }
        colour_names.insert(swatch.nome.as_str());
        swatch.color()?;
    }

    for gradient in &palette.degrades {
        validate_name(&gradient.nome)?;
        // Gradients and swatches both become `--<nome>` CSS custom properties,
        // so they share one namespace: a name collision would make one
        // declaration silently overwrite the other via the CSS cascade.
        if !seen.insert(gradient.nome.as_str()) {
            return Err(CoreError::InvalidParameter(std::format!(
                "o nome `{}` já está em uso por outra cor ou degradê",
                gradient.nome
            )));
        }
        for endpoint in [&gradient.de, &gradient.para] {
            if !colour_names.contains(endpoint.as_str()) {
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

pub const CSS_FILE_NAME: &str = "cores.css";
pub const TAILWIND_FILE_NAME: &str = "cores.tailwind.json";

/// Steps drawn for a gradient in the generated CSS. Five is enough for the
/// browser to interpolate smoothly between them while keeping the declaration
/// short enough to read in a diff.
const GRADIENT_STOPS: usize = 5;

pub fn to_css_vars(palette: &Palette) -> Result<String, CoreError> {
    validate_palette(palette)?;

    let mut out = String::from(":root {\n");

    for swatch in &palette.cores {
        let color = swatch.color()?;
        if swatch.rampa {
            for (step, shade) in RAMP_STEPS.iter().zip(ramp(color)) {
                out.push_str(&std::format!(
                    "  --{}-{step}: {};\n",
                    swatch.nome,
                    format(shade, ColorFormat::Hex)
                ));
            }
        } else {
            out.push_str(&std::format!(
                "  --{}: {};\n",
                swatch.nome,
                format(color, ColorFormat::Hex)
            ));
        }
    }

    for reference in &palette.degrades {
        let stops = gradient_stops(palette, reference)?;
        out.push_str(&std::format!(
            "  --{}: linear-gradient(90deg, {});\n",
            reference.nome,
            stops.join(", ")
        ));
    }

    out.push_str("}\n");
    Ok(out)
}

fn gradient_stops(palette: &Palette, reference: &GradientRef) -> Result<Vec<String>, CoreError> {
    let find = |name: &str| {
        palette
            .cores
            .iter()
            .find(|s| s.nome == name)
            .ok_or_else(|| {
                CoreError::InvalidParameter(std::format!(
                    "o degradê `{}` aponta para a cor `{name}`, que não existe na paleta",
                    reference.nome
                ))
            })
    };

    let from = find(&reference.de)?.color()?;
    let to = find(&reference.para)?.color()?;

    Ok(gradient(from, to, GRADIENT_STOPS)?
        .into_iter()
        .map(|c| format(c, ColorFormat::Hex))
        .collect())
}

pub fn to_tailwind(palette: &Palette) -> Result<String, CoreError> {
    validate_palette(palette)?;

    let mut root = serde_json::Map::new();
    for swatch in &palette.cores {
        let color = swatch.color()?;
        if swatch.rampa {
            let mut scale = serde_json::Map::new();
            for (step, shade) in RAMP_STEPS.iter().zip(ramp(color)) {
                scale.insert(step.to_string(), format(shade, ColorFormat::Hex).into());
            }
            root.insert(swatch.nome.clone(), scale.into());
        } else {
            root.insert(swatch.nome.clone(), format(color, ColorFormat::Hex).into());
        }
    }

    serde_json::to_string_pretty(&root)
        .map_err(|e| CoreError::InvalidParameter(std::format!("falha ao gerar o Tailwind: {e}")))
}
