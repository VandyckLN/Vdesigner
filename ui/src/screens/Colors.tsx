import { useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  api,
  PALETTE_FORMAT_VERSION,
  RAMP_STEPS,
  type ColorFormat,
  type Palette,
  type Variations,
} from "../api";

const FORMATS: ColorFormat[] = ["Hex", "Rgb", "Hsl", "Oklch"];

/** Mirrors validate_name in crates/core/src/palette.rs. Checked here too so
 *  the person learns the rule while typing instead of after a round trip. */
const VALID_NAME = /^[a-z0-9]+(-[a-z0-9]+)*$/;

/** Three or six hex digits, with the hash optional. The hash is optional on
 *  purpose: typing `0f8` is how people write a colour by hand, and the old
 *  length-based guard threw that input away silently — no ramp, no error. */
const HEX_PATTERN = /^#?(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

/** Returns the canonical `#rrggbb`-shaped form, or null when the text is not
 *  a hex colour at all. Everything downstream (the engine call, the saved
 *  swatch, the clipboard) uses the canonical form, so a colour typed without
 *  the hash is stored the same way as one typed with it. */
function normalizeHex(raw: string): string | null {
  const trimmed = raw.trim();
  if (!HEX_PATTERN.test(trimmed)) return null;
  return trimmed.startsWith("#") ? trimmed : `#${trimmed}`;
}

const EMPTY_PALETTE: Palette = {
  // Never spell the number out here: the Rust side owns the format version.
  versao: PALETTE_FORMAT_VERSION,
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
  const [withRamp, setWithRamp] = useState(false);
  const [format, setFormat] = useState<ColorFormat>("Hex");
  const [variations, setVariations] = useState<Variations | null>(null);
  const [palette, setPalette] = useState<Palette>(EMPTY_PALETTE);
  const [dir, setDir] = useState<string | null>(null);
  const [onDisk, setOnDisk] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const normalized = normalizeHex(hex);

  useEffect(() => {
    if (hex.trim().length === 0) {
      setVariations(null);
      return;
    }
    if (normalized === null) {
      // Say why instead of quietly clearing the ramp: a half-typed colour and
      // a rejected one must not look identical to the person typing.
      setVariations(null);
      setError("Cor inválida. Use três ou seis dígitos hexadecimais, com ou sem #.");
      return;
    }
    let cancelled = false;
    api
      .colorVariations(normalized)
      .then((result) => {
        if (cancelled) return;
        setVariations(result);
        setError(null);
      })
      .catch(() => {
        if (cancelled) return;
        setVariations(null);
        setError("Cor inválida. Use três ou seis dígitos hexadecimais, com ou sem #.");
      });
    // The input fires on every keystroke; a stale reply must not overwrite a
    // newer one.
    return () => {
      cancelled = true;
    };
  }, [hex, normalized]);

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
    // Both the engine call and the clipboard can fail — a clipboard locked by
    // another Windows process is routine. Unwrapped, the rejection was an
    // unhandled promise with no feedback at all: the button simply did
    // nothing. Every other async path here reports through the same alert.
    try {
      const text = await api.formatColor(normalized ?? hex, format);
      await writeText(text);
      setError(null);
    } catch (e) {
      setError(`Não foi possível copiar a cor: ${String(e)}`);
    }
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
    if (normalized === null) {
      setError("Cor inválida. Use três ou seis dígitos hexadecimais, com ou sem #.");
      return;
    }
    if (!dir) return;

    const next: Palette = {
      ...palette,
      // `rampa` is the user's choice, not a constant: it is the only thing
      // that makes the ten-step scale reach their CSS file.
      cores: [...palette.cores, { nome: name, hex: normalized, rampa: withRamp }],
    };
    const ok = await persist(next);
    if (ok) setName("");
  };

  const save = async () => {
    await persist(palette);
  };

  /** Removal goes through the same `persist` helper as everything else, so a
   *  removed colour is gone from disk, not merely from the screen. */
  const remove = async (index: number) => {
    await persist({ ...palette, cores: palette.cores.filter((_, i) => i !== index) });
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

        <label className="colors-ramp-toggle">
          <input
            type="checkbox"
            checked={withRamp}
            onChange={(e) => setWithRamp(e.target.checked)}
          />
          Gerar a escala de tons
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

      {dir && (
        <section aria-labelledby="palette-heading">
          <h2 id="palette-heading" className="mono">
            Cores da paleta
          </h2>
          {palette.cores.length === 0 ? (
            <p className="palette-empty">Nenhuma cor gravada nesta pasta ainda.</p>
          ) : (
            <ul className="palette-list" aria-label="Cores da paleta">
              {palette.cores.map((cor, index) => (
                <li key={`${cor.nome}-${index}`}>
                  {/* The chip is decorative: the name and the hex beside it
                      carry the same information as text, so the entry never
                      depends on colour perception. */}
                  <span className="swatch-chip" style={{ background: cor.hex }} aria-hidden="true" />
                  <span className="palette-name">{cor.nome}</span>
                  <code className="palette-hex">{cor.hex}</code>
                  <button
                    type="button"
                    className="palette-remove"
                    onClick={() => remove(index)}
                    aria-label={`Remover a cor ${cor.nome}`}
                  >
                    Remover
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {variations && (
        <>
          <ol className="ramp" aria-label="Escala de tons">
            {variations.ramp.map((shade, index) => (
              <li key={RAMP_STEPS[index]}>
                <span className="swatch-chip" style={{ background: shade }} aria-hidden="true" />
                <span className="swatch-meta">
                  <span>{RAMP_STEPS[index]}</span>
                  <code>{shade}</code>
                </span>
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
              <li key={`${label}-${value}`}>
                <span className="swatch-chip" style={{ background: value }} aria-hidden="true" />
                <span className="swatch-meta">
                  <span>{label}</span>
                  <code>{value}</code>
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </main>
  );
}
