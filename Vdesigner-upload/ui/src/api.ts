import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type OutputFormat = "WebP" | "Avif" | "Png" | "Jpeg" | "Tiff" | "Ico";
export type FitMode = "Contain" | "Cover" | "Stretch";

/** Mirrors vdesigner_core::Step, which serde serialises as an externally tagged enum. */
export type Step =
  | { Resize: { width: number | null; height: number | null; fit: FitMode; pad_color: [number, number, number, number] } }
  | { Sharpen: { amount: number; radius: number } }
  | { Denoise: { radius: number } }
  | { Adjust: { brightness: number; contrast: number; saturation: number } };

export interface Job {
  steps: Step[];
  output: { format: OutputFormat; quality: number; lossless: boolean };
}

export interface EditorSettings {
  width: number | null;
  height: number | null;
  lockRatio: boolean;
  fit: FitMode;
  sharpenAmount: number;
  denoiseRadius: number;
  brightness: number;
  contrast: number;
  saturation: number;
  format: OutputFormat;
  quality: number;
}

const LOSSLESS_FORMATS: OutputFormat[] = ["Png", "Tiff", "Ico"];

/** Turns the editor controls into the engine's job description. */
export function buildJob(settings: EditorSettings): Job {
  const steps: Step[] = [];

  if (settings.width !== null || settings.height !== null) {
    steps.push({
      Resize: {
        width: settings.width,
        // With the ratio locked, sending only one dimension lets the engine
        // derive the other, which is what keeps the image undistorted.
        height: settings.lockRatio ? null : settings.height,
        fit: settings.fit,
        pad_color: [0, 0, 0, 0],
      },
    });
  }

  if (settings.denoiseRadius > 0) {
    steps.push({ Denoise: { radius: settings.denoiseRadius } });
  }
  if (settings.sharpenAmount > 0) {
    steps.push({ Sharpen: { amount: settings.sharpenAmount, radius: 1.0 } });
  }
  if (settings.brightness !== 0 || settings.contrast !== 0 || settings.saturation !== 0) {
    steps.push({
      Adjust: {
        brightness: settings.brightness,
        contrast: settings.contrast,
        saturation: settings.saturation,
      },
    });
  }

  return {
    steps,
    output: {
      format: settings.format,
      quality: settings.quality,
      lossless: LOSSLESS_FORMATS.includes(settings.format),
    },
  };
}

export interface ImageInfo {
  width: number;
  height: number;
  file_stem: string;
  preview_png_base64: string;
}

export interface PreviewResult {
  png_base64: string;
  width: number;
  height: number;
}

export interface ExportResult {
  path: string;
  bytes_written: number;
}

/** Approximate size of the file the current settings would write. */
export interface EstimateResult {
  bytes: number;
}

/** One stage of a running export, as the engine reports it. */
export interface ExportProgress {
  current: number;
  total: number;
  label: string;
}

const EXPORT_PROGRESS_EVENT = "export-progress";

export type ColorFormat = "Hex" | "Rgb" | "Hsl" | "Oklch";
export type Generated = "css" | "tailwind";

export interface Swatch {
  nome: string;
  hex: string;
  rampa: boolean;
}

export interface GradientRef {
  nome: string;
  de: string;
  para: string;
}

export interface Palette {
  versao: number;
  nome: string;
  gerar: Generated[];
  cores: Swatch[];
  degrades: GradientRef[];
}

export interface PaletteSnapshot {
  palette: Palette;
  on_disk: string;
}

export interface Variations {
  ramp: string[];
  complementary: string;
  analogous: string[];
  triad: string[];
}

/** Rungs of the generated scale, mirroring vdesigner_core::RAMP_STEPS. */
export const RAMP_STEPS = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900] as const;

/** Must track vdesigner_core::PALETTE_FORMAT_VERSION (crates/core/src/palette.rs).
 *  The Rust side owns the on-disk format and refuses a palette whose `versao`
 *  is newer than its own, so the number the UI writes is not free-form: it is
 *  pinned here, next to the other shared constants, instead of being spelled
 *  out inline where a bump on the Rust side would silently leave it behind. */
export const PALETTE_FORMAT_VERSION = 1;

export interface MonitorInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
}

export interface OverlayGeometry {
  origin_x: number;
  origin_y: number;
  width: number;
  height: number;
  png_base64: string;
  monitors: MonitorInfo[];
}

const COLOR_PICKED_EVENT = "color-picked";
const SHORTCUT_UNAVAILABLE_EVENT = "shortcut-unavailable";

export const api = {
  openImage: (path: string) => invoke<ImageInfo>("open_image", { path }),
  preview: (job: Job) => invoke<PreviewResult>("preview", { job }),
  estimate: (job: Job) => invoke<EstimateResult>("estimate", { job }),
  export: (job: Job, outputDir: string, fileStem: string, overwrite: boolean) =>
    invoke<ExportResult>("export", { job, outputDir, fileStem, overwrite }),
  onExportProgress: (handler: (progress: ExportProgress) => void): Promise<UnlistenFn> =>
    listen<ExportProgress>(EXPORT_PROGRESS_EVENT, (event) => handler(event.payload)),
  loadPalette: (dir: string) => invoke<PaletteSnapshot | null>("load_palette", { dir }),
  savePalette: (dir: string, palette: Palette, expected: string | null) =>
    invoke<string>("save_palette", { dir, palette, expected }),
  colorVariations: (hex: string) => invoke<Variations>("color_variations", { hex }),
  formatColor: (hex: string, format: ColorFormat) =>
    invoke<string>("format_color", { hex, format }),
  startPick: () => invoke<OverlayGeometry>("start_pick"),
  pickAt: (x: number, y: number) => invoke<string>("pick_at", { x, y }),
  cancelPick: () => invoke<void>("cancel_pick"),
  onColorPicked: (handler: (hex: string) => void): Promise<UnlistenFn> =>
    listen<{ hex: string }>(COLOR_PICKED_EVENT, (event) => handler(event.payload.hex)),
  onShortcutUnavailable: (handler: (atalho: string) => void): Promise<UnlistenFn> =>
    listen<{ atalho: string }>(SHORTCUT_UNAVAILABLE_EVENT, (event) => handler(event.payload.atalho)),
  gradientCss: (palette: Palette, nome: string) => invoke<string>("gradient_css", { palette, nome }),
};
