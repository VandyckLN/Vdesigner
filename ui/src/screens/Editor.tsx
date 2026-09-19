import { useCallback, useEffect, useRef, useState } from "react";
import { api, buildJob, type EditorSettings, type ImageInfo } from "../api";
import { PreviewPane } from "../components/PreviewPane";
import { ResizeControls } from "../components/ResizeControls";
import { QualityControls } from "../components/QualityControls";
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

    timer.current = window.setTimeout(async () => {
      setBusy(true);
      try {
        const result = await api.preview(buildJob(settings));
        setPreviewSrc(`data:image/png;base64,${result.png_base64}`);
        setError(null);
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    }, PREVIEW_DEBOUNCE_MS);

    return () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    };
  }, [settings, image]);

  useEffect(() => {
    if (!image) return;
    if (estimateTimer.current !== null) window.clearTimeout(estimateTimer.current);

    estimateTimer.current = window.setTimeout(async () => {
      try {
        const result = await api.estimate(buildJob(settings));
        setEstimatedBytes(result.bytes);
      } catch {
        // A failed estimate is not worth an error banner: the number simply
        // disappears until the next change produces one.
        setEstimatedBytes(null);
      }
    }, ESTIMATE_DEBOUNCE_MS);

    return () => {
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
            processando…
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
            <QualityControls settings={settings} onChange={setSettings} />
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
              onPickDirectory={onPickDirectory}
              onError={setError}
            />
          </aside>
        </div>
      )}
    </main>
  );
}
