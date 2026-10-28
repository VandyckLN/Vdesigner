use std::path::{Path, PathBuf};
use thiserror::Error;
use vdesigner_core::output_stem;

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

/// Number of names the batch is willing to try before giving up on one file.
///
/// Reached only when the same file has already been converted this many times
/// into the same folder, which means something is wrong with the loop rather
/// than with the file.
const MAX_BATCH_ATTEMPTS: usize = 1000;

/// Destination for a file converted by the wheel, next to its original.
///
/// Walks [`output_stem`] until it finds a name nothing occupies. The wheel
/// fires from a keystroke, so no save dialog was shown and nothing may be
/// replaced: not the original, and not an output from an earlier pass.
///
/// The race this cannot close on its own is two passes picking the same free
/// name at the same moment; [`write_new`] is what closes it.
pub fn resolve_batch_output_path(
    directory: &Path,
    source_stem: &str,
    extension: &str,
) -> Result<PathBuf, ExportError> {
    for attempt in 0..MAX_BATCH_ATTEMPTS {
        let stem = output_stem(source_stem, attempt);
        match resolve_output_path(directory, &stem, extension, false) {
            Ok(path) => return Ok(path),
            Err(ExportError::WouldOverwrite(_)) => continue,
            Err(other) => return Err(other),
        }
    }
    Err(ExportError::Io(format!(
        "há arquivos demais com o nome {source_stem}"
    )))
}

/// Writes only if the path is still free, failing rather than replacing.
///
/// `resolve_batch_output_path` checks and this writes, and between the two a
/// concurrent pass can take the name. `create_new` makes the check and the
/// write one operation in the filesystem, so the loser of that race gets an
/// error instead of overwriting the winner's file.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<u64, ExportError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => {
                ExportError::WouldOverwrite(path.display().to_string())
            }
            _ => ExportError::Io(e.to_string()),
        })?;
    file.write_all(bytes)
        .map_err(|e| ExportError::Io(e.to_string()))?;
    Ok(bytes.len() as u64)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<u64, ExportError> {
    std::fs::write(path, bytes).map_err(|e| ExportError::Io(e.to_string()))?;
    Ok(bytes.len() as u64)
}
