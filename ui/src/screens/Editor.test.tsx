import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Editor } from "./Editor";
import { api } from "../api";

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    api: {
      openImage: vi.fn(),
      preview: vi.fn(),
      estimate: vi.fn(),
      export: vi.fn(),
      onExportProgress: vi.fn(),
    },
  };
});

const openImage = vi.mocked(api.openImage);
const preview = vi.mocked(api.preview);
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
      })
      .mockResolvedValueOnce({
        width: 640,
        height: 480,
        file_stem: "outra",
        preview_png_base64: "BBB",
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
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} onPickDirectory={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await waitFor(() => expect(screen.getByLabelText(/altura/i)).toBeDisabled());
  });
});
