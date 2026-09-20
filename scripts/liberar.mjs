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

export function montarManifesto({ tag, anexos, assinatura, notas, agora, jaLiberada }) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error(`tag fora do formato vX.Y.Z: ${tag}`);
  const versao = tag.slice(1);

  const instalador = anexos.find((a) => /_x64-setup\.exe$/.test(a.name));
  if (!instalador) throw new Error(`a release ${tag} não tem instalador anexado`);
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

  const release = JSON.parse(gh(["release", "view", tag, "-R", REPO, "--json", "assets,body"]));
  const instalador = release.assets.find((a) => /_x64-setup\.exe$/.test(a.name));
  if (!instalador) throw new Error(`a release ${tag} não tem instalador anexado`);

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
  });

  writeFileSync(MANIFESTO, `${JSON.stringify(manifesto, null, 2)}\n`);
  // Promotion is a decision, not a side effect: the person confirms it by
  // reading the diff, so the script writes the file and stops.
  console.log(`${MANIFESTO} atualizado para ${manifesto.version}. Confira o diff e commite:`);
  console.log(`  git add ${MANIFESTO} && git commit -m "release: liberar ${tag}" && git push origin main`);
}

if (process.argv[1]?.endsWith("liberar.mjs")) principal();
