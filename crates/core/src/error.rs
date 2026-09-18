use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("formato de arquivo não suportado")]
    UnsupportedFormat,

    #[error("falha ao decodificar a imagem: {0}")]
    Decode(String),

    #[error("falha ao codificar a imagem: {0}")]
    Encode(String),

    #[error("parâmetro inválido: {0}")]
    InvalidParameter(String),
}
