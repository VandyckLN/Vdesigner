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
      export: vi.fn(),
    },
  };
});

const openImage = vi.mocked(api.openImage);
const preview = vi.mocked(api.preview);

describe("Editor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    preview.mockResolvedValue({ png_base64: "AAA", width: 100, height: 50 });
  });

  it("mostra o estado vazio antes de abrir uma imagem", () => {
    render(<Editor onOpenFile={vi.fn()} />);
    expect(screen.getByText(/clique em abrir imagem/i)).toBeInTheDocument();
  });

  it("mostra as dimensões depois de abrir uma imagem", async () => {
    openImage.mockResolvedValue({
      width: 1920,
      height: 1080,
      file_stem: "hero",
      preview_png_base64: "AAA",
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));

    await waitFor(() => expect(screen.getByText(/1920 × 1080/)).toBeInTheDocument());
  });

  it("desabilita o campo de altura enquanto a proporção está travada", async () => {
    openImage.mockResolvedValue({
      width: 800,
      height: 400,
      file_stem: "hero",
      preview_png_base64: "AAA",
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await waitFor(() => expect(screen.getByLabelText(/altura/i)).toBeDisabled());
  });
});
