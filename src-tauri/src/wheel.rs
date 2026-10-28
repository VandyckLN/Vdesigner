//! Holds the selection one wheel is acting on, for as long as its window is
//! open.
//!
//! Deliberately shaped like [`crate::picker::Picker`], and for the same
//! reason: the trigger is not this module's business. The global shortcut arms
//! it today; a tray item or a drag gesture would arm it the same way, and
//! neither would touch this file.
//!
//! Nothing here decides anything about files. [`vdesigner_core::wheel`] does
//! that, on every platform, under test. This is a mutex around the answer.

use std::sync::Mutex;
use thiserror::Error;
use vdesigner_core::{OutputFormat, SelectedFile, Selection};

pub const WHEEL_LABEL: &str = "wheel";

#[derive(Debug, Error)]
pub enum WheelError {
    #[error("não há seleção para converter")]
    NotArmed,

    #[error("este formato não serve para a seleção atual")]
    TargetNotOffered,
}

/// Everything the wheel window needs to draw itself.
///
/// `skipped` and `truncated` ride along so the window can say what it is not
/// converting. A wheel that silently drops three of the person's seven files
/// is a bug report waiting to happen.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WheelState {
    pub files: Vec<SelectedFile>,
    pub skipped: Vec<String>,
    pub truncated: bool,
    pub targets: Vec<OutputFormat>,
}

#[derive(Default)]
pub struct Wheel {
    state: Mutex<Option<WheelState>>,
}

impl Wheel {
    /// Stores a classified selection and returns what the window should draw.
    pub fn arm(&self, selection: Selection, targets: Vec<OutputFormat>) -> WheelState {
        let state = WheelState {
            files: selection.files,
            skipped: selection.skipped,
            truncated: selection.truncated,
            targets,
        };
        // A poisoned mutex means another thread panicked mid-wheel. The
        // selection is disposable, so recovering the guard beats propagating
        // a panic into the interface — the same call the picker makes.
        let mut slot = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *slot = Some(state.clone());
        state
    }

    pub fn current(&self) -> Option<WheelState> {
        let slot = self.state.lock().unwrap_or_else(|e| e.into_inner());
        slot.clone()
    }

    /// The files to convert, checked against the target the window asked for.
    ///
    /// The check is here rather than in the interface because a command is
    /// reachable by anything that can talk to the webview, not only by the
    /// wheel that was drawn. `accepts` is the core's own rule, so the
    /// boundary and the wheel cannot drift apart.
    pub fn take_for(&self, target: OutputFormat) -> Result<Vec<SelectedFile>, WheelError> {
        let slot = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let state = slot.as_ref().ok_or(WheelError::NotArmed)?;
        if !state.targets.contains(&target) {
            return Err(WheelError::TargetNotOffered);
        }
        Ok(state.files.clone())
    }

    pub fn disarm(&self) {
        let mut slot = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *slot = None;
    }
}
