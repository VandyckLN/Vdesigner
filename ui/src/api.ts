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

export const api = {
  openImage: (path: string) => invoke<ImageInfo>("open_image", { path }),
  preview: (job: Job) => invoke<PreviewResult>("preview", { job }),
  estimate: (job: Job) => invoke<EstimateResult>("estimate", { job }),
  export: (job: Job, outputDir: string, fileStem: string, overwrite: boolean) =>
    invoke<ExportResult>("export", { job, outputDir, fileStem, overwrite }),
  onExportProgress: (handler: (progress: ExportProgress) => void): Promise<UnlistenFn> =>
    listen<ExportProgress>(EXPORT_PROGRESS_EVENT, (event) => handler(event.payload)),
};
