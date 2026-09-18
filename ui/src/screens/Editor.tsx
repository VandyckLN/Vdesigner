import { useCallback, useEffect, useRef, useState } from "react";
import { api, buildJob, type EditorSettings, type ImageInfo } from "../api";
import { PreviewPane } from "../components/PreviewPane";
import { ResizeControls } from "../components/ResizeControls";
import { QualityControls } from "../components/QualityControls";
import { ExportBar } from "../components/ExportBar";

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

interface EditorProps {
  /** Injected so tests do not need the Tauri dialog plugin. */
  onOpenFile: () => Promise<string | null>;
}

export function Editor({ onOpenFile }: EditorProps) {
  const [image, setImage] = useState<ImageInfo | null>(null);
  const [settings, setSettings] = useState<EditorSettings>(DEFAULT_SETTINGS);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const timer = useRef<number | null>(null);

  const open = useCallback(async () => {
    setError(null);
    const path = await onOpenFile();
    if (!path) return;
    try {
      const info = await api.openImage(path);
      setImage(info);
      setSettings(DEFAULT_SETTINGS);
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

  return (
    <main className="editor">
      <header className="editor__header">
        <button type="button" onClick={open}>
          Abrir imagem
        </button>
        {image && (
          <span className="editor__dimensions">
            {image.width} × {image.height}
          </span>
        )}
        {busy && <span role="status">processando…</span>}
      </header>

      {error && <p role="alert" className="editor__error">{error}</p>}

      {!image ? (
        <p className="editor__empty">Arraste uma imagem aqui ou clique em Abrir imagem.</p>
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
            <ExportBar settings={settings} fileStem={image.file_stem} onError={setError} />
          </aside>
        </div>
      )}
    </main>
  );
}
