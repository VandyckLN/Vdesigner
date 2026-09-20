import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const procurarAtualizacao = vi.fn();
const definirChecagemAutomatica = vi.fn();
const checagemAutomaticaLigada = vi.fn(() => true);
vi.mock("../update", () => ({
  procurarAtualizacao: () => procurarAtualizacao(),
  checagemAutomaticaLigada: () => checagemAutomaticaLigada(),
  definirChecagemAutomatica: (v: boolean) => definirChecagemAutomatica(v),
}));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("2.1.0") }));

import { About } from "./About";

describe("About", () => {
  beforeEach(() => {
    procurarAtualizacao.mockReset().mockResolvedValue(null);
    definirChecagemAutomatica.mockReset();
    checagemAutomaticaLigada.mockReset().mockReturnValue(true);
  });

  it("mostra a versão instalada", async () => {
    render(<About />);
    expect(await screen.findByText(/2\.1\.0/)).toBeInTheDocument();
  });

  it("diz que já está atualizado quando não há versão nova", async () => {
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    expect(await screen.findByText(/já está na versão mais recente/i)).toBeInTheDocument();
  });

  it("anuncia a versão encontrada na busca manual", async () => {
    procurarAtualizacao.mockResolvedValue({ versao: "2.2.0", notas: "", alvo: {} });
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    expect(await screen.findByText(/2\.2\.0/)).toBeInTheDocument();
  });

  it("grava a preferência ao desligar a checagem automática", async () => {
    render(<About />);
    await userEvent.click(screen.getByRole("checkbox", { name: /procurar atualizações ao abrir/i }));
    expect(definirChecagemAutomatica).toHaveBeenCalledWith(false);
  });
});
