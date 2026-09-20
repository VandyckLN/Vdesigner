import { useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  api,
  RAMP_STEPS,
  type ColorFormat,
  type Palette,
  type Variations,
} from "../api";

const FORMATS: ColorFormat[] = ["Hex", "Rgb", "Hsl", "Oklch"];

/** Mirrors validate_name in crates/core/src/palette.rs. Checked here too so
 *  the person learns the rule while typing instead of after a round trip. */
const VALID_NAME = /^[a-z0-9]+(-[a-z0-9]+)*$/;

const EMPTY_PALETTE: Palette = {
  versao: 1,
  nome: "paleta",
  gerar: ["css"],
  cores: [],
  degrades: [],
};

export function Colors({
  onPickDirectory,
}: {
  onPickDirectory: () => Promise<string | null>;
}) {
  const [hex, setHex] = useState("");
  const [name, setName] = useState("");
  const [format, setFormat] = useState<ColorFormat>("Hex");
  const [variations, setVariations] = useState<Variations | null>(null);
  const [palette, setPalette] = useState<Palette>(EMPTY_PALETTE);
  const [dir, setDir] = useState<string | null>(null);
  const [onDisk, setOnDisk] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (hex.trim().length < 4) {
      setVariations(null);
      return;
    }
    let cancelled = false;
    api
      .colorVariations(hex)
      .then((result) => {
        if (cancelled) return;
        setVariations(result);
        setError(null);
      })
      .catch(() => {
        if (cancelled) return;
        setVariations(null);
        setError("Cor inválida. Use três ou seis dígitos hexadecimais.");
      });
    // The input fires on every keystroke; a stale reply must not overwrite a
    // newer one.
    return () => {
      cancelled = true;
    };
  }, [hex]);

  const pickFolder = async () => {
    const chosen = await onPickDirectory();
    if (!chosen) return;
    try {
      const snapshot = await api.loadPalette(chosen);
      // Only commit to the new directory once its palette has actually
      // loaded. Setting `dir` earlier (or on failure) would let a "Gravar"
      // right after a failed pick write the previous folder's palette into
      // this one, or leave a stale palette/onDisk pair under a directory
      // whose contents the component never actually read.
      setDir(chosen);
      setPalette(snapshot?.palette ?? EMPTY_PALETTE);
      setOnDisk(snapshot?.on_disk ?? null);
      setError(null);
    } catch (e) {
      setError(`Não foi possível abrir a paleta da pasta escolhida: ${String(e)}`);
    }
  };

  const copy = async () => {
    const text = await api.formatColor(hex, format);
    await writeText(text);
  };

  /** Shared persistence so "Adicionar" and "Gravar" cannot drift apart: both
   *  end up calling the exact same save + snapshot-update path. */
  const persist = async (next: Palette): Promise<boolean> => {
    if (!dir) return false;
    try {
      const written = await api.savePalette(dir, next, onDisk);
      setPalette(next);
      setOnDisk(written);
      setError(null);
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    }
  };

  // CONTROLLER RULING: the plan had "Adicionar" and "Gravar" both call the
  // same handler, which forced name validation onto a plain re-save of an
  // already-loaded palette. Split deliberately: "Adicionar" appends a new
  // swatch (so it must validate the name), "Gravar" only persists whatever
  // is already in state (so it must not).
  const add = async () => {
    if (!VALID_NAME.test(name)) {
      setError("Use apenas letras minúsculas sem acento, números e hífen.");
      return;
    }
    if (!dir) return;

    const next: Palette = {
      ...palette,
      cores: [...palette.cores, { nome: name, hex, rampa: false }],
    };
    const ok = await persist(next);
    if (ok) setName("");
  };

  const save = async () => {
    await persist(palette);
  };

  return (
    <main className="colors">
      <div className="colors-controls">
        <label>
          Cor em hex
          <input value={hex} onChange={(e) => setHex(e.target.value)} placeholder="#8A9096" />
        </label>

        <label>
          Formato
          <select value={format} onChange={(e) => setFormat(e.target.value as ColorFormat)}>
            {FORMATS.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
        </label>

        <button type="button" onClick={copy} disabled={!variations}>
          Copiar
        </button>

        <button type="button" onClick={pickFolder}>
          Escolher pasta
        </button>

        <label>
          Nome da cor
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="acento" />
        </label>

        <button type="button" onClick={add} disabled={!dir || !name.trim() || !hex.trim()}>
          Adicionar
        </button>

        <button type="button" disabled={!dir} onClick={save}>
          Gravar
        </button>
      </div>

      {error && (
        <p className="colors-error" role="alert">
          {error}
        </p>
      )}

      {variations && (
        <>
          <ol className="ramp" aria-label="Escala de tons">
            {variations.ramp.map((shade, index) => (
              <li key={RAMP_STEPS[index]} style={{ background: shade }}>
                <span>{RAMP_STEPS[index]}</span>
                <code>{shade}</code>
              </li>
            ))}
          </ol>

          <ul className="harmony" aria-label="Harmonias">
            {[
              { label: "Complementar", value: variations.complementary },
              ...variations.analogous.map((value, i) => ({
                label: `Análoga ${i + 1}`,
                value,
              })),
              ...variations.triad.map((value, i) => ({ label: `Tríade ${i + 1}`, value })),
            ].map(({ label, value }) => (
              <li key={`${label}-${value}`} style={{ background: value }}>
                <span>{label}</span>
                <code>{value}</code>
              </li>
            ))}
          </ul>
        </>
      )}
    </main>
  );
}
