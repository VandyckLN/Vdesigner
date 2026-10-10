import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const procurarAtualizacaoManual = vi.fn();
const instalar = vi.fn();
const definirChecagemAutomatica = vi.fn();
const checagemAutomaticaLigada = vi.fn(() => true);
vi.mock("../update", () => ({
  procurarAtualizacaoManual: () => procurarAtualizacaoManual(),
  instalar: (...a: unknown[]) => instalar(...a),
  checagemAutomaticaLigada: () => checagemAutomaticaLigada(),
  definirChecagemAutomatica: (v: boolean) => definirChecagemAutomatica(v),
}));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("2.1.0") }));

import { About } from "./About";

const encontrada = { estado: "encontrada", atualizacao: { versao: "2.2.0", notas: "", alvo: {} } };

describe("About", () => {
  beforeEach(() => {
    procurarAtualizacaoManual.mockReset().mockResolvedValue({ estado: "atualizado" });
    instalar.mockReset().mockResolvedValue(undefined);
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
    procurarAtualizacaoManual.mockResolvedValue(encontrada);
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    expect(await screen.findByText(/2\.2\.0 disponível/)).toBeInTheDocument();
  });

  // Uma checagem que falhou não é "você está atualizado": com a rede fora,
  // dizer isso seria afirmar algo falso justamente sobre o que a pessoa
  // acabou de mandar verificar.
  it("não finge sucesso quando a checagem falha", async () => {
    procurarAtualizacaoManual.mockResolvedValue({ estado: "falhou" });
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    expect(await screen.findByText(/não foi possível verificar/i)).toBeInTheDocument();
    expect(screen.queryByText(/já está na versão mais recente/i)).not.toBeInTheDocument();
  });

  // A faixa é a única outra coisa que instala, e ela nem procura quando a
  // checagem automática está desligada — sem este botão, quem desmarcou a
  // opção e buscou à mão nunca conseguiria atualizar.
  it("instala a atualização encontrada na busca manual", async () => {
    procurarAtualizacaoManual.mockResolvedValue(encontrada);
    checagemAutomaticaLigada.mockReturnValue(false);
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    await userEvent.click(await screen.findByRole("button", { name: "Atualizar agora" }));
    await waitFor(() => expect(instalar).toHaveBeenCalledTimes(1));
    expect(instalar.mock.calls[0][0]).toMatchObject({ versao: "2.2.0" });
  });

  it("não oferece instalar quando não há atualização", async () => {
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    await screen.findByText(/já está na versão mais recente/i);
    expect(screen.queryByRole("button", { name: "Atualizar agora" })).not.toBeInTheDocument();
  });

  it("mostra o motivo e mantém o botão utilizável quando a instalação falha", async () => {
    procurarAtualizacaoManual.mockResolvedValue(encontrada);
    instalar.mockRejectedValue(new Error("assinatura inválida"));
    render(<About />);
    await userEvent.click(screen.getByRole("button", { name: "Procurar atualizações" }));
    await userEvent.click(await screen.findByRole("button", { name: "Atualizar agora" }));
    expect(await screen.findByText(/assinatura inválida/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Atualizar agora" })).toBeEnabled();
  });

  it("grava a preferência ao desligar a checagem automática", async () => {
    render(<About />);
    await userEvent.click(screen.getByRole("checkbox", { name: /procurar atualizações ao abrir/i }));
    expect(definirChecagemAutomatica).toHaveBeenCalledWith(false);
  });

  describe("guia de uso", () => {
    it("começa fechado", () => {
      render(<About />);
      expect(screen.getByRole("button", { name: "Como usar" })).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByRole("region", { name: "Como usar o Vdesigner" })).not.toBeInTheDocument();
    });

    it("abre com uma seção por função e fecha de novo", async () => {
      render(<About />);
      await userEvent.click(screen.getByRole("button", { name: "Como usar" }));

      const guia = screen.getByRole("region", { name: "Como usar o Vdesigner" });
      for (const titulo of ["Imagem", "Cores", "Atalhos", "O que ainda não existe"]) {
        expect(within(guia).getByRole("heading", { name: titulo })).toBeInTheDocument();
      }
      // A pergunta que mais aparece: para onde vai a cor capturada.
      expect(within(guia).getByText(/somem ao fechar/i)).toBeInTheDocument();

      const fechar = screen.getByRole("button", { name: "Fechar guia" });
      expect(fechar).toHaveAttribute("aria-expanded", "true");
      await userEvent.click(fechar);
      expect(screen.queryByRole("region", { name: "Como usar o Vdesigner" })).not.toBeInTheDocument();
    });
  });
});
