import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Editor } from "./Editor";
import { api, type PreviewResult, type SvgPreview } from "../api";
import { formatBytes } from "../format";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    api: {
      openImage: vi.fn(),
      preview: vi.fn(),
      previewSvg: vi.fn(),
      estimate: vi.fn(),
      export: vi.fn(),
      onExportProgress: vi.fn(),
    },
  };
});

const openImage = vi.mocked(api.openImage);
const preview = vi.mocked(api.preview);
const previewSvg = vi.mocked(api.previewSvg);
const estimate = vi.mocked(api.estimate);
const exportFile = vi.mocked(api.export);
const onExportProgress = vi.mocked(api.onExportProgress);

describe("Editor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    preview.mockResolvedValue({ png_base64: "AAA", width: 100, height: 50 });
    estimate.mockResolvedValue({ bytes: 0 });
    onExportProgress.mockResolvedValue(() => {});
  });

  it("mostra o estado vazio antes de abrir uma imagem", () => {
    render(<Editor onOpenFile={vi.fn()} onPickDirectory={vi.fn()} />);
    expect(screen.getByText(/clique em abrir imagem/i)).toBeInTheDocument();
  });

  it("mostra as dimensões depois de abrir uma imagem", async () => {
    openImage.mockResolvedValue({
      width: 1920,
      height: 1080,
      file_stem: "hero",
      preview_png_base64: "AAA",
      is_vector: false,
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} onPickDirectory={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));

    await waitFor(() => expect(screen.getByText(/1920 × 1080/)).toBeInTheDocument());
  });

  it("preenche a pasta de saída com o que o diálogo devolve", async () => {
    openImage.mockResolvedValue({
      width: 800,
      height: 400,
      file_stem: "hero",
      preview_png_base64: "AAA",
      is_vector: false,
    });

    render(
      <Editor
        onOpenFile={async () => "C:/temp/hero.jpg"}
        onPickDirectory={async () => "D:/Fotos/Saida"}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await userEvent.click(await screen.findByRole("button", { name: /escolher pasta/i }));

    await waitFor(() =>
      expect(screen.getByLabelText(/pasta de saída/i)).toHaveValue("D:/Fotos/Saida"),
    );
  });

  it("mantém a pasta de saída ao abrir outra imagem", async () => {
    openImage
      .mockResolvedValueOnce({
        width: 800,
        height: 400,
        file_stem: "hero",
        preview_png_base64: "AAA",
        is_vector: false,
      })
      .mockResolvedValueOnce({
        width: 640,
        height: 480,
        file_stem: "outra",
        preview_png_base64: "BBB",
        is_vector: false,
      });

    render(
      <Editor
        onOpenFile={async () => "C:/temp/hero.jpg"}
        onPickDirectory={async () => "D:/Fotos/Saida"}
      />,
    );

    const abrir = screen.getByRole("button", { name: /abrir imagem/i });
    await userEvent.click(abrir);
    await userEvent.click(await screen.findByRole("button", { name: /escolher pasta/i }));
    await waitFor(() =>
      expect(screen.getByLabelText(/pasta de saída/i)).toHaveValue("D:/Fotos/Saida"),
    );

    // The second image remounts the export panel by changing its key.
    await userEvent.click(abrir);
    await waitFor(() => expect(screen.getByText(/640 × 480/)).toBeInTheDocument());
    expect(screen.getByLabelText(/pasta de saída/i)).toHaveValue("D:/Fotos/Saida");
  });

  it("mostra o tamanho estimado do arquivo de saída", async () => {
    openImage.mockResolvedValue({
      width: 800,
      height: 400,
      file_stem: "hero",
      preview_png_base64: "AAA",
      is_vector: false,
    });
    estimate.mockResolvedValue({ bytes: 2048 });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} onPickDirectory={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));

    expect(await screen.findByText(/~2\.0 KB/, undefined, { timeout: 3000 })).toBeInTheDocument();
  });

  it("mostra o estágio do export enquanto ele roda", async () => {
    openImage.mockResolvedValue({
      width: 800,
      height: 400,
      file_stem: "hero",
      preview_png_base64: "AAA",
      is_vector: false,
    });

    let emit: ((p: { current: number; total: number; label: string }) => void) | undefined;
    onExportProgress.mockImplementation(async (handler) => {
      emit = handler;
      return () => {};
    });

    let finish: ((value: { path: string; bytes_written: number }) => void) | undefined;
    exportFile.mockImplementation(() => new Promise((resolve) => (finish = resolve)));

    render(
      <Editor
        onOpenFile={async () => "C:/temp/hero.jpg"}
        onPickDirectory={async () => "D:/Fotos/Saida"}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await userEvent.click(await screen.findByRole("button", { name: /escolher pasta/i }));
    await userEvent.click(await screen.findByRole("button", { name: /^exportar$/i }));

    emit?.({ current: 2, total: 3, label: "codificando" });
    expect(await screen.findByText(/2\/3 codificando/i)).toBeInTheDocument();

    finish?.({ path: "D:/Fotos/Saida/hero.webp", bytes_written: 100 });
    expect(await screen.findByText(/salvo em/i)).toBeInTheDocument();
  });

  it("desabilita o campo de altura enquanto a proporção está travada", async () => {
    openImage.mockResolvedValue({
      width: 800,
      height: 400,
      file_stem: "hero",
      preview_png_base64: "AAA",
      is_vector: false,
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} onPickDirectory={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await waitFor(() => expect(screen.getByLabelText(/altura/i)).toBeDisabled());
  });

  const open = async (isVector = false) => {
    openImage.mockResolvedValue({
      width: 800, height: 400, file_stem: "hero", preview_png_base64: "AAA", is_vector: isVector,
    });
    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} onPickDirectory={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await screen.findByLabelText("Formato de saída");
  };
  const pickSvg = () => userEvent.selectOptions(screen.getByLabelText("Formato de saída"), "Svg");
  const src = () => screen.getByRole("img").getAttribute("src");
  const deferred = <T,>() => {
    let resolve!: (v: T) => void;
    const promise = new Promise<T>((r) => { resolve = r; });
    return { promise, resolve };
  };
  const png = (b: string): PreviewResult => ({ png_base64: b, width: 1, height: 1 });

  it("oferece SVG no formato de saída, sem prometer vetorização", async () => {
    await open();
    expect(screen.getByRole("option", { name: "SVG" })).toBeEnabled();
    expect(screen.queryByRole("option", { name: /vetorizado/ })).toBeNull();
  });

  it("desabilita SVG quando a imagem aberta já é SVG", async () => {
    await open(true);
    expect(screen.getByRole("option", { name: "SVG" })).toBeDisabled();
  });

  it("usa a prévia SVG em Foto original e mostra o tamanho exato do SVG", async () => {
    previewSvg.mockResolvedValue({ svg: "<svg>" + "a".repeat(1234 - 11) + "</svg>" });
    await open();
    estimate.mockClear();
    await pickSvg();
    expect(await screen.findByText(`Tamanho ${formatBytes(1234)}`)).toBeInTheDocument();
    const job = previewSvg.mock.calls[0][0];
    expect(job.output.format).toBe("Svg");
    expect(job.output.trace).toEqual({ mode: "Embed", colors: 8, remove_background: true, detail: "Normal" });
    expect(estimate).not.toHaveBeenCalled();
    expect(screen.queryByText(/estimado/)).toBeNull();
    expect(src()).toMatch(/^data:image\/svg\+xml/);
  });

  it("mostra processando… na Foto original e vetorizando… na Foto em vetor", async () => {
    const embed = deferred<SvgPreview>();
    const traced = deferred<SvgPreview>();
    previewSvg.mockReturnValueOnce(embed.promise).mockReturnValueOnce(traced.promise);
    await open();
    await pickSvg();
    await waitFor(() => expect(previewSvg).toHaveBeenCalledTimes(1));
    expect(screen.getByRole("status")).toHaveTextContent("processando…");
    embed.resolve({ svg: "<svg/>" });
    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
    await userEvent.selectOptions(screen.getByLabelText("Modo"), "Photo");
    await waitFor(() => expect(previewSvg).toHaveBeenCalledTimes(2));
    expect(screen.getByRole("status")).toHaveTextContent("vetorizando…");
    traced.resolve({ svg: "<svg/>" });
    await waitFor(() => expect(screen.queryByText("vetorizando…")).toBeNull());
  });

  it("mostra Compressão na Foto original e a leva para o job; na Foto em vetor ela some", async () => {
    previewSvg.mockResolvedValue({ svg: "<svg/>" });
    await open();
    await pickSvg();
    fireEvent.change(screen.getByLabelText(/compressão/i), { target: { value: "60" } });
    await waitFor(() => expect(previewSvg.mock.lastCall?.[0].output.quality).toBe(60));
    expect(previewSvg.mock.lastCall?.[0].output.trace.mode).toBe("Embed");
    await userEvent.selectOptions(screen.getByLabelText("Modo"), "Photo");
    expect(screen.queryByLabelText(/compressão/i)).toBeNull();
    expect(screen.getByLabelText("Detalhe")).toBeInTheDocument();
  });

  it("desabilita Copiar código SVG até a prévia nova chegar", async () => {
    const second = deferred<SvgPreview>();
    previewSvg.mockResolvedValueOnce({ svg: "<svg/>" }).mockReturnValueOnce(second.promise);
    await open();
    await pickSvg();
    const copy = () => screen.getByRole("button", { name: "Copiar código SVG" });
    await waitFor(() => expect(copy()).toBeEnabled());
    fireEvent.change(screen.getByLabelText(/compressão/i), { target: { value: "60" } });
    await waitFor(() => expect(copy()).toBeDisabled());
    second.resolve({ svg: "<svg>2</svg>" });
    await waitFor(() => expect(copy()).toBeEnabled());
  });

  it("esconde o tamanho do SVG anterior durante uma nova vetorização", async () => {
    const second = deferred<SvgPreview>();
    previewSvg.mockResolvedValueOnce({ svg: "<svg>1234</svg>" }).mockReturnValueOnce(second.promise);
    await open();
    await pickSvg();
    expect(await screen.findByText(/^Tamanho /)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/compressão/i), { target: { value: "60" } });
    await waitFor(() => expect(previewSvg).toHaveBeenCalledTimes(2));
    expect(screen.queryByText(/^Tamanho /)).toBeNull();
    second.resolve({ svg: "<svg>22222</svg>" });
    expect(await screen.findByText(/^Tamanho /)).toBeInTheDocument();
  });

  it("ignora uma prévia antiga que chega depois da nova", async () => {
    const a = deferred<PreviewResult>();
    const b = deferred<PreviewResult>();
    await open();
    preview.mockReset();
    preview.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise);
    fireEvent.change(screen.getByLabelText(/nitidez/i), { target: { value: "1" } });
    await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
    fireEvent.change(screen.getByLabelText(/nitidez/i), { target: { value: "2" } });
    await waitFor(() => expect(preview).toHaveBeenCalledTimes(2));
    b.resolve(png("BBB"));
    await waitFor(() => expect(src()).toContain("BBB"));
    a.resolve(png("AAAOLD"));
    await new Promise((r) => setTimeout(r, 20));
    expect(src()).toContain("BBB");
  });

  it("mantém processando… quando a prévia antiga chega antes da nova", async () => {
    const a = deferred<PreviewResult>();
    const b = deferred<PreviewResult>();
    await open();
    preview.mockReset();
    preview.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise);
    fireEvent.change(screen.getByLabelText(/nitidez/i), { target: { value: "1" } });
    await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
    fireEvent.change(screen.getByLabelText(/nitidez/i), { target: { value: "2" } });
    await waitFor(() => expect(preview).toHaveBeenCalledTimes(2));
    a.resolve(png("OLDOLD"));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.getByText("processando…")).toBeInTheDocument();
    expect(src()).not.toContain("OLDOLD");
    b.resolve(png("NEWNEW"));
    await waitFor(() => expect(src()).toContain("NEWNEW"));
    await waitFor(() => expect(screen.queryByText("processando…")).toBeNull());
  });

  it("não mostra Tamanho exato antes da prévia SVG chegar", async () => {
    estimate.mockResolvedValue({ bytes: 5000 });
    const d = deferred<SvgPreview>();
    previewSvg.mockReturnValue(d.promise);
    await open();
    await screen.findByText(/Tamanho estimado/);
    await pickSvg();
    await waitFor(() => expect(previewSvg).toHaveBeenCalled());
    expect(screen.queryByText(/^Tamanho /)).toBeNull();
    d.resolve({ svg: "<svg/>" });
    expect(await screen.findByText(/^Tamanho /)).toBeInTheDocument();
  });

  it("uma estimativa raster tardia não sobrescreve o tamanho do SVG", async () => {
    const late = deferred<{ bytes: number }>();
    await open();
    estimate.mockReset();
    estimate.mockReturnValue(late.promise);
    fireEvent.change(screen.getByLabelText(/nitidez/i), { target: { value: "1" } });
    await waitFor(() => expect(estimate).toHaveBeenCalled());
    previewSvg.mockResolvedValue({ svg: "<svg>1234</svg>" });
    await pickSvg();
    await screen.findByText(`Tamanho ${formatBytes(15)}`);
    late.resolve({ bytes: 999999 });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.getByText(`Tamanho ${formatBytes(15)}`)).toBeInTheDocument();
  });
});
