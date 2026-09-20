import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export const CHAVE_CHECAGEM = "vdesigner.checarAtualizacoes";

export type Atualizacao = { versao: string; notas: string; alvo: Update };

/** A failed update check must never be a reason to bother someone who only
 *  wants to convert an image, so every failure path collapses to `null`:
 *  no network, no DNS, malformed manifest, GitHub down. */
export async function procurarAtualizacao(): Promise<Atualizacao | null> {
  try {
    const encontrada = await check();
    if (!encontrada?.available) return null;
    return { versao: encontrada.version, notas: encontrada.body ?? "", alvo: encontrada };
  } catch {
    return null;
  }
}

/** Downloads, verifies the signature against the embedded public key, runs the
 *  installer and restarts. Errors propagate: the banner shows them, because at
 *  this point the person clicked and is owed an answer. */
export async function instalar(
  atualizacao: Atualizacao,
  aoProgredir: (porcento: number) => void,
): Promise<void> {
  let total = 0;
  let recebido = 0;
  await atualizacao.alvo.downloadAndInstall((evento) => {
    if (evento.event === "Started") total = evento.data.contentLength ?? 0;
    if (evento.event === "Progress") {
      recebido += evento.data.chunkLength;
      if (total > 0) aoProgredir(Math.round((recebido / total) * 100));
    }
  });
  await relaunch();
}

export function checagemAutomaticaLigada(): boolean {
  try {
    return localStorage.getItem(CHAVE_CHECAGEM) !== "false";
  } catch {
    return true;
  }
}

export function definirChecagemAutomatica(ligada: boolean): void {
  try {
    localStorage.setItem(CHAVE_CHECAGEM, String(ligada));
  } catch {
    // A WebView with storage blocked still gets a working app; it just
    // forgets the preference between sessions.
  }
}
