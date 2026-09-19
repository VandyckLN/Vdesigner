import type { EditorSettings, FitMode } from "../api";

/** Mirrors vdesigner_core::MAX_SIDE. The engine is what actually enforces it;
 *  this only keeps the browser's own validation in step. */
const MAX_SIDE = 16384;

interface ResizeControlsProps {
  settings: EditorSettings;
  sourceWidth: number;
  sourceHeight: number;
  onChange: (next: EditorSettings) => void;
}

export function ResizeControls({ settings, sourceWidth, sourceHeight, onChange }: ResizeControlsProps) {
  const ratio = sourceHeight / sourceWidth;

  const setWidth = (value: string) => {
    const width = value === "" ? null : Math.max(1, Number(value));
    onChange({
      ...settings,
      width,
      // The mirrored height is only a hint for the user; with the ratio locked
      // the engine is the one that computes the real value.
      height: settings.lockRatio && width !== null ? Math.round(width * ratio) : settings.height,
    });
  };

  return (
    <fieldset className="controls">
      <legend>Tamanho</legend>

      <label htmlFor="width">Largura</label>
      <input
        id="width"
        type="number"
        min={1}
        max={MAX_SIDE}
        value={settings.width ?? ""}
        placeholder={String(sourceWidth)}
        onChange={(e) => setWidth(e.target.value)}
      />

      <label htmlFor="height">Altura</label>
      <input
        id="height"
        type="number"
        min={1}
        max={MAX_SIDE}
        disabled={settings.lockRatio}
        value={settings.height ?? ""}
        placeholder={String(sourceHeight)}
        onChange={(e) =>
          onChange({ ...settings, height: e.target.value === "" ? null : Math.max(1, Number(e.target.value)) })
        }
      />

      <label htmlFor="lock">
        <input
          id="lock"
          type="checkbox"
          checked={settings.lockRatio}
          onChange={(e) =>
            onChange({
              ...settings,
              lockRatio: e.target.checked,
              fit: e.target.checked ? "Contain" : settings.fit,
              // Re-locking hides the height field, and `buildJob` then stops
              // sending the value. Dropping it here keeps a height typed while
              // unlocked from later producing a resize with neither dimension,
              // which the engine rejects with no way back through the UI.
              height: e.target.checked ? null : settings.height,
            })
          }
        />
        Manter proporção
      </label>

      {!settings.lockRatio && (
        <>
          <label htmlFor="fit">Encaixe</label>
          <select
            id="fit"
            value={settings.fit}
            onChange={(e) => onChange({ ...settings, fit: e.target.value as FitMode })}
          >
            <option value="Contain">Caber dentro, com preenchimento</option>
            <option value="Cover">Preencher e recortar</option>
            <option value="Stretch">Esticar (distorce)</option>
          </select>
        </>
      )}
    </fieldset>
  );
}
