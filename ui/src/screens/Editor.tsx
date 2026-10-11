import { useCallback, useEffect, useRef, useState } from "react";
import { api, buildJob, type EditorSettings, type ImageInfo } from "../api";
import { PreviewPane } from "../components/PreviewPane";
import { ResizeControls } from "../components/ResizeControls";
import { QualityControls } from "../components/QualityControls";
import { TraceControls } from "../components/TraceControls";
import { ExportBar } from "../components/ExportBar";
import { VdkMark } from "../components/VdkMark";

const DEFAULT_SETTINGS: EditorSettings = {
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
  traceMode: "Embed",
  traceColors: 8,
  removeBackground: true,
  traceDetail: "Normal",
};

/** Waiting this long after the last control change keeps the preview from
 *  running on every single slider pixel. */
const PREVIEW_DEBOUNCE_MS = 250;

/** The estimate runs the real output format, which AVIF makes slow. Its own,
 *  longer debounce keeps that off the preview's path entirely. */
const ESTIMATE_DEBOUNCE_MS = 500;

interface EditorProps {
  /** Injected so tests do not need the Tauri dialog plugin. */
  onOpenFile: () => Promise<string | null>;
  /** Injected so tests do not need the Tauri dialog plugin. */
  onPickDirectory: () => Promise<string | null>;
}

export function Editor({ onOpenFile, onPickDirectory }: EditorProps) {
  const [image, setImage] = useState<ImageInfo | null>(null);
  const [settings, setSettings] = useState<EditorSettings>(DEFAULT_SETTINGS);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Kept here, not inside ExportBar, so opening another image does not wipe a
  // destination the person already chose.
  const [outputDir, setOutputDir] = useState("");
  const [estimatedBytes, setEstimatedBytes] = useState<number | null>(null);
  const [svgCode, setSvgCode] = useState<string | null>(null);
  // Counts preview runs so a response that arrives after a newer run started
  // (or after the format changed) can be recognised and dropped.
  const previewRequest = useRef(0);
  const lastFormat = useRef(DEFAULT_SETTINGS.format);
  const timer = useRef<number | null>(null);
  const estimateTimer = useRef<number | null>(null);

  const open = useCallback(async () => {
    setError(null);
    try {
      const path = await onOpenFile();
      if (!path) return;
      const info = await api.openImage(path);
      setImage(info);
      setSettings(DEFAULT_SETTINGS);
      setEstimatedBytes(null);
      setPreviewSrc(`data:image/png;base64,${info.preview_png_base64}`);
    } catch (e) {
      setError(String(e));
    }
  }, [onOpenFile]);

  useEffect(() => {
    if (!image) return;
    if (timer.current !== null) window.clearTimeout(timer.current);

    const id = ++previewRequest.current;
    const isSvg = settings.format === "Svg";
    // A fresh run invalidates whatever SVG text was showing: copying must wait
    // for the preview that matches the current controls.
    setSvgCode(null);
    // An estimate (raster) and an exact size (SVG) are different things: never
    // let one be shown as the other after the format flips.
    if (lastFormat.current !== settings.format) {
      lastFormat.current = settings.format;
      setEstimatedBytes(null);
    }
    // The previous SVG's exact size must not linger while a new one is made.
    if (isSvg) setEstimatedBytes(null);

    timer.current = window.setTimeout(async () => {
      setBusy(true);
      try {
        if (isSvg) {
          const { svg } = await api.previewSvg(buildJob(settings));
          if (id !== previewRequest.current) return;
          setPreviewSrc(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
          setSvgCode(svg);
          setEstimatedBytes(new TextEncoder().encode(svg).length);
        } else {
          const result = await api.preview(buildJob(settings));
          if (id !== previewRequest.current) return;
          setPreviewSrc(`data:image/png;base64,${result.png_base64}`);
        }
        setError(null);
      } catch (e) {
        // A superseded run's failure (including the backend's "cancelada")
        // belongs to a request nobody is waiting for any more.
        if (id !== previewRequest.current) return;
        if (isSvg) setEstimatedBytes(null);
        setError(String(e));
      } finally {
        if (id === previewRequest.current) setBusy(false);
      }
    }, PREVIEW_DEBOUNCE_MS);

    return () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    };
  }, [settings, image]);

  useEffect(() => {
    if (!image || settings.format === "Svg") return;
    if (estimateTimer.current !== null) window.clearTimeout(estimateTimer.current);

    let stale = false;
    estimateTimer.current = window.setTimeout(async () => {
      try {
        const result = await api.estimate(buildJob(settings));
        if (!stale) setEstimatedBytes(result.bytes);
      } catch {
        if (stale) return;
        // A failed estimate is not worth an error banner: the number simply
        // disappears until the next change produces one.
        setEstimatedBytes(null);
      }
    }, ESTIMATE_DEBOUNCE_MS);

    return () => {
      stale = true;
      if (estimateTimer.current !== null) window.clearTimeout(estimateTimer.current);
    };
  }, [settings, image]);

  return (
    <main className="editor">
      <header className="editor__header">
        <span className="editor__brand">
          <VdkMark />
          <span className="editor__brand-name">Vdesigner</span>
        </span>
        <button type="button" onClick={open}>
          Abrir imagem
        </button>
        {image && (
          <span className="editor__dimensions mono">
            {image.width} × {image.height}
          </span>
        )}
        {busy && (
          <span role="status" className="mono">
            {settings.format === "Svg" && settings.traceMode !== "Embed" ? "vetorizando…" : "processando…"}
          </span>
        )}
      </header>

      {error && <p role="alert" className="editor__error">{error}</p>}

      {!image ? (
        <p className="editor__empty">Clique em Abrir imagem.</p>
      ) : (
        <div className="editor__body">
          <PreviewPane src={previewSrc} />
          <aside className="editor__panel">
            <ResizeControls
              settings={settings}
              sourceWidth={image.width}
              sourceHeight={image.height}
              onChange={setSettings}
            />
            <QualityControls settings={settings} onChange={setSettings} isVector={image.is_vector} />
            {settings.format === "Svg" && (
              <TraceControls
                settings={settings}
                onChange={setSettings}
                svgCode={svgCode}
                onError={setError}
              />
            )}
            {/* Remounting on a new image resets the export name and the
                overwrite flag, which would otherwise carry over from the
                previously opened file. The output folder is deliberately
                exempt: it lives in this component and survives the remount. */}
            <ExportBar
              key={image.file_stem}
              settings={settings}
              fileStem={image.file_stem}
              outputDir={outputDir}
              onOutputDirChange={setOutputDir}
              estimatedBytes={estimatedBytes}
              sizeIsExact={settings.format === "Svg"}
              onPickDirectory={onPickDirectory}
              onError={setError}
            />
          </aside>
        </div>
      )}
    </main>
  );
}
