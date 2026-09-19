import type { EditorSettings, OutputFormat } from "../api";

interface QualityControlsProps {
  settings: EditorSettings;
  onChange: (next: EditorSettings) => void;
}

const FORMATS: { value: OutputFormat; label: string }[] = [
  { value: "WebP", label: "WebP" },
  { value: "Avif", label: "AVIF" },
  { value: "Png", label: "PNG" },
  { value: "Jpeg", label: "JPG" },
  { value: "Tiff", label: "TIFF" },
  { value: "Ico", label: "ICO (até 256px)" },
];

const LOSSY: OutputFormat[] = ["WebP", "Avif", "Jpeg"];

export function QualityControls({ settings, onChange }: QualityControlsProps) {
  const slider = (
    id: string,
    label: string,
    value: number,
    min: number,
    max: number,
    step: number,
    key: keyof EditorSettings,
  ) => (
    <>
      <label htmlFor={id}>
        {label} <span className="mono">{value}</span>
      </label>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange({ ...settings, [key]: Number(e.target.value) })}
      />
    </>
  );

  return (
    <fieldset className="controls">
      <legend>Qualidade</legend>

      {slider("sharpen", "Nitidez", settings.sharpenAmount, 0, 3, 0.1, "sharpenAmount")}
      {slider("denoise", "Redução de ruído", settings.denoiseRadius, 0, 5, 1, "denoiseRadius")}
      {slider("brightness", "Brilho", settings.brightness, -1, 1, 0.05, "brightness")}
      {slider("contrast", "Contraste", settings.contrast, -1, 1, 0.05, "contrast")}
      {slider("saturation", "Saturação", settings.saturation, -1, 1, 0.05, "saturation")}

      <label htmlFor="format">Formato de saída</label>
      <select
        id="format"
        value={settings.format}
        onChange={(e) => onChange({ ...settings, format: e.target.value as OutputFormat })}
      >
        {FORMATS.map((f) => (
          <option key={f.value} value={f.value}>
            {f.label}
          </option>
        ))}
      </select>

      {LOSSY.includes(settings.format) &&
        slider("quality", "Compressão", settings.quality, 1, 100, 1, "quality")}
    </fieldset>
  );
}
