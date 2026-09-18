import { describe, expect, it } from "vitest";
import { buildJob, type EditorSettings } from "./api";

const base: EditorSettings = {
  width: null,
  height: null,
  lockRatio: true,
  fit: "Contain",
  sharpenAmount: 0,
  denoiseRadius: 0,
  brightness: 0,
  contrast: 0,
  saturation: 0,
  format: "WebP",
  quality: 82,
};

describe("buildJob", () => {
  it("produz um job sem etapas quando nada foi alterado", () => {
    const job = buildJob(base);
    expect(job.steps).toHaveLength(0);
    expect(job.output).toEqual({ format: "WebP", quality: 82, lossless: false });
  });

  it("inclui o resize quando há largura", () => {
    const job = buildJob({ ...base, width: 1920 });
    expect(job.steps).toEqual([
      { Resize: { width: 1920, height: null, fit: "Contain", pad_color: [0, 0, 0, 0] } },
    ]);
  });

  it("omite a altura enquanto a proporção está travada", () => {
    const job = buildJob({ ...base, width: 800, height: 600, lockRatio: true });
    expect(job.steps[0]).toEqual({
      Resize: { width: 800, height: null, fit: "Contain", pad_color: [0, 0, 0, 0] },
    });
  });

  it("envia as duas dimensões quando a proporção está destravada", () => {
    const job = buildJob({ ...base, width: 800, height: 600, lockRatio: false, fit: "Stretch" });
    expect(job.steps[0]).toEqual({
      Resize: { width: 800, height: 600, fit: "Stretch", pad_color: [0, 0, 0, 0] },
    });
  });

  it("ordena as etapas como denoise, sharpen e ajuste", () => {
    const job = buildJob({ ...base, sharpenAmount: 1, denoiseRadius: 2, brightness: 0.1 });
    expect(job.steps.map((s) => Object.keys(s)[0])).toEqual(["Denoise", "Sharpen", "Adjust"]);
  });

  it("marca png e tiff como lossless", () => {
    expect(buildJob({ ...base, format: "Png" }).output.lossless).toBe(true);
  });
});
