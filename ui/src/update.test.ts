import { describe, it, expect, vi, beforeEach } from "vitest";

const check = vi.fn();
vi.mock("@tauri-apps/plugin-updater", () => ({ check: (...a: unknown[]) => check(...a) }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));

import {
  procurarAtualizacao,
  procurarAtualizacaoManual,
  checagemAutomaticaLigada,
  definirChecagemAutomatica,
  CHAVE_CHECAGEM,
} from "./update";

describe("procurarAtualizacao", () => {
  beforeEach(() => {
    check.mockReset();
    localStorage.clear();
  });

  it("devolve a versão quando há uma disponível", async () => {
    check.mockResolvedValue({ available: true, version: "2.1.0", body: "notas" });
    expect(await procurarAtualizacao()).toMatchObject({ versao: "2.1.0", notas: "notas" });
  });

  it("devolve nulo quando já está na versão mais nova", async () => {
    check.mockResolvedValue({ available: false });
    expect(await procurarAtualizacao()).toBeNull();
  });

  it("devolve nulo quando o plugin devolve nada", async () => {
    check.mockResolvedValue(null);
    expect(await procurarAtualizacao()).toBeNull();
  });

  it("devolve nulo em falha de rede, sem lançar", async () => {
    check.mockRejectedValue(new Error("network error"));
    await expect(procurarAtualizacao()).resolves.toBeNull();
  });

  // Sem prazo, uma conexão engolida deixa a promessa pendente para sempre:
  // a faixa nunca aparece e o botão da tela Sobre fica desabilitado pelo
  // resto da vida da janela.
  it("passa um tempo limite de 10 segundos para o plugin", async () => {
    check.mockResolvedValue({ available: false });
    await procurarAtualizacao();
    expect(check).toHaveBeenCalledWith({ timeout: 10_000 });
  });

  it("trata o estouro do tempo limite como qualquer outra falha", async () => {
    check.mockRejectedValue(new Error("Request timed out"));
    await expect(procurarAtualizacao()).resolves.toBeNull();
  });
});

describe("procurarAtualizacaoManual", () => {
  beforeEach(() => {
    check.mockReset();
    localStorage.clear();
  });

  it("devolve a atualização encontrada", async () => {
    check.mockResolvedValue({ available: true, version: "2.2.0", body: "notas" });
    const resposta = await procurarAtualizacaoManual();
    expect(resposta.estado).toBe("encontrada");
    expect(resposta).toMatchObject({ atualizacao: { versao: "2.2.0" } });
  });

  it("distingue estar atualizado de não ter conseguido checar", async () => {
    check.mockResolvedValue({ available: false });
    expect(await procurarAtualizacaoManual()).toEqual({ estado: "atualizado" });

    check.mockRejectedValue(new Error("network error"));
    expect(await procurarAtualizacaoManual()).toEqual({ estado: "falhou" });
  });

  it("relata falha quando a checagem estoura o tempo limite", async () => {
    check.mockRejectedValue(new Error("Request timed out"));
    expect(await procurarAtualizacaoManual()).toEqual({ estado: "falhou" });
  });

  it("também passa o tempo limite na busca manual", async () => {
    check.mockResolvedValue({ available: false });
    await procurarAtualizacaoManual();
    expect(check).toHaveBeenCalledWith({ timeout: 10_000 });
  });
});

describe("preferência de checagem", () => {
  beforeEach(() => localStorage.clear());

  it("vem ligada quando nunca foi definida", () => {
    expect(checagemAutomaticaLigada()).toBe(true);
  });

  it("lembra que foi desligada", () => {
    definirChecagemAutomatica(false);
    expect(localStorage.getItem(CHAVE_CHECAGEM)).toBe("false");
    expect(checagemAutomaticaLigada()).toBe(false);
  });

  it("considera ligada quando o valor gravado é lixo", () => {
    localStorage.setItem(CHAVE_CHECAGEM, "talvez");
    expect(checagemAutomaticaLigada()).toBe(true);
  });
});
