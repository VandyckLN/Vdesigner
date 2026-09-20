import { test } from "node:test";
import assert from "node:assert/strict";
import { montarManifesto } from "./liberar.mjs";

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
