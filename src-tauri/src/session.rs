use std::sync::Mutex;

/// The bytes of the image currently open in the Editor. Kept in memory so the
/// preview never re-reads the disk on every slider move.
#[derive(Default)]
pub struct Session {
    pub source: Mutex<Option<SourceImage>>,
}

pub struct SourceImage {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub file_stem: String,
}
