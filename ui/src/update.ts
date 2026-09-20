import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export const CHAVE_CHECAGEM = "vdesigner.checarAtualizacoes";

export type Atualizacao = { versao: string; notas: string; alvo: Update };

/** Outcome of a check the person asked for. A manual search has to tell
 *  "nothing new" apart from "could not find out" — rendering a failure as
 *  "you are up to date" is an affirmative lie told exactly when the network
 *  is down. The automatic path has no use for the distinction and keeps
 *  collapsing everything to `null`. */
export type ResultadoChecagem =
  | { estado: "encontrada"; atualizacao: Atualizacao }
  | { estado: "atualizado" }
  | { estado: "falhou" };

/** A blackholed connection never rejects on its own: without a deadline the
 *  promise stays pending for the life of the window, which leaves the banner
 *  invisibly stuck and the "Procurar atualizações" button disabled forever.
 *  Ten seconds is long enough for a slow link and short enough that the
 *  person notices nothing at startup. */
const TEMPO_LIMITE_MS = 10_000;

async function checar(): Promise<Atualizacao | null> {
  const encontrada = await check({ timeout: TEMPO_LIMITE_MS });
  if (!encontrada?.available) return null;
  return { versao: encontrada.version, notas: encontrada.body ?? "", alvo: encontrada };
}

/** A failed update check must never be a reason to bother someone who only
 *  wants to convert an image, so every failure path collapses to `null`:
 *  no network, no DNS, malformed manifest, GitHub down, timeout. */
export async function procurarAtualizacao(): Promise<Atualizacao | null> {
  try {
    return await checar();
  } catch {
    return null;
  }
}

/** The manual path: the person clicked and is owed an honest answer, so a
 *  failure (timeout included) reports itself instead of passing for
 *  "already up to date". */
export async function procurarAtualizacaoManual(): Promise<ResultadoChecagem> {
  try {
    const encontrada = await checar();
    return encontrada ? { estado: "encontrada", atualizacao: encontrada } : { estado: "atualizado" };
  } catch {
    return { estado: "falhou" };
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
