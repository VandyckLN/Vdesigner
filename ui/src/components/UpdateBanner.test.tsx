import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const procurarAtualizacao = vi.fn();
const instalar = vi.fn();
const checagemAutomaticaLigada = vi.fn(() => true);
vi.mock("../update", () => ({
  procurarAtualizacao: () => procurarAtualizacao(),
  instalar: (...a: unknown[]) => instalar(...a),
  checagemAutomaticaLigada: () => checagemAutomaticaLigada(),
}));

import { UpdateBanner } from "./UpdateBanner";

const disponivel = { versao: "2.1.0", notas: "", alvo: {} };

describe("UpdateBanner", () => {
  beforeEach(() => {
    procurarAtualizacao.mockReset().mockResolvedValue(null);
    instalar.mockReset().mockResolvedValue(undefined);
    checagemAutomaticaLigada.mockReset().mockReturnValue(true);
  });

  it("não mostra nada quando não há versão nova", async () => {
    render(<UpdateBanner />);
    await waitFor(() => expect(procurarAtualizacao).toHaveBeenCalled());
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("anuncia a versão nova", async () => {
    procurarAtualizacao.mockResolvedValue(disponivel);
    render(<UpdateBanner />);
    expect(await screen.findByText(/2\.1\.0/)).toBeInTheDocument();
  });

  it("não procura nada quando a checagem automática está desligada", async () => {
    checagemAutomaticaLigada.mockReturnValue(false);
    render(<UpdateBanner />);
    await new Promise((r) => setTimeout(r, 0));
    expect(procurarAtualizacao).not.toHaveBeenCalled();
  });

  it("esconde a faixa em Agora não", async () => {
    procurarAtualizacao.mockResolvedValue(disponivel);
    render(<UpdateBanner />);
    await userEvent.click(await screen.findByRole("button", { name: "Agora não" }));
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("instala uma vez ao clicar em Atualizar", async () => {
    procurarAtualizacao.mockResolvedValue(disponivel);
    render(<UpdateBanner />);
    await userEvent.click(await screen.findByRole("button", { name: "Atualizar" }));
    await waitFor(() => expect(instalar).toHaveBeenCalledTimes(1));
  });

  it("mostra o erro e mantém o botão quando a instalação falha", async () => {
    procurarAtualizacao.mockResolvedValue(disponivel);
    instalar.mockRejectedValue(new Error("assinatura inválida"));
    render(<UpdateBanner />);
    await userEvent.click(await screen.findByRole("button", { name: "Atualizar" }));
    expect(await screen.findByText(/assinatura inválida/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Atualizar" })).toBeEnabled();
  });
});
