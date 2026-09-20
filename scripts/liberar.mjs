import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO = "VandyckLN/Vdesigner";
const MANIFESTO = "updates/stable.json";

/** Numeric comparison, never lexicographic: as text, "2.10.0" sorts below
 *  "2.9.0", which would silently allow promoting a older build over a newer. */
function partes(versao) {
  const casou = /^(\d+)\.(\d+)\.(\d+)$/.exec(versao);
  if (!casou) throw new Error(`versão fora do formato X.Y.Z: ${versao}`);
  return casou.slice(1).map(Number);
}

function menorQue(a, b) {
  const [x, y] = [partes(a), partes(b)];
  for (let i = 0; i < 3; i += 1) {
    if (x[i] !== y[i]) return x[i] < y[i];
  }
  return false;
}

// Single source of truth for "which asset is the installer for this tag".
// Both principal() (to know which .sig to download) and montarManifesto (to
// build the URL and validate) must call this exact function — two independent
// selections previously drifted apart (round 1's bug) and produced a manifest
// whose signature and URL named different binaries. Tauri's NSIS bundler
// always emits exactly "{productName}_{version}_x64-setup.exe", so an exact
// name match is both stricter and simpler than any pattern.
export function selecionarInstalador({ tag, anexos }) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error(`tag fora do formato vX.Y.Z: ${tag}`);
  const versao = tag.slice(1);
  const nomeEsperado = `Vdesigner_${versao}_x64-setup.exe`;

  const candidatos = anexos.filter((a) => a.name === nomeEsperado);
  if (candidatos.length === 0) throw new Error(`a release ${tag} não tem instalador anexado para a versão ${versao}`);
  // An exact filename match makes more than one candidate practically
  // unreachable (assets in a single release can't share a name), but this
  // guard is cheap and keeps the refusal explicit instead of silently
  // picking one by array order if that assumption is ever wrong.
  if (candidatos.length > 1) {
    throw new Error(
      `a release ${tag} tem mais de um instalador correspondente à versão ${versao}; ambíguo, recusando`
    );
  }
  return candidatos[0];
}

export function montarManifesto({ tag, anexos, assinatura, notas, agora, jaLiberada, isDraft, isPrerelease }) {
  // A draft's assets are not served from the public download URL at all, and
  // a prerelease tagged like a normal release must not reach stable users —
  // both are refused here, in the pure function, so no caller can skip them.
  if (isDraft) throw new Error(`a release ${tag} é um rascunho (draft); publique-a antes de liberar`);
  if (isPrerelease) throw new Error(`a release ${tag} é uma prerelease; não pode ser liberada como estável`);

  const instalador = selecionarInstalador({ tag, anexos });
  const versao = tag.slice(1);

  if (!anexos.some((a) => a.name === `${instalador.name}.sig`)) {
    throw new Error(`a release ${tag} não tem assinatura (.sig) anexada`);
  }
  if (!assinatura.trim()) throw new Error("a assinatura veio vazia");

  if (jaLiberada && jaLiberada !== "0.0.0" && menorQue(versao, jaLiberada)) {
    throw new Error(`${versao} é menor que a já liberada ${jaLiberada}; liberar isso seria um rebaixamento`);
  }

  return {
    version: versao,
    notes: notas,
    pub_date: agora,
    platforms: {
      "windows-x86_64": {
        signature: assinatura.trim(),
        url: `https://github.com/${REPO}/releases/download/${tag}/${instalador.name}`,
      },
    },
  };
}

function gh(args) {
  return execFileSync("gh", args, { encoding: "utf8" });
}

function principal() {
  const tag = process.argv[2];
  if (!tag) throw new Error("uso: npm run liberar -- vX.Y.Z");

  const release = JSON.parse(
    gh(["release", "view", tag, "-R", REPO, "--json", "assets,body,isDraft,isPrerelease"])
  );
  // Same selection montarManifesto will use — calling the shared function
  // here (instead of a separate ad-hoc lookup) is what keeps the downloaded
  // .sig and the manifest's URL naming the same binary.
  const instalador = selecionarInstalador({ tag, anexos: release.assets });

  const pasta = mkdtempSync(join(tmpdir(), "liberar-"));
  gh(["release", "download", tag, "-R", REPO, "-p", `${instalador.name}.sig`, "-D", pasta]);
  const assinatura = readFileSync(join(pasta, `${instalador.name}.sig`), "utf8");

  const atual = JSON.parse(readFileSync(MANIFESTO, "utf8"));
  const manifesto = montarManifesto({
    tag,
    anexos: release.assets,
    assinatura,
    notas: release.body?.split("\n")[0] ?? `Versão ${tag.slice(1)}.`,
    agora: new Date().toISOString(),
    jaLiberada: atual.version,
    isDraft: release.isDraft,
    isPrerelease: release.isPrerelease,
  });

  writeFileSync(MANIFESTO, `${JSON.stringify(manifesto, null, 2)}\n`);
  // Promotion is a decision, not a side effect: the person confirms it by
  // reading the diff, so the script writes the file and stops.
  console.log(`${MANIFESTO} atualizado para ${manifesto.version}. Confira o diff e commite:`);
  console.log(`  git add ${MANIFESTO} && git commit -m "release: liberar ${tag}" && git push origin main`);
}

if (process.argv[1]?.endsWith("liberar.mjs")) principal();
