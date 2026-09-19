use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("o arquivo já existe: {0}")]
    WouldOverwrite(String),

    #[error("nome de arquivo inválido: {0}")]
    InvalidName(String),

    #[error("a pasta de saída não existe: {0}")]
    InvalidDirectory(String),

    #[error("falha ao gravar: {0}")]
    Io(String),
}

/// Builds the destination path and enforces the overwrite rule. Overwriting an
/// existing file is only ever allowed when the caller asks for it explicitly.
pub fn resolve_output_path(
    directory: &Path,
    stem: &str,
    extension: &str,
    allow_overwrite: bool,
) -> Result<PathBuf, ExportError> {
    let trimmed = stem.trim();
    if trimmed.is_empty() {
        return Err(ExportError::InvalidName(
            "o nome não pode ficar vazio".into(),
        ));
    }
    if trimmed.contains(['/', '\\', ':']) || trimmed.contains("..") {
        return Err(ExportError::InvalidName(format!(
            "o nome não pode conter separador de caminho: {trimmed}"
        )));
    }
    if !directory.is_dir() {
        return Err(ExportError::InvalidDirectory(
            directory.display().to_string(),
        ));
    }

    let path = directory.join(format!("{trimmed}.{extension}"));
    if path.exists() && !allow_overwrite {
        return Err(ExportError::WouldOverwrite(path.display().to_string()));
    }
    Ok(path)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<u64, ExportError> {
    std::fs::write(path, bytes).map_err(|e| ExportError::Io(e.to_string()))?;
    Ok(bytes.len() as u64)
}
