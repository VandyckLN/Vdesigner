import { useEffect, useRef, useState } from "react";
import { api, buildJob, type EditorSettings, type ExportProgress } from "../api";
import { formatBytes } from "../format";

interface ExportBarProps {
  settings: EditorSettings;
  fileStem: string;
  /** Owned by the Editor so it survives opening another image. */
  outputDir: string;
  onOutputDirChange: (dir: string) => void;
  /** Approximate size of the file this job would write, or null while unknown. */
  estimatedBytes: number | null;
  onPickDirectory: () => Promise<string | null>;
  onError: (message: string | null) => void;
}

export function ExportBar({
  settings,
  fileStem,
  outputDir,
  onOutputDirChange,
  estimatedBytes,
  onPickDirectory,
  onError,
}: ExportBarProps) {
  const [stem, setStem] = useState(fileStem);
  const [overwrite, setOverwrite] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);
  const [progress, setProgress] = useState<ExportProgress | null>(null);
  const [busy, setBusy] = useState(false);
  // The subscription is created once, so it reads the live flag through a ref
  // rather than closing over a stale `busy`.
  const running = useRef(false);

  // The engine reports each stage as it finishes. Subscribing for the life of
  // the panel rather than per export keeps a slow subscription from missing
  // the first stages of a fast job.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    api
      .onExportProgress((next) => {
        if (running.current) setProgress(next);
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // No progress channel just means no stage line; the export still runs.
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const run = async () => {
    onError(null);
    setSaved(null);
    setProgress(null);
    running.current = true;
    setBusy(true);
    try {
      const result = await api.export(buildJob(settings), outputDir, stem, overwrite);
      setSaved(result.path);
    } catch (e) {
      onError(String(e));
    } finally {
      running.current = false;
      setBusy(false);
      setProgress(null);
    }
  };

  const pickDirectory = async () => {
    onError(null);
    const selected = await onPickDirectory();
    if (selected) onOutputDirChange(selected);
  };

  return (
    <fieldset className="controls">
      <legend>Exportar</legend>

      <label htmlFor="outdir">Pasta de saída</label>
      <div className="controls__row">
        <input
          id="outdir"
          value={outputDir}
          onChange={(e) => onOutputDirChange(e.target.value)}
        />
        <button type="button" onClick={pickDirectory}>
          Escolher pasta
        </button>
      </div>

      <label htmlFor="stem">Nome do arquivo</label>
      <input id="stem" value={stem} onChange={(e) => setStem(e.target.value)} />

      <label htmlFor="overwrite">
        <input
          id="overwrite"
          type="checkbox"
          checked={overwrite}
          onChange={(e) => setOverwrite(e.target.checked)}
        />
        Sobrescrever se já existir
      </label>
      {overwrite && <small>O arquivo existente será substituído sem aviso.</small>}

      {estimatedBytes !== null && (
        <p className="mono">Tamanho estimado ~{formatBytes(estimatedBytes)}</p>
      )}

      <button
        type="button"
        className="btn--primary"
        onClick={run}
        disabled={busy || !outputDir || !stem}
      >
        Exportar
      </button>

      {busy && (
        <p role="status" className="mono">
          {progress ? `${progress.current}/${progress.total} ${progress.label}` : "exportando…"}
        </p>
      )}

      {saved && (
        <p role="status" className="mono">
          Salvo em {saved}
        </p>
      )}
    </fieldset>
  );
}
