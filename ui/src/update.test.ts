import { describe, it, expect, vi, beforeEach } from "vitest";

const check = vi.fn();
vi.mock("@tauri-apps/plugin-updater", () => ({ check: (...a: unknown[]) => check(...a) }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));

import {
  procurarAtualizacao,
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
