//! The only place in the project that knows what a screen is. Cursor-to-pixel
//! conversion with DPI scaling lives here and nowhere else: if it is wrong, it
//! is wrong in one place.
//!
//! **Coordinate space.** Everything below — `Snapshot::origin_x/origin_y`,
//! `MonitorInfo::x/y/width/height`, and the `vx`/`vy` taken by `pixel_at` —
//! is in the space `xcap` reports on Windows: physical pixels straight from
//! `DEVMODE` (`dmPosition`, `dmPelsWidth`/`dmPelsHeight`), unscaled by any
//! monitor's DPI factor.
//!
//! **How that space relates to Tauri's.** Tauri's window backend (`tao`)
//! calls `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` at startup.
//! In a per-monitor-v2 process Windows stops virtualising desktop
//! coordinates: the virtual screen, `GetCursorPos`, `MONITORINFO::rcMonitor`
//! and therefore every Tauri `PhysicalPosition`/`PhysicalSize` are in the
//! same unscaled physical pixels as `DEVMODE`. So **Tauri's physical space
//! and the space here are one space**, and no per-monitor scaling is needed
//! to move between them. What is *not* the same space is Tauri's **logical**
//! space: `WebviewWindowBuilder::position`/`inner_size` take logical pixels,
//! and CSS pixels inside the overlay's webview are logical too. Those need
//! the window's own DPI factor applied, which is what
//! [`overlay_point_to_snapshot`] does — and it is the only scaling
//! arithmetic in the project, deliberately.
//!
//! Note the consequence for callers: place and size the overlay with the
//! **physical** setters (`set_position`/`set_size` with
//! `PhysicalPosition`/`PhysicalSize`), never with the builder's logical
//! ones, and the snapshot's rectangle can be handed over unchanged.
//!
//! **Known capture limitation.** The capture path uses GDI `BitBlt`
//! (`xcap`'s default backend, not the Windows Graphics Capture API). BitBlt
//! cannot read hardware-overlay or protected surfaces — video played
//! through overlay, DRM-protected content, some GPU-accelerated video — and
//! returns black for those pixels instead of an error. The manual test
//! script (Task 7) needs to exercise the eyedropper over a playing video to
//! confirm this is an acceptable, visible limitation rather than a silent
//! wrong reading.

use base64::Engine;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use thiserror::Error;
use vdesigner_core::Color;

#[derive(Debug, Error)]
pub enum ScreenError {
    #[error("falha ao capturar a tela: {0}")]
    Capture(String),

    #[error("nenhum monitor foi encontrado")]
    NoMonitors,

    #[error("a posição está fora da área de trabalho")]
    OutOfBounds,

    #[error("falha ao codificar o retrato: {0}")]
    Encode(String),

    #[error("o buffer não corresponde ao tamanho declarado")]
    SizeMismatch,

    #[error("a posição informada não é um ponto válido")]
    InvalidPoint,
}

// `xcap::XCapError` covers every platform backend's failure in one type;
// mapping it here once means the call sites in `capture_all_monitors` don't
// each repeat their own `.map_err(...)`.
impl From<xcap::XCapError> for ScreenError {
    fn from(err: xcap::XCapError) -> Self {
        ScreenError::Capture(err.to_string())
    }
}

/// A monitor's position and size, in the DEVMODE-derived physical-pixel
/// space documented on the module — not the scaled Win32 virtual-screen
/// space. `scale` is the DPI factor Windows reports for that monitor.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

/// A frozen picture of every monitor, composed into a single RGBA bitmap
/// covering the bounding box of the virtual desktop. Everything here is in
/// physical pixels; logical pixels are the UI's problem, not this module's.
pub struct Snapshot {
    /// Top-left corner of the composed bitmap, in the same physical-pixel
    /// space as `MonitorInfo` (see the module doc comment). Can be negative:
    /// a monitor left of the primary has a negative `x` on Windows.
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub monitors: Vec<MonitorInfo>,
    pixels: Vec<u8>,
}

impl Snapshot {
    pub fn from_raw(
        origin_x: i32,
        origin_y: i32,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        monitors: Vec<MonitorInfo>,
    ) -> Result<Self, ScreenError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(ScreenError::SizeMismatch)?;
        if pixels.len() != expected {
            return Err(ScreenError::SizeMismatch);
        }
        Ok(Self {
            origin_x,
            origin_y,
            width,
            height,
            monitors,
            pixels,
        })
    }

    /// Takes **virtual** physical coordinates in the same DEVMODE-derived
    /// space as `origin_x`/`origin_y` (see the module doc comment) — not
    /// the scaled Win32 virtual-screen space a raw cursor position arrives
    /// in. Negative values are expected for a monitor left of the primary.
    /// Subtracting the origin here, rather than at every call site, is what
    /// keeps the arithmetic in one place.
    pub fn pixel_at(&self, vx: i32, vy: i32) -> Result<Color, ScreenError> {
        let x = vx
            .checked_sub(self.origin_x)
            .ok_or(ScreenError::OutOfBounds)?;
        let y = vy
            .checked_sub(self.origin_y)
            .ok_or(ScreenError::OutOfBounds)?;
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return Err(ScreenError::OutOfBounds);
        }
        // `x`/`y` are already proven in `0..width`/`0..height` above, and
        // `pixels.len() == width * height * 4` is an invariant `from_raw`
        // enforces, so `index + 3` is always in bounds — direct indexing
        // (not `.get(..).ok_or(..)`) documents that this cannot fail rather
        // than pretending it can.
        let index = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        Ok(Color {
            r: self.pixels[index],
            g: self.pixels[index + 1],
            b: self.pixels[index + 2],
        })
    }

    /// Base64 of a PNG, with no `data:` prefix: the prefix is a detail of how
    /// the UI consumes it, and baking it in here would make the encoder lie
    /// about what it produces.
    pub fn to_png_base64(&self) -> Result<String, ScreenError> {
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(
                &self.pixels,
                self.width,
                self.height,
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| ScreenError::Encode(e.to_string()))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(png))
    }
}

/// Turns a point the overlay's webview reported — CSS pixels measured from
/// the overlay window's own top-left corner — into a point in the snapshot's
/// DEVMODE physical-pixel space, ready for [`Snapshot::pixel_at`].
///
/// `scale` is the overlay **window's** DPI factor (`Window::scale_factor()`),
/// not any single monitor's. That is the correct factor even when the overlay
/// spans monitors of different DPI: Windows gives a window one backing scale,
/// the whole webview surface is rendered at it, and the surface itself sits at
/// the window's physical origin inside the one physical virtual screen. A
/// per-monitor factor applied here would be wrong precisely on the mixed-DPI
/// setup it looks like it is protecting.
///
/// `floor`, not `round`: a click at CSS 10.9 with scale 1.0 is inside physical
/// pixel 10, and rounding it up would read the neighbour's colour.
///
/// A `NaN` or infinite coordinate is an error, not a point: silently
/// collapsing it to the origin would hand the user the top-left pixel of the
/// desktop and call it the colour they clicked on.
pub fn overlay_point_to_snapshot(
    origin_x: i32,
    origin_y: i32,
    css_x: f64,
    css_y: f64,
    scale: f64,
) -> Result<(i32, i32), ScreenError> {
    if !css_x.is_finite() || !css_y.is_finite() {
        return Err(ScreenError::InvalidPoint);
    }
    // A non-finite or non-positive factor, unlike a bad coordinate, is the
    // backend failing to report rather than a nonsensical click. Treating it
    // as 1:1 keeps the pick in the right ballpark; `pixel_at` still refuses
    // whatever falls outside the picture.
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let offset = |css: f64| -> i32 {
        // Finite times finite can still overflow to infinity, so clamp into
        // `i32` rather than relying on the `as` cast's saturation alone.
        // A small epsilon absorbs IEEE 754 floating-point representation error so that
        // coordinates originating from physical pixel division (e.g. at 175% DPI)
        // do not round down to p - 1 under floor().
        (css * scale + 1e-6)
            .floor()
            .clamp(i32::MIN as f64, i32::MAX as f64) as i32
    };
    Ok((
        origin_x.saturating_add(offset(css_x)),
        origin_y.saturating_add(offset(css_y)),
    ))
}

/// Composes captured monitor images into a single RGBA bitmap covering
/// their bounding box, returning `(origin_x, origin_y, width, height,
/// pixels)`. Pure arithmetic over already-captured images — no I/O, no
/// platform calls — which is what makes it testable without a real screen.
///
/// A gap between differently sized monitors stays opaque black
/// (`0, 0, 0, 255`), not transparent: the overlay draws this bitmap over
/// nothing, and transparent there would show the live screen underneath,
/// exactly what freezing the picture was meant to prevent.
fn compose(shots: &[(i32, i32, RgbaImage)]) -> Result<(i32, i32, u32, u32, Vec<u8>), ScreenError> {
    let origin_x = shots
        .iter()
        .map(|(x, _, _)| *x)
        .min()
        .ok_or(ScreenError::NoMonitors)?;
    let origin_y = shots
        .iter()
        .map(|(_, y, _)| *y)
        .min()
        .ok_or(ScreenError::NoMonitors)?;
    let right = shots
        .iter()
        .map(|(x, _, image)| x + image.width() as i32)
        .max()
        .ok_or(ScreenError::NoMonitors)?;
    let bottom = shots
        .iter()
        .map(|(_, y, image)| y + image.height() as i32)
        .max()
        .ok_or(ScreenError::NoMonitors)?;

    let width = (right - origin_x).max(0) as u32;
    let height = (bottom - origin_y).max(0) as u32;
    if width == 0 || height == 0 {
        return Err(ScreenError::NoMonitors);
    }

    // Opaque black, not transparent: see the doc comment above.
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    for index in (3..pixels.len()).step_by(4) {
        pixels[index] = 255;
    }

    for (x, y, image) in shots {
        let offset_x = (x - origin_x).max(0) as usize;
        let offset_y = (y - origin_y).max(0) as usize;
        for (px, py, pixel) in image.enumerate_pixels() {
            let dx = offset_x + px as usize;
            let dy = offset_y + py as usize;
            if dx >= width as usize || dy >= height as usize {
                continue;
            }
            let target = (dy * width as usize + dx) * 4;
            pixels[target] = pixel[0];
            pixels[target + 1] = pixel[1];
            pixels[target + 2] = pixel[2];
            pixels[target + 3] = 255;
        }
    }

    Ok((origin_x, origin_y, width, height, pixels))
}

/// Captures every monitor and composes them into one bitmap. Monitors are
/// blitted at their own position inside the virtual desktop; see `compose`
/// for the arithmetic and the opaque-black gap-fill rationale.
pub fn capture_all_monitors() -> Result<Snapshot, ScreenError> {
    let monitors = xcap::Monitor::all()?;
    if monitors.is_empty() {
        return Err(ScreenError::NoMonitors);
    }

    let mut infos = Vec::new();
    let mut shots = Vec::new();
    for monitor in &monitors {
        let x = monitor.x()?;
        let y = monitor.y()?;
        let image = monitor.capture_image()?;
        let scale = monitor.scale_factor()? as f64;
        infos.push(MonitorInfo {
            x,
            y,
            width: image.width(),
            height: image.height(),
            scale,
        });
        shots.push((x, y, image));
    }

    let (origin_x, origin_y, width, height, pixels) = compose(&shots)?;
    Snapshot::from_raw(origin_x, origin_y, width, height, pixels, infos)
}

#[cfg(test)]
mod coordinate_tests {
    use super::{overlay_point_to_snapshot, ScreenError};

    #[test]
    fn it_leaves_a_point_untouched_when_nothing_is_scaled() {
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 10.0, 20.0, 1.0).expect("ponto válido"),
            (10, 20),
            "sem escala a conversão deve ser a identidade"
        );
    }

    #[test]
    fn it_expands_a_point_by_the_window_factor_on_a_scaled_display() {
        // A 2560x1440 panel at 150%: the webview is 1707x960 CSS pixels, and
        // CSS (10, 20) is physical (15, 30).
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 10.0, 20.0, 1.5).expect("ponto válido"),
            (15, 30),
            "a 150% um pixel CSS vale um e meio pixel físico"
        );
    }

    #[test]
    fn it_keeps_a_click_inside_the_pixel_it_landed_on() {
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 10.9, 10.9, 1.0).expect("ponto válido"),
            (10, 10),
            "arredondar para cima leria a cor do pixel vizinho"
        );
    }

    #[test]
    fn it_resolves_a_point_on_the_second_of_two_differently_scaled_monitors() {
        // A 1920x1080 panel at 100% to the right of a 2560x1440 panel at
        // 150%. DEVMODE lays them out in physical pixels, so the second
        // starts at x = 2560. The overlay spans both and Windows gives it a
        // single factor — 1.5 here, the factor of the monitor it was born
        // on. CSS x = 1800 is physical 2700 — past the first monitor's 2560,
        // so 140 pixels into the second one.
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 1800.0, 100.0, 1.5).expect("ponto válido"),
            (2700, 150),
            "o ponto deve cair 140 pixels dentro do segundo monitor, não no primeiro"
        );
    }

    #[test]
    fn it_resolves_a_point_on_a_monitor_left_of_the_primary() {
        // A monitor at x = -1920 makes the snapshot origin negative; the
        // overlay's own CSS origin is still zero, so the origin has to be
        // added back or every pick on that screen reads the wrong monitor.
        let point = overlay_point_to_snapshot(-1920, 0, 10.0, 10.0, 1.0).expect("ponto válido");
        assert_eq!(point, (-1910, 10), "a origem negativa deve ser somada");
    }

    #[test]
    fn it_falls_back_to_one_to_one_when_the_backend_reports_no_factor() {
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 10.0, 10.0, 0.0).expect("ponto válido"),
            (10, 10),
            "um fator inválido deve virar 1:1, nunca uma coordenada NaN"
        );
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 10.0, 10.0, f64::NAN).expect("ponto válido"),
            (10, 10),
            "um fator NaN deve virar 1:1"
        );
    }

    #[test]
    fn it_refuses_nan_and_infinite_coordinates() {
        assert!(matches!(
            overlay_point_to_snapshot(0, 0, f64::NAN, 10.0, 1.0),
            Err(ScreenError::InvalidPoint)
        ));
        assert!(matches!(
            overlay_point_to_snapshot(0, 0, 10.0, f64::INFINITY, 1.0),
            Err(ScreenError::InvalidPoint)
        ));
        assert!(matches!(
            overlay_point_to_snapshot(0, 0, f64::NEG_INFINITY, 10.0, 1.0),
            Err(ScreenError::InvalidPoint)
        ));
    }

    #[test]
    fn it_resolves_fractional_scale_boundary_coordinates_without_underflow() {
        // At 175% DPI (scale 1.75), pixel 61 is represented as CSS 61.0 / 1.75.
        // Plain (css * 1.75) evaluates to 60.99999999999999 in IEEE 754,
        // which naive .floor() drops to 60. The epsilon must absorb this.
        assert_eq!(
            overlay_point_to_snapshot(0, 0, 61.0 / 1.75, 0.0, 1.75).expect("ponto válido"),
            (61, 0),
            "coordenadas CSS derivadas de divisão por escala fracionária não devem subestimar o pixel"
        );
    }
}

#[cfg(test)]
mod compose_tests {
    use super::compose;
    use image::RgbaImage;

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
        RgbaImage::from_fn(width, height, |_, _| image::Rgba(rgba))
    }

    #[test]
    fn it_places_a_negative_origin_monitor_at_the_left_edge_of_the_bitmap() {
        let shots = vec![(-2, 0, solid(2, 2, [255, 0, 0, 255]))];
        let (origin_x, origin_y, width, height, pixels) = compose(&shots).expect("deve compor");
        assert_eq!((origin_x, origin_y, width, height), (-2, 0, 2, 2));
        assert_eq!(
            &pixels[0..4],
            [255, 0, 0, 255],
            "o primeiro pixel deve ser vermelho"
        );
    }

    #[test]
    fn it_places_two_side_by_side_monitors_without_overlap() {
        let shots = vec![
            (0, 0, solid(2, 2, [255, 0, 0, 255])),
            (2, 0, solid(2, 2, [0, 0, 255, 255])),
        ];
        let (origin_x, origin_y, width, height, pixels) = compose(&shots).expect("deve compor");
        assert_eq!((origin_x, origin_y, width, height), (0, 0, 4, 2));
        // Row 0: red, red, blue, blue.
        assert_eq!(
            &pixels[0..4],
            [255, 0, 0, 255],
            "coluna 0 deve ser vermelha"
        );
        assert_eq!(
            &pixels[4..8],
            [255, 0, 0, 255],
            "coluna 1 deve ser vermelha"
        );
        assert_eq!(&pixels[8..12], [0, 0, 255, 255], "coluna 2 deve ser azul");
        assert_eq!(&pixels[12..16], [0, 0, 255, 255], "coluna 3 deve ser azul");
    }

    #[test]
    fn it_fills_a_gap_between_differently_sized_monitors_with_opaque_black() {
        // A tall 2x4 monitor at x=0 and a short 2x2 monitor at x=2: the
        // bounding box is 4 wide, 4 tall, and the bottom-right 2x2 corner
        // under the short monitor is unpainted — it must come out opaque
        // black, never transparent.
        let shots = vec![
            (0, 0, solid(2, 4, [255, 0, 0, 255])),
            (2, 0, solid(2, 2, [0, 0, 255, 255])),
        ];
        let (_, _, width, height, pixels) = compose(&shots).expect("deve compor");
        assert_eq!((width, height), (4, 4));
        // Bottom-right corner (x=3, y=3) falls in the gap.
        let index = ((3 * width as usize) + 3) * 4;
        assert_eq!(
            &pixels[index..index + 4],
            [0, 0, 0, 255],
            "o buraco entre monitores deve ficar preto opaco, nunca transparente"
        );
    }
}
