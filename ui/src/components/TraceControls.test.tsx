import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TraceControls } from "./TraceControls";
import type { EditorSettings } from "../api";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const base: EditorSettings = {
  width: null, height: null, lockRatio: true, fit: "Contain",
  sharpenAmount: 0, denoiseRadius: 0, brightness: 0, contrast: 0, saturation: 0,
  format: "Svg", quality: 82, traceMode: "Color", traceColors: 8, removeBackground: true, traceDetail: "Normal",
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

  it("lista Foto original, Foto em vetor, Cores e Uma cor, nesta ordem", () => {
    render(<TraceControls settings={base} onChange={vi.fn()} svgCode="<svg/>" onError={vi.fn()} />);
    const options = within(screen.getByLabelText("Modo")).getAllByRole<HTMLOptionElement>("option");
    expect(options.map((o) => [o.value, o.textContent])).toEqual([
      ["Embed", "Foto original"],
      ["Photo", "Foto em vetor"],
      ["Color", expect.stringMatching(/^Cores \(logo/)],
      ["Mono", expect.stringMatching(/^Uma cor \(ícone/)],
    ]);
  });

  it("no modo Foto original esconde Detalhe, Cores e Remover fundo e diz o que fica no arquivo", () => {
    render(<TraceControls settings={{ ...base, traceMode: "Embed" }} onChange={vi.fn()} svgCode="<svg/>" onError={vi.fn()} />);
    expect(screen.getByRole("group", { name: "SVG" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Detalhe")).toBeNull();
    expect(screen.queryByLabelText(/^Cores/)).toBeNull();
    expect(screen.queryByLabelText("Remover fundo")).toBeNull();
    expect(screen.getByText(/não vira formas editáveis/)).toBeInTheDocument();
    expect(screen.getByText(/pixela/)).toBeInTheDocument();
    expect(screen.getByText(/Com transparência, ela vai em PNG: sem perda, mais pesada, e aí a Compressão não muda nada/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copiar código SVG" })).toBeEnabled();
  });

  it("no modo Foto em vetor mostra Detalhe e esconde Cores e Remover fundo", async () => {
    const onChange = vi.fn();
    render(<TraceControls settings={{ ...base, traceMode: "Photo" }} onChange={onChange} svgCode="<svg/>" onError={vi.fn()} />);
    expect(screen.queryByLabelText(/^Cores/)).toBeNull();
    expect(screen.queryByLabelText("Remover fundo")).toBeNull();
    const detail = screen.getByLabelText("Detalhe");
    expect(Array.from(detail.querySelectorAll("option")).map((o) => o.textContent)).toEqual(["Normal", "Alto", "Máximo"]);
    expect(screen.getByText(/cara de pintura/)).toBeInTheDocument();
    await userEvent.selectOptions(detail, "Max");
    expect(onChange).toHaveBeenCalledWith({ ...base, traceMode: "Photo", traceDetail: "Max" });
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
    expect(onError).toHaveBeenLastCalledWith(expect.stringMatching(/^Não foi possível copiar o código SVG/));
  });
});
