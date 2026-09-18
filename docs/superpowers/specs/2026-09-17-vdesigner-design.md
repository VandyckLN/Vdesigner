# Vdesigner — Documento de Design

Data: 2026-09-17
Status: aprovado, pronto para plano de implementação

## 1. Objetivo

Vdesigner é um aplicativo de desktop instalável, de código aberto, para
designers e desenvolvedores front end. Ele converte e otimiza imagens,
redimensiona sem distorção, melhora qualidade, e captura cores de qualquer
ponto da tela com geração de paletas prontas para uso em código.

A primeira versão atende apenas Windows.

## 2. Escopo da versão 1

### Incluído

- Conversão entre formatos de imagem.
- Redimensionamento com preservação de proporção.
- Melhoria de qualidade clássica: sharpening, denoise, ajuste de contraste e cor.
- Upscale com inteligência artificial como módulo opcional, baixado sob demanda.
- Edição de imagem única com prévia.
- Processamento em lote com fila, presets e relatório de falhas.
- Conta-gotas global de tela, com lupa e atalho de teclado.
- Geração de escalas, harmonias e gradientes a partir de uma cor.
- Verificação de contraste WCAG.
- Exportação de paletas em CSS, JSON, SCSS e ASE.

### Excluído explicitamente

PSD, geração de SVG, macOS, Linux, plugin de Figma, edição por camadas,
recorte de fundo e assinatura de código.

### Formatos

Entrada: JPG, PNG, WebP, GIF, BMP, TIFF, AVIF, HEIC e SVG. O SVG é aceito
apenas como entrada, rasterizado para o tamanho pedido.

Saída: WebP, AVIF, PNG, JPG, TIFF e ICO.

### Formatos de cor

HEX, RGB, RGBA, HSL, HSLA, HSB, HSV, OKLCH e CMYK.

## 3. Stack

Tauri 2, com o motor em Rust e a interface em React com TypeScript.

A escolha se apoia em dois pontos. O recurso central do produto, o conta-gotas
global, depende de API nativa do Windows, e em Rust isso é uma função curta em
vez de uma ponte frágil. E o instalador base fica entre 8 e 15 MB, o que importa
porque o módulo de upscale já pede cerca de 80 MB de download opcional.

## 4. Arquitetura

O projeto é um workspace Cargo com cinco crates. O motor nunca conhece a
interface. Se uma decisão sobre imagem ou sobre cor aparecer em `src-tauri` ou
em `ui/`, ela está no lugar errado.

### `vdesigner-core`

Crate pura, sem Tauri e sem chamadas ao Windows. Recebe bytes e uma descrição de
operação, devolve bytes e metadados. Reporta progresso por callback. Testável
isoladamente com `cargo test`.

Dependências principais: `image`, `fast_image_resize`, `resvg`, `ravif` e
`libheif-rs`.

### `vdesigner-color`

Crate pura, sem entrada e saída. Converte entre os formatos de cor, gera escalas
e harmonias, calcula contraste WCAG e serializa paletas.

### `vdesigner-capture`

A única crate que fala com a API Win32. Captura de pixel, lupa e atalho global.
Exposta atrás do trait `ScreenPicker`, com a implementação `WindowsPicker`. Uma
porta futura para macOS acrescenta uma implementação nova sem alterar nada acima.

### `vdesigner-upscale`

Módulo opcional. Baixa o binário e o modelo, confere o SHA-256, executa o
processo filho, lê o progresso e trata o cancelamento. O aplicativo funciona
por inteiro quando este módulo não está instalado.

### `src-tauri`

Camada de cola. Comandos Tauri, fila de lote, estado de sessão e histórico de
cores. Nenhuma lógica de imagem.

### `ui/`

React com TypeScript. Três telas: Editor, Lote e Cores.

## 5. Processamento de imagem

### Pipeline declarativo

Uma operação é uma estrutura de dados, não um caminho de código:

```
Job {
  source: caminho ou bytes,
  steps: [Resize{...}, Sharpen{...}, Denoise{...}, Upscale{...}],
  output: Encode{ formato, qualidade, metadados }
}
```

O Editor produz um `Job`. O Lote produz N `Job` a partir de um preset. O motor
não distingue a origem.

### Prévia

A prévia executa o mesmo pipeline sobre uma versão reduzida da imagem, limitada
a 2048px, para permanecer interativa. A exportação executa o pipeline de novo em
resolução cheia. Existe um único caminho de código, de modo que a prévia nunca
diverge do resultado final.

### Redimensionamento

Lanczos3 como padrão, via `fast_image_resize` com SIMD. A proporção fica travada
por padrão, e destravar é uma ação explícita do usuário.

Quando o alvo tem proporção diferente da origem, o usuário escolhe entre
`contain`, `cover` com recorte, ou preenchimento. A imagem nunca é esticada de
forma implícita.

### Upscale com IA

Implementado sobre `realesrgan-ncnn-vulkan`, executado como processo filho.

O tamanho de tile é calculado a partir da VRAM disponível, para evitar que o
processo morra no meio em imagens grandes. A ausência de Vulkan, de GPU
compatível ou do próprio módulo é detectada antes do processamento e comunicada
com instruções, não com falha bruta.

A licença de cada peso de modelo precisa ser verificada individualmente antes da
inclusão, porque nem todos seguem a licença BSD-3-Clause do código do
Real-ESRGAN.

### Fila de lote

Pool de workers com paralelismo igual ao número de núcleos menos um. Cada job
emite eventos de progresso para a interface. O cancelamento aborta os pendentes
e deixa o job em execução terminar. A falha de um arquivo não interrompe a fila:
os erros são reunidos em uma lista apresentada ao final.

### Presets

Salvos como JSON no diretório de configuração do usuário, e exportáveis como
arquivo para a equipe compartilhar a mesma configuração. Um preset é um `Job`
sem a fonte.

### Proteção contra sobrescrita

A gravação em pasta de saída separada é o padrão. Sobrescrever o original exige
que o usuário marque a opção, e a interface avisa antes.

## 6. Subsistema de cor

### Conta-gotas global

Um atalho global registrado no Windows, com padrão `Ctrl+Shift+C` e reconfigurável,
abre um overlay de tela cheia, transparente e sempre no topo. O cursor exibe uma
lupa com zoom de 8x, grade de pixels e o valor HEX ao lado. O clique captura,
`Esc` cancela, e as setas movem um pixel por vez para acertar bordas finas.

O overlay faz um `BitBlt` da tela inteira no momento da abertura e lê o pixel do
bitmap em memória. Isso evita ler o próprio overlay e funciona sobre qualquer
janela, vídeo ou jogo.

A captura cobre o retângulo virtual de todos os monitores. O aplicativo declara
reconhecimento de DPI `PerMonitorV2`, para não ler o pixel errado em telas com
escala diferente de 100%.

### Histórico

As últimas 50 cores ficam persistidas, com origem e horário. O clique copia o
valor, e o clique longo abre a cor no gerador.

### Gerador de paletas

A partir de uma cor de origem:

- Escala de 50 a 950, calculada em OKLCH para manter variação de luminosidade
  perceptualmente uniforme. Em HSL a mesma escala escureceria em degraus
  irregulares.
- Harmonias: complementar, análogas e triádicas.
- Gradiente entre duas cores, interpolado em OKLCH, com saída em
  `linear-gradient` do CSS.
- Contraste WCAG contra branco e preto, com rótulo AA ou AAA, e indicação do tom
  mais próximo da escala que atinge AA.

### Exportação

A mesma paleta em quatro alvos: CSS custom properties, JSON de tokens, SCSS e
`.ase` (Adobe Swatch). O resultado pode ir para a área de transferência ou para
um arquivo.

### Limite conhecido

CMYK sem perfil ICC é aproximação, e não cor de gráfica. O valor é exibido com
marcação de "aproximado" na interface, para que ninguém envie arquivo para
impressão confiando nele.

## 7. Tratamento de erros

**Erros esperados** — arquivo corrompido, formato não suportado, disco cheio,
permissão negada. Geram mensagem clara com o caminho do arquivo e a causa. Em
lote, alimentam a lista de falhas do final, sem parar a fila.

**Erros de ambiente** — driver Vulkan desatualizado, VRAM insuficiente, módulo
de IA ausente. São detectados antes da tentativa de processamento, com
explicação do que fazer.

**Erros inesperados** — `panic` em uma crate. Capturado na borda do comando
Tauri, convertido em erro de interface, sem derrubar o aplicativo. Registrado em
arquivo de log rotacionado no diretório de dados do usuário.

## 8. Estratégia de testes

`vdesigner-core` recebe testes por formato, testes de ida e volta de codificação,
e comparação contra imagens de referência com tolerância de diferença
perceptual. As fixtures são pequenas e versionadas no repositório.

`vdesigner-color` é matemática pura: testes de ida e volta mais valores conhecidos
conferidos contra a especificação CSS Color 4.

`vdesigner-capture` usa um `FakePicker` nos testes automatizados. A implementação
Win32 real recebe um roteiro de teste manual documentado, porque automatizar
captura de tela em integração contínua não compensa.

A fila de lote é testada com um motor falso, verificando paralelismo,
cancelamento e continuidade após falha.

## 9. Distribuição

Instalador MSI gerado pelo Tauri, mais um build portátil em `.zip` para máquinas
corporativas com restrição de instalação.

Licença MIT. Cada dependência tem sua licença registrada em `THIRD-PARTY.md`,
o que é obrigatório aqui porque `libheif` e os pesos de upscale têm termos
próprios.

As releases saem por tag no GitHub Actions, com checksum publicado.

A versão 1 não tem assinatura de código. O SmartScreen exibirá aviso nas
primeiras instalações, e isso fica documentado no README de forma direta. A
compra de certificado fica para depois, caso o uso justifique.
