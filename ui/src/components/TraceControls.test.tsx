import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TraceControls } from "./TraceControls";
import type { EditorSettings } from "../api";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const base: EditorSettings = {
  width: null, height: null, lockRatio: true, fit: "Contain",
  sharpenAmount: 0, denoiseRadius: 0, brightness: 0, contrast: 0, saturation: 0,
  format: "Svg", quality: 82, traceMode: "Color", traceColors: 8, removeBackground: true,
};

describe("TraceControls", () => {
  beforeEach(() => vi.clearAllMocks());

  it("mostra Cores e Remover fundo no modo Cores", () => {
    render(<TraceControls settings={base} onChange={vi.fn()} svgCode="<svg/>" onError={vi.fn()} />);
    expect(screen.getByLabelText(/^Cores/)).toBeInTheDocument();
    expect(screen.getByLabelText("Remover fundo")).toBeInTheDocument();
    expect(screen.getByText(/inclusive dentro das letras/)).toBeInTheDocument();
  });

  it("esconde Cores e Remover fundo no modo Uma cor e explica o currentColor", () => {
    render(<TraceControls settings={{ ...base, traceMode: "Mono" }} onChange={vi.fn()} svgCode="<svg/>" onError={vi.fn()} />);
    expect(screen.queryByLabelText(/^Cores/)).toBeNull();
    expect(screen.queryByLabelText("Remover fundo")).toBeNull();
    expect(screen.getByText(/currentColor/)).toBeInTheDocument();
  });

  it("copia o código SVG da prévia atual", async () => {
    render(<TraceControls settings={base} onChange={vi.fn()} svgCode="<svg>x</svg>" onError={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Copiar código SVG" }));
    expect(writeText).toHaveBeenCalledWith("<svg>x</svg>");
    expect(await screen.findByRole("status")).toHaveTextContent("Código SVG copiado.");
  });

  it("desabilita Copiar código SVG sem prévia pronta", () => {
    render(<TraceControls settings={base} onChange={vi.fn()} svgCode={null} onError={vi.fn()} />);
    expect(screen.getByRole("button", { name: "Copiar código SVG" })).toBeDisabled();
  });

  it("mostra o erro quando a cópia falha", async () => {
    vi.mocked(writeText).mockRejectedValueOnce(new Error("bloqueada"));
    const onError = vi.fn();
    render(<TraceControls settings={base} onChange={vi.fn()} svgCode="<svg/>" onError={onError} />);
    await userEvent.click(screen.getByRole("button", { name: "Copiar código SVG" }));
    await vi.waitFor(() => expect(onError).toHaveBeenCalledTimes(2));
    expect(onError.mock.calls[1][0]).toMatch(/^Não foi possível copiar o código SVG/);
  });
});
