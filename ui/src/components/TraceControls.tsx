import { useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { EditorSettings, TraceDetail, TraceMode } from "../api";

interface TraceControlsProps {
  settings: EditorSettings;
  onChange: (next: EditorSettings) => void;
  /** SVG text of the current preview, or null while a newer one is on its way. */
  svgCode: string | null;
  onError: (message: string | null) => void;
}

export function TraceControls({ settings, onChange, svgCode, onError }: TraceControlsProps) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    if (svgCode === null) return;
    onError(null);
    setCopied(false);
    try {
      await writeText(svgCode);
      setCopied(true);
    } catch (e) {
      onError("Não foi possível copiar o código SVG: " + (e instanceof Error ? e.message : String(e)));
    }
  };

  const color = settings.traceMode === "Color";

  return (
    <fieldset className="controls">
      <legend>Vetorização</legend>

      <label htmlFor="trace-mode">Modo</label>
      <select
        id="trace-mode"
        value={settings.traceMode}
        onChange={(e) => onChange({ ...settings, traceMode: e.target.value as TraceMode })}
      >
        <option value="Photo">Foto (fiel)</option>
        <option value="Color">Cores (logo, ilustração)</option>
        <option value="Mono">Uma cor (ícone, assinatura)</option>
      </select>

      {settings.traceMode === "Photo" ? (
        <>
          <label htmlFor="trace-detail">Detalhe</label>
          <select
            id="trace-detail"
            value={settings.traceDetail}
            onChange={(e) => onChange({ ...settings, traceDetail: e.target.value as TraceDetail })}
          >
            <option value="Normal">Normal</option>
            <option value="High">Alto</option>
            <option value="Max">Máximo</option>
          </select>
          <small>Fica fiel à foto, mas o arquivo sai pesado.</small>
        </>
      ) : color ? (
        <>
          <label htmlFor="trace-colors">
            Cores <span className="mono">{settings.traceColors}</span>
          </label>
          <input
            id="trace-colors"
            type="range"
            min={2}
            max={16}
            step={1}
            value={settings.traceColors}
            onChange={(e) => onChange({ ...settings, traceColors: Number(e.target.value) })}
          />

          <label htmlFor="remove-background">
            <input
              id="remove-background"
              type="checkbox"
              checked={settings.removeBackground}
              onChange={(e) => onChange({ ...settings, removeBackground: e.target.checked })}
            />
            Remover fundo
          </label>
          {settings.removeBackground && (
            <small>A cor da borda vira transparente em toda a imagem, inclusive dentro das letras.</small>
          )}
        </>
      ) : (
        <small>Sai em currentColor: no site, o desenho herda a cor do texto.</small>
      )}

      <button type="button" onClick={copy} disabled={svgCode === null}>
        Copiar código SVG
      </button>
      {copied && <p role="status" className="mono">Código SVG copiado.</p>}
    </fieldset>
  );
}
