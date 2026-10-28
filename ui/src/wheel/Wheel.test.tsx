import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { centroDaFatia, fatia, QUALIDADE_PADRAO, Wheel } from "./Wheel";

const startWheel = vi.fn();
const cancelWheel = vi.fn();
const convertSelection = vi.fn();
const onWheelProgress = vi.fn();

vi.mock("../api", () => ({
  api: {
    startWheel: () => startWheel(),
    cancelWheel: () => cancelWheel(),
    convertSelection: (target: string, quality: number) => convertSelection(target, quality),
    onWheelProgress: (handler: (p: unknown) => void) => onWheelProgress(handler),
  },
}));

const ESTADO = {
  files: [
    { path: "C:\\fotos\\a.png", kind: "Raster" as const },
    { path: "C:\\fotos\\b.jpg", kind: "Raster" as const },
  ],
  skipped: [],
  truncated: false,
  targets: ["WebP", "Avif", "Png", "Jpeg", "Tiff", "Svg"] as const,
};

describe("Wheel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    startWheel.mockResolvedValue(ESTADO);
    cancelWheel.mockResolvedValue(undefined);
    convertSelection.mockResolvedValue({ written: [], failed: [] });
    // The progress stream is a no-op unlisten in every test but the one that
    // exercises it.
    onWheelProgress.mockResolvedValue(() => {});
  });

  it("desenha uma fatia por formato oferecido", async () => {
    render(<Wheel />);
    const itens = await screen.findAllByRole("menuitem");
    expect(itens).toHaveLength(ESTADO.targets.length);
    expect(itens.map((i) => i.getAttribute("aria-label"))).toEqual([
      "WebP",
      "AVIF",
      "PNG",
      "JPG",
      "TIFF",
      "SVG",
    ]);
  });

  it("mostra quantos arquivos vão ser convertidos", async () => {
    render(<Wheel />);
    expect(await screen.findByText("2")).toBeInTheDocument();
    expect(screen.getByText("arquivos")).toBeInTheDocument();
  });

  it("usa o singular quando é um arquivo só", async () => {
    startWheel.mockResolvedValue({ ...ESTADO, files: [ESTADO.files[0]] });
    render(<Wheel />);
    expect(await screen.findByText("arquivo")).toBeInTheDocument();
  });

  it("converte no formato clicado", async () => {
    render(<Wheel />);
    const itens = await screen.findAllByRole("menuitem");
    await userEvent.click(itens[2]);
    expect(convertSelection).toHaveBeenCalledWith("Png", QUALIDADE_PADRAO);
  });

  it("não dispara uma segunda conversão enquanto a primeira está em voo", async () => {
    // The window is already closing; a second call would meet a disarmed
    // wheel and raise an error the person cannot act on.
    convertSelection.mockReturnValue(new Promise(() => {}));
    render(<Wheel />);
    const itens = await screen.findAllByRole("menuitem");
    await userEvent.click(itens[0]);
    await userEvent.click(itens[1]);
    expect(convertSelection).toHaveBeenCalledTimes(1);
  });

  it("as setas andam pelas fatias e Enter confirma a que está em foco", async () => {
    render(<Wheel />);
    await screen.findAllByRole("menuitem");
    await userEvent.keyboard("{ArrowRight}{ArrowRight}");
    await userEvent.keyboard("{Enter}");
    expect(convertSelection).toHaveBeenCalledWith("Png", QUALIDADE_PADRAO);
  });

  it("a seta para a esquerda dá a volta no começo da lista", async () => {
    render(<Wheel />);
    await screen.findAllByRole("menuitem");
    await userEvent.keyboard("{ArrowLeft}{Enter}");
    expect(convertSelection).toHaveBeenCalledWith("Svg", QUALIDADE_PADRAO);
  });

  it("Esc fecha sem converter nada", async () => {
    render(<Wheel />);
    await screen.findAllByRole("menuitem");
    await userEvent.keyboard("{Escape}");
    expect(cancelWheel).toHaveBeenCalled();
    expect(convertSelection).not.toHaveBeenCalled();
  });

  it("avisa quantos arquivos ficaram fora da seleção", async () => {
    startWheel.mockResolvedValue({
      ...ESTADO,
      skipped: ["C:\\x\\notas.txt", "C:\\x\\planilha.pdf"],
    });
    render(<Wheel />);
    expect(
      await screen.findByText("2 arquivos fora: o Vdesigner não abre esses formatos."),
    ).toBeInTheDocument();
  });

  it("usa o singular no aviso de um arquivo fora", async () => {
    startWheel.mockResolvedValue({ ...ESTADO, skipped: ["C:\\x\\notas.txt"] });
    render(<Wheel />);
    expect(
      await screen.findByText("1 arquivo fora: o Vdesigner não abre esse formato."),
    ).toBeInTheDocument();
  });

  it("avisa quando a seleção passou do limite", async () => {
    startWheel.mockResolvedValue({ ...ESTADO, truncated: true });
    render(<Wheel />);
    expect(
      await screen.findByText(/só os primeiros 2 entraram/, { exact: false }),
    ).toBeInTheDocument();
  });

  it("mostra o erro quando não há seleção para ler", async () => {
    startWheel.mockRejectedValue("nenhum arquivo está selecionado no Explorer");
    render(<Wheel />);
    const aviso = await screen.findByRole("alert");
    expect(aviso).toHaveTextContent("nenhum arquivo está selecionado no Explorer");
  });

  it("troca a roda pelo andamento quando o lote começa", async () => {
    let publicar: ((p: unknown) => void) | undefined;
    onWheelProgress.mockImplementation((handler: (p: unknown) => void) => {
      publicar = handler;
      return Promise.resolve(() => {});
    });
    render(<Wheel />);
    await screen.findAllByRole("menuitem");
    await waitFor(() => expect(publicar).toBeDefined());

    publicar?.({ current: 1, total: 2, path: "C:\\fotos\\a.png", error: null });
    expect(await screen.findByRole("status")).toHaveTextContent("Convertendo 1 de 2");
    expect(screen.queryAllByRole("menuitem")).toHaveLength(0);
  });

  it("mostra o motivo de um arquivo que falhou no meio do lote", async () => {
    let publicar: ((p: unknown) => void) | undefined;
    onWheelProgress.mockImplementation((handler: (p: unknown) => void) => {
      publicar = handler;
      return Promise.resolve(() => {});
    });
    render(<Wheel />);
    await screen.findAllByRole("menuitem");
    await waitFor(() => expect(publicar).toBeDefined());

    publicar?.({
      current: 1,
      total: 2,
      path: "C:\\fotos\\a.png",
      error: "falha ao decodificar a imagem",
    });
    expect(await screen.findByText("falha ao decodificar a imagem")).toBeInTheDocument();
  });
});

describe("geometria da roda", () => {
  it("cada fatia é um anel fechado", () => {
    for (const total of [2, 5, 6]) {
      for (let i = 0; i < total; i++) {
        const caminho = fatia(i, total);
        expect(caminho.startsWith("M ")).toBe(true);
        expect(caminho.endsWith("Z")).toBe(true);
        // Outer arc, inner arc.
        expect(caminho.match(/A /g)).toHaveLength(2);
        expect(caminho).not.toContain("NaN");
      }
    }
  });

  it("marca o arco grande só quando a fatia passa de meia volta", () => {
    // Two targets means each slice is exactly half a turn, which is not
    // "large"; a single target would be the whole circle.
    expect(fatia(0, 2)).toContain("0 1 ");
    expect(fatia(0, 1)).toContain("1 1 ");
  });

  it("as fatias dividem a volta inteira sem repetir centro", () => {
    const total = 6;
    const centros = Array.from({ length: total }, (_, i) =>
      JSON.stringify(centroDaFatia(i, total)),
    );
    expect(new Set(centros).size).toBe(total);
  });

  it("a primeira fatia começa no topo", () => {
    const centro = centroDaFatia(0, 4);
    // A quarter turn starting at twelve o'clock has its midpoint up and to
    // the right of centre.
    expect(centro.y).toBeLessThan(200);
    expect(centro.x).toBeGreaterThan(200);
  });
});
