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

  // Each mode shows only what it reads: Foto original keeps the picture as it
  // is, so it has nothing here but the warning; its Compressão sits with the
  // output format.
  const modeControls = () => {
    switch (settings.traceMode) {
      case "Embed":
        return (
          <small>
            Guarda a própria foto dentro do SVG: fica igual à original e leve, mas não vira formas
            editáveis e pixela se você ampliar além da resolução original. A Compressão acima vale
            para essa foto. Com transparência, ela vai em PNG: sem perda, mais pesada, e aí a
            Compressão não muda nada.
          </small>
        );
      case "Photo":
        return (
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
            <small>Vira formas editáveis, mas com cara de pintura, e o arquivo sai pesado.</small>
          </>
        );
      case "Color":
        return (
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
        );
      case "Mono":
        return <small>Sai em currentColor: no site, o desenho herda a cor do texto.</small>;
    }
  };

  return (
    <fieldset className="controls">
      <legend>SVG</legend>

      <label htmlFor="trace-mode">Modo</label>
      <select
        id="trace-mode"
        value={settings.traceMode}
        onChange={(e) => onChange({ ...settings, traceMode: e.target.value as TraceMode })}
      >
        <option value="Embed">Foto original</option>
        <option value="Photo">Foto em vetor</option>
        <option value="Color">Cores (logo, ilustração)</option>
        <option value="Mono">Uma cor (ícone, assinatura)</option>
      </select>

      {modeControls()}

      <button type="button" onClick={copy} disabled={svgCode === null}>
        Copiar código SVG
      </button>
      {copied && <p role="status" className="mono">Código SVG copiado.</p>}
    </fieldset>
  );
}
