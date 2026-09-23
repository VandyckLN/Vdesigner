import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { api } from "./api";
import { procurarAtualizacao, checagemAutomaticaLigada } from "./update";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn().mockResolvedValue(null) }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));
// App now mounts UpdateBanner and About, both of which reach into ./update,
// which in turn calls the Tauri updater plugin — unavailable under jsdom.
// Dubbing it here keeps the startup check a no-op by default, but as `vi.fn()`
// (not fixed arrow functions) so individual tests can override the return
// value to make the banner actually render and assert on it.
vi.mock("./update", () => ({
  procurarAtualizacao: vi.fn(),
  checagemAutomaticaLigada: vi.fn(),
  definirChecagemAutomatica: vi.fn(),
}));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("0.0.0") }));

let aoAtalhoIndisponivel: ((atalho: string) => void) | null = null;
let aoFalharCaptura: ((mensagem: string) => void) | null = null;
function emitirAtalhoIndisponivel(atalho: string) {
  aoAtalhoIndisponivel?.(atalho);
}

beforeEach(() => {
  vi.restoreAllMocks();
  aoAtalhoIndisponivel = null;
  vi.spyOn(api, "onShortcutUnavailable").mockImplementation((handler) => {
    aoAtalhoIndisponivel = handler;
    return Promise.resolve(() => {});
  });
  aoFalharCaptura = null;
  vi.spyOn(api, "onPickFailed").mockImplementation((handler) => {
    aoFalharCaptura = handler;
    return Promise.resolve(() => {});
  });
  vi.spyOn(api, "onColorPicked").mockResolvedValue(() => {});
  vi.spyOn(api, "loadPalette").mockResolvedValue(null);
  vi.spyOn(api, "colorVariations").mockResolvedValue({
    ramp: ["#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000", "#000"],
    complementary: "#000",
    analogous: ["#000", "#000"],
    triad: ["#000", "#000"],
  });
  // Default: no update available, so the banner and About's effects resolve
  // to nothing observable — the same no-op baseline every other test in this
  // file relies on. Individual tests override these before rendering.
  vi.mocked(procurarAtualizacao).mockResolvedValue(null);
  vi.mocked(checagemAutomaticaLigada).mockReturnValue(false);
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

  it("mostra a faixa de atualização como primeiro filho de .app-shell quando há versão nova", async () => {
    vi.mocked(checagemAutomaticaLigada).mockReturnValue(true);
    vi.mocked(procurarAtualizacao).mockResolvedValue({ versao: "9.9.9", notas: "", alvo: {} as never });

    render(<App />);

    const faixa = await screen.findByRole("status");
    expect(faixa).toHaveTextContent("9.9.9");
    // `.update-banner`'s `grid-column: 1 / -1` only spans the top row of the
    // app-shell grid when the banner is the FIRST child — presence alone
    // would not catch it being moved below the sidebar.
    const shell = faixa.closest(".app-shell");
    expect(shell?.firstElementChild).toBe(faixa);
  });

  it("avisa quando o atalho global já está tomado por outro programa", async () => {
    render(<App />);
    emitirAtalhoIndisponivel("Ctrl+Alt+C");
    expect(await screen.findByRole("alert")).toHaveTextContent("Ctrl+Alt+C");
  });

  it("avisa quando a captura pelo atalho falha", async () => {
    render(<App />);
    aoFalharCaptura?.("não há monitores");
    expect(await screen.findByRole("alert")).toHaveTextContent("não há monitores");
  });
});
