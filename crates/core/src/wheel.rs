//! What the radial menu offers for a set of files picked outside the app.
//!
//! The engine never learns where the selection came from. File Explorer today,
//! a tray menu or a drag gesture later: all of them hand this module a list of
//! paths and get back the same answer. That is the same bargain `picker.rs`
//! makes on the platform side, and it is what keeps a second trigger from
//! turning into a rewrite.
//!
//! **Extensions are triage, not identification.** [`decode::detect_format`]
//! reads the leading bytes and is the only thing in the project allowed to say
//! what a file *is*. This module looks at extensions instead, on purpose: the
//! wheel has to appear the instant the shortcut fires, and opening every
//! selected file to sniff it would mean touching a hundred files — possibly on
//! a network share — before drawing anything. So a file accepted here is a
//! *candidate*. The conversion that follows still decodes it properly, and a
//! `.png` that is really a text file still fails there, as it should. The cost
//! of the trade is bounded and visible: the wheel may offer a target that one
//! file in the batch then refuses.
//!
//! [`decode::detect_format`]: crate::detect_format

use crate::encode::OutputFormat;

/// Upper bound on the files one wheel acts upon.
///
/// Not a technical limit — the engine would happily take more. It exists
/// because the person is one keystroke away from starting this, and a slip
/// that selects a whole photo library should cost a visible message rather
/// than an export that runs for an hour. The remainder is reported as
/// [`Selection::truncated`], never silently dropped.
pub const MAX_SELECTION: usize = 100;

/// Raster extensions the engine decodes. Mirrors the arms of
/// [`detect_format`](crate::detect_format); SVG is absent because it is not a
/// raster format and takes a different path through the engine.
const RASTER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"];

/// How a candidate file enters the engine. Not an output format and not
/// [`InputFormat`](crate::InputFormat): this is the coarse split that decides
/// which targets the wheel may offer, which is all the wheel needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SelectionKind {
    /// Decoded by the `image` crate, then resized, filtered and re-encoded.
    Raster,
    /// Rasterised by `resvg` before anything else touches it.
    Svg,
}

/// A file the wheel is willing to work on, with the path exactly as the
/// platform reported it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SelectedFile {
    pub path: String,
    pub kind: SelectionKind,
}

/// The triaged selection: what the wheel will act on, and everything it is
/// leaving behind.
///
/// `skipped` and `truncated` are carried all the way to the interface on
/// purpose. A wheel that quietly converts four of the seven files the person
/// selected is worse than one that says it is converting four.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    pub files: Vec<SelectedFile>,
    /// Paths dropped because the engine has no decoder for them, in the order
    /// they arrived.
    pub skipped: Vec<String>,
    /// Set when the selection exceeded [`MAX_SELECTION`] and the tail was cut.
    pub truncated: bool,
}

impl Selection {
    /// True when there is nothing to draw a wheel around.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Lowercased extension, or an empty string when the name has none.
///
/// Deliberately hand-rolled rather than taken from `std::path`: a path that
/// arrives from COM is a platform string that may not be valid Unicode, and
/// the engine is forbidden from knowing about Windows. Splitting on the last
/// `.` of the last path component is all the triage above needs, and it
/// behaves the same on every platform, which is what makes it testable here.
fn extension_of(path: &str) -> String {
    let name = match path.rsplit_once(['/', '\\']) {
        Some((_, name)) => name,
        None => path,
    };
    // A leading dot makes a hidden file, not an extension: `.gitignore` has
    // no extension, while `.gitignore.png` has one.
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => extension.to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn kind_of(path: &str) -> Option<SelectionKind> {
    let extension = extension_of(path);
    if extension == "svg" {
        return Some(SelectionKind::Svg);
    }
    if RASTER_EXTENSIONS.contains(&extension.as_str()) {
        return Some(SelectionKind::Raster);
    }
    None
}

/// Triages raw paths into a [`Selection`].
///
/// Order is preserved so the interface can show the files the way the file
/// manager showed them. Duplicates are collapsed because a selection can name
/// the same file twice — the shell reports what is highlighted, and an
/// exported file landing twice in the same run would race itself for the
/// output name.
pub fn classify_selection<S: AsRef<str>>(paths: &[S]) -> Selection {
    let mut files: Vec<SelectedFile> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut truncated = false;

    for path in paths {
        let path = path.as_ref();
        if path.is_empty() {
            continue;
        }
        let Some(kind) = kind_of(path) else {
            if !skipped.iter().any(|seen| seen == path) {
                skipped.push(path.to_string());
            }
            continue;
        };
        if files.iter().any(|seen| seen.path == path) {
            continue;
        }
        // Checked after the duplicate test so a selection of 120 entries that
        // is really 80 distinct files is not reported as truncated.
        if files.len() == MAX_SELECTION {
            truncated = true;
            break;
        }
        files.push(SelectedFile {
            path: path.to_string(),
            kind,
        });
    }

    Selection {
        files,
        skipped,
        truncated,
    }
}

/// The targets the wheel lays out, in order.
///
/// Raster first because they are the common case; SVG last because it is the
/// one target that changes what kind of thing the file is.
///
/// **[`OutputFormat::Ico`] is deliberately absent**, even though the engine
/// encodes it. ICO stores at most 256 pixels per side, and this module decides
/// what to offer from paths alone — it cannot know a file's dimensions without
/// decoding it, which is exactly the cost the wheel exists to avoid. Offering
/// ICO here would mean a target that fails on nearly every real photo after
/// the person committed the whole batch to it. An icon is made in the Editor,
/// where the size is on screen and resizing to fit comes first.
const WHEEL_ORDER: &[OutputFormat] = &[
    OutputFormat::WebP,
    OutputFormat::Avif,
    OutputFormat::Png,
    OutputFormat::Jpeg,
    OutputFormat::Tiff,
    OutputFormat::Svg,
];

/// Targets a single kind of input can reach.
fn targets_for(kind: SelectionKind) -> &'static [OutputFormat] {
    match kind {
        // Svg is reachable from a raster through `trace`/`embed`.
        SelectionKind::Raster => WHEEL_ORDER,
        // Vector in, bitmap out. SVG to SVG is excluded because the engine
        // would rasterise and re-trace the file, returning something strictly
        // worse than the input — a conversion nobody wants offered to them.
        SelectionKind::Svg => &WHEEL_ORDER[..WHEEL_ORDER.len() - 1],
    }
}

/// What the wheel offers for a whole selection: the targets every file in it
/// can reach.
///
/// The intersection, not the union. A wheel built from the union would show
/// SVG for a selection of one logo and one photo, and then half the batch
/// would fail after the person committed to it. Offering less is the honest
/// read of a mixed selection.
///
/// Returns an empty vector for an empty selection — there is no wheel to draw.
pub fn wheel_targets(files: &[SelectedFile]) -> Vec<OutputFormat> {
    if files.is_empty() {
        return Vec::new();
    }
    WHEEL_ORDER
        .iter()
        .copied()
        .filter(|target| {
            files
                .iter()
                .all(|file| targets_for(file.kind).contains(target))
        })
        .collect()
}

/// Marks a file the wheel produced, so a converted file is never mistaken for
/// the original it came from.
pub const CONVERTED_SUFFIX: &str = "convertido";

/// Name for the n-th attempt at writing a converted file, without extension.
///
/// `attempt` 0 is the plain name; later attempts are for when that name is
/// already taken. Originals and earlier outputs are never overwritten — the
/// wheel fires from a keystroke, so the person has not been shown a save
/// dialog and has not agreed to replace anything. The caller is what knows
/// whether a name is free; this only says what to try next.
pub fn output_stem(source_stem: &str, attempt: usize) -> String {
    if attempt == 0 {
        format!("{source_stem}-{CONVERTED_SUFFIX}")
    } else {
        format!("{source_stem}-{CONVERTED_SUFFIX}-{attempt}")
    }
}

/// Whether a chosen target is one the engine will accept for this file.
///
/// The wheel offers only valid targets, so this guards the boundary rather
/// than the interface: the command that receives a target from the webview
/// cannot assume the webview is the only thing that can call it.
pub fn accepts(kind: SelectionKind, target: OutputFormat) -> bool {
    targets_for(kind).contains(&target)
}
