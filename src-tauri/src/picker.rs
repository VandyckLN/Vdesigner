//! Trigger-agnostic picking service. It does not know whether it was started
//! by the global shortcut, by the button on the Colors screen, or — later —
//! by a tray menu item. Keeping it ignorant is what lets the tray arrive
//! without a rewrite.
//!
//! **Coordinate space.** Every coordinate here — `OverlayGeometry`'s origin
//! and size, and `resolve`'s `vx`/`vy` — is passed straight through to
//! `screen::Snapshot`, so it lives in the DEVMODE-derived physical-pixel
//! space that module documents. This module deliberately does no scaling
//! arithmetic of its own: the cursor → pixel conversion belongs in
//! `screen.rs` and nowhere else.

use crate::screen::{overlay_point_to_snapshot, MonitorInfo, ScreenError, Snapshot};
use std::sync::Mutex;
use thiserror::Error;
use vdesigner_core::{format, ColorFormat};

pub const OVERLAY_LABEL: &str = "overlay";

#[derive(Debug, Error)]
pub enum PickerError {
    #[error("não há retrato de tela para escolher")]
    NotArmed,

    #[error(transparent)]
    Screen(#[from] ScreenError),
}

/// Everything the overlay window needs to draw itself and to translate a
/// cursor position back into a pixel of the frozen picture.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OverlayGeometry {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub png_base64: String,
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Default)]
pub struct Picker {
    snapshot: Mutex<Option<(Snapshot, OverlayGeometry)>>,
}

impl Picker {
    pub fn arm(&self, snapshot: Snapshot) -> Result<OverlayGeometry, PickerError> {
        let geometry = OverlayGeometry {
            origin_x: snapshot.origin_x,
            origin_y: snapshot.origin_y,
            width: snapshot.width,
            height: snapshot.height,
            png_base64: snapshot.to_png_base64()?,
            monitors: snapshot.monitors.clone(),
        };
        // A poisoned mutex here means another thread panicked mid-pick. The
        // snapshot is disposable, so recovering the guard is strictly better
        // than propagating a panic into the UI.
        let mut slot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        *slot = Some((snapshot, geometry.clone()));
        Ok(geometry)
    }

    /// Returns the geometry of the active snapshot if armed.
    pub fn current_geometry(&self) -> Option<OverlayGeometry> {
        let slot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        slot.as_ref().map(|(_, g)| g.clone())
    }

    /// Virtual physical coordinates, same space as `origin_x`/`origin_y`.
    pub fn resolve(&self, vx: i32, vy: i32) -> Result<String, PickerError> {
        let slot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        let (snapshot, _) = slot.as_ref().ok_or(PickerError::NotArmed)?;
        hex_at(snapshot, vx, vy)
    }

    /// Resolves a point as the overlay's webview reports it — CSS pixels
    /// from the overlay window's top-left corner — together with the overlay
    /// window's DPI factor. The conversion itself lives in `screen.rs`; this
    /// only supplies the snapshot's origin, which the overlay does not know.
    ///
    /// The lock is held from reading the origin to reading the pixel. Taking
    /// it twice left a window in which a new `arm` could swap the snapshot,
    /// so the origin of one picture would be applied to the pixels of
    /// another.
    pub fn resolve_overlay_point(
        &self,
        css_x: f64,
        css_y: f64,
        scale: f64,
    ) -> Result<String, PickerError> {
        let slot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        let (snapshot, _) = slot.as_ref().ok_or(PickerError::NotArmed)?;
        let (vx, vy) =
            overlay_point_to_snapshot(snapshot.origin_x, snapshot.origin_y, css_x, css_y, scale)?;
        hex_at(snapshot, vx, vy)
    }

    /// Drops the snapshot. A two-monitor 4K picture is tens of megabytes;
    /// keeping it alive after the overlay closes is a leak the user can see
    /// in Task Manager.
    pub fn disarm(&self) {
        let mut slot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        *slot = None;
    }
}

fn hex_at(snapshot: &Snapshot, vx: i32, vy: i32) -> Result<String, PickerError> {
    let color = snapshot.pixel_at(vx, vy)?;
    Ok(format(color, ColorFormat::Hex))
}
