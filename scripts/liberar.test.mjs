import { test } from "node:test";
import assert from "node:assert/strict";
import { montarManifesto, selecionarInstalador } from "./liberar.mjs";

const anexos = [{ name: "Vdesigner_2.1.0_x64-setup.exe" }, { name: "Vdesigner_2.1.0_x64-setup.exe.sig" }];
const base = { tag: "v2.1.0", anexos, assinatura: "ASSINATURA", notas: "notas", agora: "2026-09-20T15:00:00Z" };

test("monta o manifesto que o plugin espera", () => {
  const m = montarManifesto(base);
  assert.equal(m.version, "2.1.0");
  assert.equal(m.platforms["windows-x86_64"].signature, "ASSINATURA");
  assert.match(m.platforms["windows-x86_64"].url, /releases\/download\/v2\.1\.0\/Vdesigner_2\.1\.0_x64-setup\.exe$/);
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

  const instalador = selecionarInstalador({ tag: "v2.1.0", anexos: anexosComOrdemEnganosa });
  assert.equal(instalador.name, "Vdesigner_2.1.0_x64-setup.exe");

  const m = montarManifesto({
    tag: "v2.1.0",
    anexos: anexosComOrdemEnganosa,
    assinatura: "ASSINATURA-DA-2.1.0",
    notas: "notas",
    agora: "2026-09-20T15:00:00Z",
  });
  // The URL montarManifesto builds must name the same installer
  // selecionarInstalador returned — not the first asset in the array.
  assert.match(m.platforms["windows-x86_64"].url, /Vdesigner_2\.1\.0_x64-setup\.exe$/);
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
