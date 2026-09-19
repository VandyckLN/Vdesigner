# Vdesigner

Ferramenta de imagem para quem trabalha com interface. Converte, redimensiona
sem distorção e melhora a qualidade de imagens numa única janela, com prévia ao
vivo e estimativa de tamanho antes de exportar.

A ideia é resolver num lugar só o que hoje se faz espalhado: um site para
converter, outro para comprimir, um editor pesado para redimensionar, e nenhum
deles dizendo quanto o arquivo vai pesar no fim. Tarefa de todo dia de quem
desenvolve front end e de quem desenha — rápida, local e sem upload.

## O que ele faz

**Converter.** Entre os formatos da tabela abaixo, com controle de qualidade e
modo sem perdas onde o formato suporta.

**Redimensionar.** Reamostragem Lanczos3 em três modos: `Contain` cabe dentro do
alvo e preenche a sobra sem nunca cortar, `Cover` preenche e corta o excedente
pelo centro, `Stretch` ignora a proporção e só fica acessível quando você
destrava. Limite de 16384 px por lado.

**Melhorar.** Três filtros independentes: nitidez por máscara de desfoque
(intensidade 0 a 5 e raio próprio), redução de ruído por mediana, e ajuste de
brilho, contraste e saturação de −1 a 1.

**Rasterizar SVG.** Vetor entra e sai como bitmap em qualquer dimensão, sem
perder definição na subida.

**Prever antes de gastar.** A prévia recalcula ao vivo enquanto você mexe nos
controles, e a faixa de dados mostra a estimativa de tamanho do arquivo final.
Exportações longas reportam progresso por etapa.

## Formatos

| Entrada | Saída |
|---|---|
| JPG, PNG, WebP, GIF, BMP, TIFF, SVG | WebP, AVIF, PNG, JPG, TIFF, ICO |

## Tecnologias

| Camada | O que é usado |
|---|---|
| Motor de imagem | Rust — `image`, `fast_image_resize` (Lanczos3), `imageproc`, `resvg` |
| Codificadores | `webp`, `ravif` (AVIF, com rotinas em assembly via NASM) |
| Aplicação | Tauri 2 — janela nativa sobre o WebView2 do Windows |
| Interface | React 18, TypeScript, Vite |
| Testes | 82 no total — 64 em Rust (`cargo test`) e 18 na interface (Vitest, Testing Library) |
| CI | GitHub Actions — `rustfmt`, `clippy` com avisos como erro, e as duas suítes |

O motor vive em `crates/core` e é uma biblioteca pura: não conhece Tauri, não
conhece Windows e não conhece interface nenhuma. Toda operação é descrita como
dado — um `Job` com uma lista de passos e uma especificação de saída — e o
motor não distingue um pedido vindo da tela de um pedido vindo de um script.
Essa separação é o que deixa o projeto crescer sem embolar.

O executável fica em torno de 20 MB porque o Tauri reaproveita o WebView2 que já
existe no Windows, em vez de embutir um navegador inteiro.

## Instalação

Baixe o instalador `.msi` mais recente em Releases.

O Windows SmartScreen exibe um aviso na primeira execução, porque o aplicativo
ainda não tem assinatura de código. Clique em "Mais informações" e depois em
"Executar assim mesmo". O checksum SHA-256 de cada release é publicado junto do
arquivo, e permite conferir que o download não foi alterado.

Quem não pode instalar nada na máquina pode usar o pacote `.zip`, que roda
direto da pasta — inclusive de um pen drive. Ele conta com o Microsoft Edge
WebView2 Runtime já presente no sistema, o que vale para o Windows 11 e para a
maioria das instalações atualizadas do Windows 10; o instalador `.msi` é o
caminho para as máquinas que não o tenham.

## Desenvolvimento

Requer Rust 1.98 (fixado em `rust-toolchain.toml`; o MSRV mínimo do workspace é
1.85), Node 20 e NASM no PATH — o codificador AVIF não compila sem ele.

    npm --prefix ui ci
    cargo tauri dev

Testes:

    cargo test --all
    npm --prefix ui run test

## Contribuindo

O projeto é aberto e aceita contribuições. A intenção é que ele sirva à
comunidade, então melhoria vinda de fora é bem-vinda: formato novo, filtro
novo, correção, tradução, documentação ou simplesmente um caso de uso que
quebrou na sua máquina.

Abra uma issue descrevendo o que pretende antes de um trabalho grande, para não
gastar seu tempo numa direção que não casa com o resto. Para mudanças pequenas,
o pull request direto já serve.

O que se espera de um pull request:

- `cargo fmt --all --check` e `cargo clippy --all-targets -- -D warnings` limpos
- `cargo test --all` e `npm --prefix ui run test` passando
- teste cobrindo o comportamento novo, no mesmo estilo dos que já existem
- lógica de imagem em `crates/core`, nunca no código da janela

## Limitações conhecidas

Elementos `<text>` de SVG ainda não são desenhados, porque o carregamento de
fontes do sistema não foi ligado. Um SVG com texto rasteriza sem ele.

O empacotamento hoje é só para Windows. O motor em si não tem nada preso à
plataforma, e o Tauri suporta Linux e macOS — falta o trabalho de build e teste.

O processamento é de uma imagem por vez. O motor já aceita lotes, porque um
`Job` é só dado, mas a tela para isso ainda não existe.

## Autoria

Criado e mantido por **Vandyck — [@VandyckLN](https://github.com/VandyckLN)**.

O projeto é aberto de propósito: a intenção é que ele ajude quem precisa e que
melhore com quem quiser contribuir. Isso não é o mesmo que ser anônimo. Quem
usar, adaptar ou distribuir o Vdesigner, inclusive dentro de produto comercial,
deve manter o crédito de autoria — a licença MIT exige que o aviso de direito
autoral acompanhe o código, e é justamente esse aviso que faz o reconhecimento
viajar junto com o trabalho.

Contribuições entram com o crédito de quem as fez, no histórico do Git e na
lista de contribuidores do repositório.

## Licença

MIT — veja [LICENSE](LICENSE). Permite uso, cópia, modificação e venda, desde
que o aviso de copyright e a licença sejam mantidos.

As licenças das dependências estão em [THIRD-PARTY.md](THIRD-PARTY.md).
