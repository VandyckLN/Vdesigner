# Contribuindo

Este arquivo cobre o dia a dia de quem mexe no código: montar o ambiente, onde
cada coisa mora, o que um pull request precisa ter, e como sai um lançamento.

## Ambiente

Precisa de três coisas no PATH:

- **Rust 1.98**, fixado em `rust-toolchain.toml`. O mínimo do workspace é 1.85.
- **Node 20**
- **NASM** — o codificador AVIF não compila sem ele.

    npm --prefix ui ci
    cargo tauri dev

## Onde cada coisa mora

O repositório é um workspace Cargo com dois crates e uma interface:

- `crates/core` — o motor. Imagem, cor, paleta. **Puro**: não conhece Tauri,
  Windows nem WebView. O teste `crates/core/tests/purity_test.rs` lê o manifesto
  do crate e falha se `tauri`, `windows`, `winapi` ou `wry` aparecerem nele.
- `src-tauri` — a camada fina. Comandos, diálogos de arquivo, área de
  transferência. Só amarra o motor à janela.
- `ui` — React e TypeScript.

A regra que decorre disso: **lógica vai em `crates/core`, nunca no código da
janela**. Se você precisa de um teste com janela aberta para exercitar uma
regra, ela está no lugar errado.

## Convenções

**Idioma.** Testes em Rust têm nome em inglês, `snake_case`, com português
apenas nas mensagens de asserção e de erro. Testes do Vitest têm nome em
português. Comentários de documentação em inglês, explicando o *porquê* — o
*o quê* o código já diz. Texto que o usuário lê é sempre português.

**Visual.** Os tokens da marca estão em `ui/src/styles.css`. Sem sombras, sem
cantos arredondados fora do anel de foco, separação por filete de 1 px, e a cor
de acento só como estado ou medida — nunca como preenchimento.

**Acessibilidade.** Componentes interativos seguem o padrão WAI-ARIA
correspondente, com navegação por teclado funcionando. As abas, por exemplo,
implementam `tablist` completo: setas, Home, End e tabindex móvel.

## O que um pull request precisa ter

- `cargo fmt --all --check` limpo
- `cargo clippy --all-targets -- -D warnings` limpo
- `cargo test --all` e `npm --prefix ui run test` passando
- teste cobrindo o comportamento novo, no mesmo estilo dos que já existem
- lógica de imagem e de cor em `crates/core`

Para trabalho grande, abra uma issue antes descrevendo o que pretende. Para
mudança pequena, o pull request direto já serve.

Um detalhe sobre testes de cor: o jsdom não compõe camadas, então teste de
interface **não** enxerga erro de renderização de cor. Se você mexer em algo que
pinta cor, o teste que vale é em Rust, sobre os valores.

## Lançamento

Cinco atos. Nenhum é opcional.

### 1. Trabalhe numa branch

Nada vai direto para a `main`. Use `feat/nome-da-coisa` e só mescle com os
testes verdes.

### 2. Escolha o número

Semver, `MAJOR.MINOR.PATCH`:

- **PATCH** (`2.0.0` → `2.0.1`) — só correção, nada novo.
- **MINOR** (`2.0.0` → `2.1.0`) — funcionalidade nova, nada quebrou.
- **MAJOR** (`2.0.0` → `3.0.0`) — mudança incompatível, ou marco grande.

### 3. Bumpe a versão em dois arquivos

A versão vive em **dois** lugares, e os dois precisam bater:

- `Cargo.toml`, campo `version` dentro de `[workspace.package]`
- `src-tauri/tauri.conf.json`, campo `version`

`crates/core` e `src-tauri` herdam por `version.workspace = true` — nesses você
não toca. O nome do instalador sai do `tauri.conf.json`; se os dois divergirem,
o arquivo sai com um número e o aplicativo reporta outro.

Rode os testes **com o bump já aplicado**, e só então commite.

### 4. Mescle, marque e empurre

    git checkout main && git pull --ff-only origin main
    git merge --no-ff feat/sua-branch -m "feat: descricao curta"
    git tag -a vX.Y.Z -m "vX.Y.Z - descricao curta"
    git push origin main
    git push origin vX.Y.Z

O `--no-ff` força um commit de merge, então o histórico mostra onde cada
funcionalidade entrou. A tag precisa de um `git push` próprio — ela não vai
junto no push da branch.

### 5. Deixe o CI publicar

**Não compile nem anexe nada à mão.** O `.github/workflows/release.yml` dispara
sozinho no push de qualquer tag `v*`: compila em `windows-latest`, gera o
instalador NSIS, monta o zip portátil, calcula os SHA-256 em `checksums.txt` e
cria a release com os três anexados. Leva uns quinze minutos.

    gh run watch

Publicar um instalador feito na sua máquina em vez do que o CI produziu quebra
a correspondência com o `checksums.txt`, que continua sendo o do CI — quem
conferir o hash vai concluir que o arquivo foi adulterado.

O que sobra para você é escrever as notas da release depois que o CI terminar:

    gh release edit vX.Y.Z --title "vX.Y.Z - Titulo" --notes-file notas.md

Para compilar **localmente**, durante o desenvolvimento, o `tauri` não está
instalado globalmente: o binário vive em `ui/node_modules`, e o comando roda da
**raiz** do projeto, não de dentro de `ui/`.

    ./ui/node_modules/.bin/tauri build

O instalador não é assinado. O SmartScreen do Windows vai avisar quem baixar,
até que haja um certificado de assinatura de código. Vale dizer isso nas notas
de cada lançamento.
