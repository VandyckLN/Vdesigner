import { useState } from "react";
import { api, buildJob, type EditorSettings } from "../api";

interface ExportBarProps {
  settings: EditorSettings;
  fileStem: string;
  onError: (message: string | null) => void;
}

export function ExportBar({ settings, fileStem, onError }: ExportBarProps) {
  const [outputDir, setOutputDir] = useState("");
  const [stem, setStem] = useState(fileStem);
  const [overwrite, setOverwrite] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  const run = async () => {
    onError(null);
    setSaved(null);
    try {
      const result = await api.export(buildJob(settings), outputDir, stem, overwrite);
      setSaved(result.path);
    } catch (e) {
      onError(String(e));
    }
  };

  return (
    <fieldset className="controls">
      <legend>Exportar</legend>

      <label htmlFor="outdir">Pasta de saída</label>
      <input id="outdir" value={outputDir} onChange={(e) => setOutputDir(e.target.value)} />

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

      <button type="button" onClick={run} disabled={!outputDir || !stem}>
        Exportar
      </button>

      {saved && <p role="status">Salvo em {saved}</p>}
    </fieldset>
  );
}
