import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Overlay } from "./Overlay";

const startPick = vi.fn();
const pickAt = vi.fn();
const cancelPick = vi.fn();

vi.mock("../api", () => ({
  api: {
    startPick: () => startPick(),
    pickAt: (x: number, y: number) => pickAt(x, y),
    cancelPick: () => cancelPick(),
  },
}));

const GEOMETRIA = {
  origin_x: 0,
  origin_y: 0,
  width: 4,
  height: 2,
  png_base64: "aGVsbG8=",
  monitors: [],
};

describe("Overlay", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    startPick.mockResolvedValue(GEOMETRIA);
    pickAt.mockResolvedValue("#ff0000");
    cancelPick.mockResolvedValue(undefined);
  });

  it("mostra o retrato congelado da tela", async () => {
    render(<Overlay />);
    const retrato = await screen.findByAltText("Retrato da tela");
    expect(retrato).toHaveAttribute("src", "data:image/png;base64,aGVsbG8=");
  });

  it("cancela com Esc sem escolher cor nenhuma", async () => {
    render(<Overlay />);
    await screen.findByAltText("Retrato da tela");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(cancelPick).toHaveBeenCalledTimes(1));
    expect(pickAt).not.toHaveBeenCalled();
  });

  it("escolhe a cor no clique, na posição do cursor", async () => {
    render(<Overlay />);
    const retrato = await screen.findByAltText("Retrato da tela");
    await userEvent.click(retrato);
    await waitFor(() => expect(pickAt).toHaveBeenCalledTimes(1));
  });

  it("move um pixel por vez com as setas do teclado", async () => {
    render(<Overlay />);
    await screen.findByAltText("Retrato da tela");
    await userEvent.keyboard("{ArrowRight}{ArrowRight}{ArrowDown}");
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(pickAt).toHaveBeenCalledWith(2, 1));
  });

  it("escolhe a cor nas coordenadas exatas do clique do mouse", async () => {
    render(<Overlay />);
    const retrato = await screen.findByAltText("Retrato da tela");
    fireEvent.click(retrato, { clientX: 3, clientY: 1 });
    await waitFor(() => expect(pickAt).toHaveBeenCalledWith(3, 1));
  });

  it("confirma a escolha também com a tecla Espaço", async () => {
    render(<Overlay />);
    await screen.findByAltText("Retrato da tela");
    await userEvent.keyboard("{ArrowRight} ");
    await waitFor(() => expect(pickAt).toHaveBeenCalledWith(1, 0));
  });

  it("cancela quando a janela perde o foco", async () => {
    render(<Overlay />);
    await screen.findByAltText("Retrato da tela");
    window.dispatchEvent(new Event("blur"));
    await waitFor(() => expect(cancelPick).toHaveBeenCalledTimes(1));
  });

  it("permite alcançar o pixel da borda direita em escala de alta densidade (200%)", async () => {
    const originalDpr = window.devicePixelRatio;
    try {
      window.devicePixelRatio = 2.0;
      render(<Overlay />);
      await screen.findByAltText("Retrato da tela");
      // GEOMETRIA width is 4 physical px. In DPR 2, valid x coordinates are 0 to 1.5 CSS px.
      // 3 presses of ArrowRight (0.5 each) reach x = 1.5 (physical pixel 3, the last pixel).
      await userEvent.keyboard("{ArrowRight}{ArrowRight}{ArrowRight}{ArrowRight}");
      await userEvent.keyboard("{Enter}");
      await waitFor(() => expect(pickAt).toHaveBeenCalledWith(1.5, 0));
    } finally {
      window.devicePixelRatio = originalDpr;
    }
  });

  it("restringe cliques na borda externa ao limite do retrato", async () => {
    render(<Overlay />);
    const retrato = await screen.findByAltText("Retrato da tela");
    // Click outside boundary (clientX: 10, clientY: 10 when max is 3, 1)
    fireEvent.click(retrato, { clientX: 10, clientY: 10 });
    await waitFor(() => expect(pickAt).toHaveBeenCalledWith(3, 1));
  });

  it("permite tentar de novo se o pickAt falhar", async () => {
    pickAt.mockRejectedValueOnce(new Error("falha de leitura"));
    render(<Overlay />);
    const retrato = await screen.findByAltText("Retrato da tela");
    fireEvent.click(retrato, { clientX: 1, clientY: 0 });
    await waitFor(() => expect(pickAt).toHaveBeenCalledTimes(1));

    // Can try again after failure without being frozen by escolhendo.current
    pickAt.mockResolvedValueOnce("#00ff00");
    fireEvent.click(retrato, { clientX: 2, clientY: 0 });
    await waitFor(() => expect(pickAt).toHaveBeenCalledTimes(2));
    expect(pickAt).toHaveBeenLastCalledWith(2, 0);
  });

  it("avança pixel a pixel sem saltos nem duplicatas em escala fracionária (150%)", async () => {
    const originalDpr = window.devicePixelRatio;
    try {
      window.devicePixelRatio = 1.5;
      startPick.mockResolvedValueOnce({
        origin_x: 0,
        origin_y: 0,
        width: 20,
        height: 10,
        png_base64: "aGVsbG8=",
        monitors: [],
      });
      render(<Overlay />);
      await screen.findByAltText("Retrato da tela");
      // 7 presses of ArrowRight at 150% DPI must land exactly on physical pixel 7 (CSS 7 / 1.5)
      for (let i = 0; i < 7; i++) {
        await userEvent.keyboard("{ArrowRight}");
      }
      await userEvent.keyboard("{Enter}");
      await waitFor(() => expect(pickAt).toHaveBeenCalledWith(7 / 1.5, 0));
    } finally {
      window.devicePixelRatio = originalDpr;
    }
  });
});
