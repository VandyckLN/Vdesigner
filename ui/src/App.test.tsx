import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { api } from "./api";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn().mockResolvedValue(null) }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));
// App now mounts UpdateBanner and About, both of which reach into ./update,
// which in turn calls the Tauri updater plugin — unavailable under jsdom.
// Dubbing it here keeps the startup check a no-op so it resolves to nothing.
vi.mock("./update", () => ({
  procurarAtualizacao: () => Promise.resolve(null),
  checagemAutomaticaLigada: () => false,
  definirChecagemAutomatica: () => {},
}));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("0.0.0") }));

beforeEach(() => {
  vi.restoreAllMocks();
  vi.spyOn(api, "loadPalette").mockResolvedValue(null);
  vi.spyOn(api, "colorVariations").mockResolvedValue({
    ramp: ["#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000"],
    complementary: "#000",
    analogous: ["#000", "#000"],
    triad: ["#000", "#000"],
  });
});

describe("App", () => {
  it("liga cada aba ao seu painel com aria-controls e aria-labelledby", () => {
    render(<App />);
    const imagemTab = screen.getByRole("tab", { name: /imagem/i });
    const coresTab = screen.getByRole("tab", { name: /cores/i });
    const panel = screen.getByRole("tabpanel");

    // The visible panel belongs to the currently selected tab in both
    // directions: the tab names the panel it controls, and the panel names
    // the tab that labels it.
    expect(imagemTab).toHaveAttribute("aria-controls", panel.id);
    expect(panel).toHaveAttribute("aria-labelledby", imagemTab.id);
    expect(coresTab).toHaveAttribute("aria-controls", "panel-cores");
  });

  it("move a seleção com as setas do teclado e o foco acompanha", async () => {
    render(<App />);
    const imagemTab = screen.getByRole("tab", { name: /imagem/i });
    const coresTab = screen.getByRole("tab", { name: /cores/i });

    imagemTab.focus();
    await userEvent.keyboard("{ArrowRight}");

    expect(coresTab).toHaveAttribute("aria-selected", "true");
    expect(coresTab).toHaveFocus();
    expect(screen.getByRole("tabpanel")).toHaveAttribute("aria-labelledby", coresTab.id);

    await userEvent.keyboard("{Home}");
    expect(imagemTab).toHaveAttribute("aria-selected", "true");
    expect(imagemTab).toHaveFocus();
  });

  it("mantém o painel Sobre montado e escondido quando não é a aba ativa", () => {
    render(<App />);
    const painel = document.getElementById("panel-sobre");
    expect(painel).not.toBeNull();
    expect(painel).toHaveAttribute("hidden");
  });
});
