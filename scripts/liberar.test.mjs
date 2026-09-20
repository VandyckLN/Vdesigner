import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync as lerConfig } from "node:fs";
import { montarManifesto, selecionarInstalador, principal } from "./liberar.mjs";

const anexos = [{ name: "Vdesigner_2.1.0_x64-setup.exe" }, { name: "Vdesigner_2.1.0_x64-setup.exe.sig" }];
const base = {
  tag: "v2.1.0",
  anexos,
  assinatura: "ASSINATURA",
  notas: "notas",
  agora: "2026-09-20T15:00:00Z",
  productName: "Vdesigner",
};

// Pins montarManifesto's productName parameter to the same string that lives
// in src-tauri/tauri.conf.json (bundle.productName). A rename of the app
// there without updating this test's fixtures would otherwise go unnoticed
// while the actual installer filename it produces silently stopped matching.
test("productName usado nos testes corresponde ao configurado em tauri.conf.json", () => {
  const { productName } = JSON.parse(lerConfig(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  assert.equal(productName, base.productName);
});

test("monta o manifesto que o plugin espera", () => {
  const m = montarManifesto(base);
  assert.equal(m.version, "2.1.0");
  assert.equal(m.platforms["windows-x86_64"].signature, "ASSINATURA");
  assert.match(
    m.platforms["windows-x86_64"].url,
    /^https:\/\/github\.com\/VandyckLN\/Vdesigner\/releases\/download\/v2\.1\.0\/Vdesigner_2\.1\.0_x64-setup\.exe$/
  );
});

test("recusa uma release sem instalador", () => {
  assert.throws(() => montarManifesto({ ...base, anexos: [{ name: "checksums.txt" }] }), /instalador/);
});

test("recusa uma release sem assinatura", () => {
  assert.throws(() => montarManifesto({ ...base, assinatura: "" }), /assinatura/);
});

test("recusa uma tag fora do formato", () => {
  assert.throws(() => montarManifesto({ ...base, tag: "2.1.0" }), /formato/);
});

test("recusa liberar uma versão menor que a já liberada", () => {
  assert.throws(() => montarManifesto({ ...base, jaLiberada: "2.2.0" }), /menor|anterior/);
});

test("aceita liberar a mesma versão de novo", () => {
  assert.doesNotThrow(() => montarManifesto({ ...base, jaLiberada: "2.1.0" }));
});

// Extra: version comparison must be numeric per component, never lexicographic.
// As strings, "2.10.0" < "2.9.0", which would silently allow a downgrade to be
// promoted. This test pins the numeric behaviour so a lexicographic regression
// would be caught immediately.
test("compara versões numericamente, não como texto (2.10.0 não é menor que 2.9.0)", () => {
  assert.doesNotThrow(() =>
    montarManifesto({
      ...base,
      tag: "v2.10.0",
      anexos: [{ name: "Vdesigner_2.10.0_x64-setup.exe" }, { name: "Vdesigner_2.10.0_x64-setup.exe.sig" }],
      jaLiberada: "2.9.0",
    })
  );
});

// Extra: an installer without a matching .sig must be refused even when a
// different, unrelated .sig file is present in the assets list — the .sig
// check must match the installer's own name, not just "some .sig exists".
test("recusa quando o .sig presente não corresponde ao instalador", () => {
  assert.throws(
    () =>
      montarManifesto({
        ...base,
        anexos: [{ name: "Vdesigner_2.1.0_x64-setup.exe" }, { name: "Vdesigner_9.9.9_x64-setup.exe.sig" }],
      }),
    /assinatura/
  );
});

// Fix round 1, finding 2: the installer's filename must name the version
// being promoted. Without this, a stale binary left over from another tag
// gets shipped under a tag it does not match, with a valid signature attached.
test("recusa quando o instalador não corresponde à versão da tag", () => {
  assert.throws(
    () =>
      montarManifesto({
        ...base,
        tag: "v2.1.0",
        anexos: [{ name: "Vdesigner_9.9.9_x64-setup.exe" }, { name: "Vdesigner_9.9.9_x64-setup.exe.sig" }],
      }),
    /instalador/
  );
});

// Fix round 2, residual gap: the matcher used to accept -beta/-rc suffixes or
// a version substring buried inside a longer name because "-" and "_" counted
// as boundaries. Tauri's NSIS bundler only ever emits the exact filename
// "Vdesigner_<versao>_x64-setup.exe", so anything else must be refused.
test("recusa nomes parecidos mas não exatos (pre-release, substring, sufixo)", () => {
  for (const nomeInstalador of [
    "Vdesigner_2.1.0-beta_x64-setup.exe",
    "Vdesigner_2.1.0-rc.1_x64-setup.exe",
    "Vdesigner_3.0.0_base-2.1.0_x64-setup.exe",
    "Vdesigner_2.1.0_x64-setup-old.exe",
  ]) {
    assert.throws(
      () =>
        montarManifesto({
          ...base,
          tag: "v2.1.0",
          anexos: [{ name: nomeInstalador }, { name: `${nomeInstalador}.sig` }],
        }),
      /instalador/,
      `deveria recusar: ${nomeInstalador}`
    );
  }
});

// Fix round 2: principal() downloads the .sig for whatever selecionarInstalador
// picks, and montarManifesto (via the same function) builds the URL from it.
// Before this fix the two call sites picked installers independently and could
// disagree, shipping a manifest whose signature and URL named different
// binaries. Pinning that selecionarInstalador is the one function both paths
// call — and that it picks the version-matched asset even when a mismatched
// one appears earlier in the array — is what keeps them from drifting apart
// again.
test("selecionarInstalador é a única autoridade e nunca diverge por ordem do array", () => {
  const anexosComOrdemEnganosa = [
    { name: "Vdesigner_9.9.9_x64-setup.exe" },
    { name: "Vdesigner_9.9.9_x64-setup.exe.sig" },
    { name: "Vdesigner_2.1.0_x64-setup.exe" },
    { name: "Vdesigner_2.1.0_x64-setup.exe.sig" },
  ];

  const instalador = selecionarInstalador({ tag: "v2.1.0", anexos: anexosComOrdemEnganosa, productName: "Vdesigner" });
  assert.equal(instalador.name, "Vdesigner_2.1.0_x64-setup.exe");

  const m = montarManifesto({
    tag: "v2.1.0",
    anexos: anexosComOrdemEnganosa,
    assinatura: "ASSINATURA-DA-2.1.0",
    notas: "notas",
    agora: "2026-09-20T15:00:00Z",
    productName: "Vdesigner",
  });
  // The URL montarManifesto builds must name the same installer
  // selecionarInstalador returned — not the first asset in the array — and
  // must still point at this repository's own releases, not just any URL
  // that happens to end in the right filename.
  assert.match(
    m.platforms["windows-x86_64"].url,
    /^https:\/\/github\.com\/VandyckLN\/Vdesigner\/releases\/download\/v2\.1\.0\/Vdesigner_2\.1\.0_x64-setup\.exe$/
  );
});

// Fix round 3: the test above only calls selecionarInstalador and
// montarManifesto directly — but montarManifesto calls selecionarInstalador
// internally, so that assertion is tautological and never exercises
// principal(), which is where round 1's actual bug lived (a separate,
// unfiltered lookup deciding which .sig to download). This test drives the
// whole promotion path through principal() with fakes for gh and the
// filesystem — no process spawned, no disk touched — and checks that the
// .sig requested for download is the .sig of the installer that ends up
// named in the manifest's URL. A reintroduced open-coded lookup in
// principal() (picking the first "_x64-setup.exe" asset by array order)
// would make this fail: it would request the 9.9.9 .sig while the manifest
// still names the 2.1.0 installer.
test("principal() baixa o .sig do mesmo instalador que acaba no manifesto (ponta a ponta, com fakes)", () => {
  // Deliberately mismatched order: a non-matching installer first, the
  // correct one second — this is exactly the shape that fooled the old
  // unfiltered `.find()` in principal().
  const anexosDaRelease = [
    { name: "Vdesigner_9.9.9_x64-setup.exe" },
    { name: "Vdesigner_9.9.9_x64-setup.exe.sig" },
    { name: "Vdesigner_2.1.0_x64-setup.exe" },
    { name: "Vdesigner_2.1.0_x64-setup.exe.sig" },
  ];

  const chamadasGh = [];
  let sigSolicitado = null;

  function executarGhFalso(args) {
    chamadasGh.push(args);
    if (args[1] === "view") {
      return JSON.stringify({ assets: anexosDaRelease, body: "notas de teste", isDraft: false, isPrerelease: false });
    }
    if (args[1] === "download") {
      sigSolicitado = args[args.indexOf("-p") + 1];
      return "";
    }
    throw new Error(`gh falso não sabe responder a: ${args.join(" ")}`);
  }

  const escritos = [];
  function lerFalso(caminho) {
    if (caminho === "updates/stable.json") {
      return JSON.stringify({ version: "0.0.0", notes: "", pub_date: "2026-01-01T00:00:00Z", platforms: {} });
    }
    if (caminho === "src-tauri/tauri.conf.json") {
      return JSON.stringify({ productName: "Vdesigner" });
    }
    // Any other read is the downloaded .sig's contents — its own path is not
    // meaningful here since criarPastaTemp is also faked below.
    return "CONTEUDO-DA-ASSINATURA";
  }
  function escreverFalso(caminho, conteudo) {
    escritos.push({ caminho, conteudo });
  }

  principal({
    tag: "v2.1.0",
    executarGh: executarGhFalso,
    ler: lerFalso,
    escrever: escreverFalso,
    criarPastaTemp: () => "PASTA-FALSA",
    agora: () => "2026-09-20T15:00:00Z",
    log: () => {},
  });

  assert.ok(sigSolicitado, "principal() deveria ter pedido um .sig ao gh falso");
  assert.equal(escritos.length, 1);
  const manifesto = JSON.parse(escritos[0].conteudo);
  const instaladorNaUrl = manifesto.platforms["windows-x86_64"].url.split("/").pop();

  // The core assertion: the .sig requested for download must belong to the
  // exact same installer the manifest's URL ends up naming.
  assert.equal(sigSolicitado, `${instaladorNaUrl}.sig`);
  assert.equal(instaladorNaUrl, "Vdesigner_2.1.0_x64-setup.exe");
});

// Fix round 1, finding 3: a draft's assets aren't served from the public
// download URL, so a draft manifest would 404 on every installed copy.
test("recusa uma release que é rascunho (draft)", () => {
  assert.throws(() => montarManifesto({ ...base, isDraft: true }), /rascunho|draft/);
});

// Fix round 1, finding 3: a prerelease tagged like a normal release must not
// reach stable users just because its tag looks like vX.Y.Z.
test("recusa uma release que é prerelease", () => {
  assert.throws(() => montarManifesto({ ...base, isPrerelease: true }), /prerelease/);
});

// Fix round 1, finding 4 (kept exact-name-aware in round 2): more than one
// asset with the exact expected installer name is ambiguous — refuse
// explicitly rather than silently picking the first by array order. An exact
// name match makes duplicates practically unreachable in a real release, but
// the guard is cheap and this pins that it still fires if that ever changes.
test("recusa quando mais de um instalador corresponde à versão", () => {
  assert.throws(
    () =>
      montarManifesto({
        ...base,
        tag: "v2.1.0",
        anexos: [
          { name: "Vdesigner_2.1.0_x64-setup.exe" },
          { name: "Vdesigner_2.1.0_x64-setup.exe.sig" },
          { name: "Vdesigner_2.1.0_x64-setup.exe" },
        ],
      }),
    /mais de um|ambígu/
  );
});
