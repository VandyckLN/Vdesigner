use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// The bytes of the image currently open in the Editor. Kept in memory so the
/// preview never re-reads the disk on every slider move.
#[derive(Default)]
pub struct Session {
    pub source: Mutex<Option<SourceImage>>,
    pub traces: TraceGeneration,
}

/// Numbers SVG previews so a newer one can stop an older one mid-trace.
#[derive(Default, Clone)]
pub struct TraceGeneration(Arc<AtomicU64>);

impl TraceGeneration {
    /// Registers a new trace. The returned check turns true once another begins.
    pub fn begin(&self) -> impl Fn() -> bool + Send + 'static {
        let mine = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        let counter = Arc::clone(&self.0);
        move || counter.load(Ordering::SeqCst) != mine
    }
}

pub struct SourceImage {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub file_stem: String,
}
