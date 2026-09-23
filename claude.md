# Log de Implementação — Vdesigner: Conta-gotas

## Rodada de Correções da Tarefa 2 (2026-09-22)

### Resumo
Correção dos achados 1 a 5 da revisão da Tarefa 2:
1. **Achado 1 (ordem de captura):** Em `start_pick` (`src-tauri/src/commands.rs`), a sobreposição existente agora é fechada *antes* da captura de tela para não congelar uma janela antiga no novo retrato.
2. **Achado 2 (delegação de `resolve`):** Em `src-tauri/src/picker.rs`, `resolve` foi documentado como bloco de baixo nível e `resolve_overlay_point` delega diretamente a ele após converter as coordenadas CSS para físicas via `overlay_point_to_snapshot`.
3. **Achado 3 (teste auto-referente):** Em `src-tauri/src/screen.rs`, o auxiliar de teste `monitor_containing` foi removido; os testes agora asserem diretamente os pontos calculados no espaço DEVMODE.
4. **Achado 4 (teste de `arm` substitutivo):** Em `src-tauri/tests/picker_test.rs`, adicionado teste `it_replaces_the_previous_snapshot_when_armed_again` cobrindo a substituição de retrato anterior.
5. **Achado 5 (rejeição de NaN/infinito):** `overlay_point_to_snapshot` retorna `Result<..., ScreenError>` com variante `InvalidPoint` ao receber coordenadas não-finitas; propagado em `picker.rs` e coberto por testes em `screen.rs` e `picker_test.rs`.

### Arquivos Modificados
- `src-tauri/src/commands.rs`
- `src-tauri/src/picker.rs`
- `src-tauri/src/screen.rs`
- `src-tauri/tests/picker_test.rs`

### Desvios / Decisões Notáveis
Nenhum desvio. Conformidade total com o progress ledger e os vereditos da revisão da Tarefa 2.

### Resultados dos Testes
- Rust unit/integration tests: 108 testes passando, 0 falhas (`cargo test --all -j 2`)
- Rust lints e formatação: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- UI tests: 78 testes passando, 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)

## Tarefa 3: A janela de sobreposição (2026-09-22)

### Resumo
Implementação da janela de sobreposição (overlay) para o conta-gotas:
1. `ui/src/overlay/Overlay.tsx`: Componente com o retrato estático da tela, lupa renderizada via Canvas 2D em zoom 12x (15x15 pixels de origem), mira central de contraste alto (branco com borda preta), navegação pixel a pixel via teclas de seta, confirmação por Enter/Espaço/Clique, cancelamento com Esc ou perda de foco (`blur`).
2. `ui/src/overlay/overlay.css`: Estilização isolada dos tokens de marca, atuando como ferramenta de precisão sobre o conteúdo da tela.
3. `ui/src/main.tsx`: Roteamento condicional pelo rótulo da janela (`getCurrentWindow().label === "overlay"`), dispensando configuração multi-page no Vite.
4. `ui/src/api.ts`: Adicionados os tipos `MonitorInfo`, `OverlayGeometry`, evento `color-picked` e os métodos `startPick`, `pickAt`, `cancelPick` e `onColorPicked`.
5. `ui/src/overlay/Overlay.test.tsx`: Suíte com 5 testes automatizados cobrindo renderização da imagem congelada, cancelamento com Esc, cancelamento por blur, movimento com setas e seleção de cor no clique.

### Arquivos Criados ou Modificados
- Criados: `ui/src/overlay/Overlay.tsx`, `ui/src/overlay/overlay.css`, `ui/src/overlay/Overlay.test.tsx`
- Modificados: `ui/src/main.tsx`, `ui/src/api.ts`

### Desvios / Decisões Notáveis
- As coordenadas passadas a `api.pickAt(x, y)` são pixels CSS relativos à sobreposição, respeitando o contrato fixado em `b7d1b17` onde o backend cuida da conversão DPI e das origens de monitores em `screen.rs`.

### Resultados dos Testes
- UI tests: 83 testes passando (5 novos em Overlay.test.tsx), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 108 testes passando, 0 falhas (`cargo test --all -j 2`)
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)

## Tarefa 4: Atalho global e o aviso de atalho tomado (2026-09-22)

### Resumo
Implementação do atalho global para acionamento do conta-gotas e alerta de conflito:
1. `src-tauri/Cargo.toml` e `ui/package.json`: Adicionada dependência do plugin `tauri-plugin-global-shortcut` e seu pacote frontend.
2. `src-tauri/src/main.rs`: Registro do plugin e do atalho `Ctrl+Alt+C` no `setup`. Ao detectar a tecla pressionada (`ShortcutState::Pressed`), despacha a chamada de `commands::start_pick` para a thread principal via `run_on_main_thread`. Se o registro falhar (ex: atalho em uso por outro aplicativo), emite evento `shortcut-unavailable` para a janela principal com o nome do atalho.
3. `src-tauri/capabilities/default.json`: Concedida permissão `global-shortcut:default` para a janela principal.
4. `ui/src/api.ts`: Adicionado `onShortcutUnavailable` ouvindo o evento `shortcut-unavailable`.
5. `ui/src/App.tsx`: Estado `atalhoIndisponivel` e banner com `role="alert"` orientando o usuário a utilizar o botão alternativo caso o atalho esteja indisponível.
6. `ui/src/styles.css`: Estilização `.shortcut-warning` respeitando a linguagem visual (filete de 1px e sem preenchimento intrusivo).
7. `ui/src/App.test.tsx`: Teste unitário verificando a exibição da faixa de alerta quando o atalho global está tomado.

### Arquivos Modificados
- `src-tauri/Cargo.toml`
- `Cargo.lock`
- `src-tauri/src/main.rs`
- `src-tauri/capabilities/default.json`
- `ui/package.json`
- `ui/package-lock.json`
- `ui/src/api.ts`
- `ui/src/App.tsx`
- `ui/src/App.test.tsx`
- `ui/src/styles.css`

### Desvios / Decisões Notáveis
- Em `main.rs`, importado `tauri::Manager` para disponibilizar o método `try_state` e clonado o handle do app antes da closure de `run_on_main_thread` para satisfazer as regras de ownership e empréstimo do Rust.

### Resultados dos Testes
- UI tests: 84 testes passando (1 novo em App.test.tsx), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 108 testes passando, 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)

## Tarefa 5: A faixa de cores capturadas (2026-09-22)

### Resumo
Implementação da faixa de cores capturadas pelo conta-gotas na tela de Cores:
1. `ui/src/screens/Colors.tsx`: Estado `capturadas` mantendo até 12 cores recentes. Efeito escutando `api.onColorPicked`, desduplicando valores, posicionando o mais recente na frente e copiando automaticamente para a área de transferência (`writeText`). Botão "Capturar cor" em `.colors-controls` acionando `api.startPick()`. Renderização da seção "Capturadas" com botão "Usar" para promoção explícita (inserindo no campo de hex e permitindo nomeá-la antes de gravar na paleta do projeto).
2. `ui/src/styles.css`: Estilização flex e filetes finos de 1px para `.captured-list`.
3. `ui/src/screens/Colors.test.tsx`: Quatro novos testes cobrindo a faixa de capturadas, cópia no clipboard, garantia de que não grava automaticamente no disco/paleta, fluxo de promoção de cor capturada para a paleta com nome e acionamento da captura pelo botão.
4. `ui/src/App.test.tsx`: Atualizado mock do `api.onColorPicked` para manter o isolamento de eventos Tauri no teste de casca.

### Arquivos Modificados
- `ui/src/screens/Colors.tsx`
- `ui/src/styles.css`
- `ui/src/screens/Colors.test.tsx`
- `ui/src/App.test.tsx`

### Desvios / Decisões Notáveis
- A chamada de `writeText` foi protegida via `Promise.resolve(writeText(capturada)).catch(...)` para garantir resiliência diante de variações no retorno da API do plugin de clipboard em ambientes de teste jsdom.

### Resultados dos Testes
- UI tests: 88 testes passando (4 novos em Colors.test.tsx), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 108 testes passando, 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)

## Tarefa 6: Degradês entre cores da paleta (2026-09-22)

### Resumo
Implementação de suporte a degradês derivados de cores da paleta:
1. `crates/core/src/palette.rs`: Função pública `gradient_css(palette: &Palette, reference: &GradientRef) -> Result<String, CoreError>` gerando o valor `linear-gradient(...)` a partir da interpolação OKLCH das duas cores referenciadas.
2. `crates/core/src/lib.rs`: Reexportação de `gradient_css`.
3. `crates/core/tests/palette_test.rs`: Testes unitários para `gradient_css` cobrindo sucesso e erro quando uma cor inexistente é referenciada.
4. `src-tauri/src/commands.rs`: Comando Tauri `gradient_css(palette: Palette, nome: String) -> Result<String, String>` que localiza a referência na paleta e gera o CSS.
5. `src-tauri/src/main.rs`: Registro do comando `gradient_css` na macro `generate_handler!`.
6. `ui/src/api.ts`: Método `gradientCss` exposto no cliente `api`.
7. `ui/src/screens/Colors.tsx`: Controles para criar e listar degradês salvos na paleta, com botão para copiar a regra CSS diretamente para a área de transferência (`writeText(await api.gradientCss(...))`) e botão para remover degradê.
8. `ui/src/styles.css`: Estilização para `.gradient-list`, `.gradient-name` e layout alinhado ao design system.
9. `ui/src/screens/Colors.test.tsx`: Testes cobrindo criação de degradê, validação ao tentar submeter sem as duas pontas escolhidas, e disambiguação de seletores de botões (`/^adicionar$/i` vs "Adicionar degradê").

### Arquivos Modificados
- `crates/core/src/lib.rs`
- `crates/core/src/palette.rs`
- `crates/core/tests/palette_test.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/main.rs`
- `ui/src/api.ts`
- `ui/src/screens/Colors.tsx`
- `ui/src/screens/Colors.test.tsx`
- `ui/src/styles.css`

### Desvios / Decisões Notáveis
- O botão "Adicionar degradê" não utiliza desabilitação prévia por `palette.cores.length < 2` no HTML para permitir que cliques sem preenchimento correto acionem `addGradient()`, apresentando a mensagem de validação correspondente com `role="alert"` conforme especificado no teste.
- Seletores de teste para o botão "Adicionar" da paleta de cores foram ancorados com `/^adicionar$/i` para evitar ambiguidades com o botão "Adicionar degradê".

### Resultados dos Testes
- UI tests: 90 testes passando (2 novos em Colors.test.tsx), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 161 testes passando no workspace, 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)

## Tarefa 7: Roteiro de teste manual e documentação do conta-gotas (2026-09-22)

### Resumo
Conclusão da documentação e roteiro manual para validação das partes não testáveis em CI (captura real de múltiplos monitores com DPIs mistos, taxa de atualização e atalho global):
1. `docs/roteiro-manual-conta-gotas.md`: Criação do roteiro com os 6 cenários manuais de verificação (monitor único, monitor secundário, escala 150%, vídeo congelado, cancelamento com Esc, atalho tomado por outro aplicativo).
2. `README.md`: Atualização da lista de funcionalidades com o parágrafo "Capturar cor da tela", limitações conhecidas sobre Windows e atalho global, e atualização da contagem de testes para 267 testes (161 Rust, 90 UI, 16 scripts).
3. `CONTRIBUTING.md`: Adição de requisito de execução do roteiro manual em PRs que alterem `screen.rs`, `picker.rs` ou sobreposição, e documento da regra de fronteira de `screen.rs`.

### Arquivos Modificados
- `docs/roteiro-manual-conta-gotas.md` (criado)
- `README.md`
- `CONTRIBUTING.md`

### Desvios / Decisões Notáveis
- Nenhuma.

### Resultados dos Testes
- UI tests: 90 testes passando, 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 161 testes passando no workspace, 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)
- Total combinado: 267 testes passando.

## Rodada de Revisão Adversarial 1 (2026-09-22)

### Resumo
Identificação e correção de 6 fragilidades e defeitos na implementação do conta-gotas e degradês:
1. **Coordenadas de clique no mouse (`Overlay.tsx`):** `onClick` lia o estado assíncrono `posicao` em vez de extrair `evento.clientX`/`evento.clientY` do evento do clique, resultando em seleção de `(0, 0)` caso o clique ocorresse sem disparo prévio de `onMouseMove`.
2. **Sincronia de subpixel e arredondamento da lupa (`Overlay.tsx`):** A lupa utilizava `Math.round(posicao.x * escala)` enquanto o backend `screen.rs` utiliza `(css * scale).floor()`. Para coordenadas com fração >= 0.5 (ou displays com DPI 150%), o pixel exibido na mira divergia do pixel amostrado e resolvido pelo Rust. Ajustado para `Math.floor`.
3. **Limites e precisão da navegação por setas (`Overlay.tsx`):** Em displays escalados (ex: 200%), as setas avançavam 1 pixel CSS por toque (saltando 2 pixels físicos e ignorando pixels ímpares) e o clamper utilizava `geometria.width - 1` físico em coordenadas lógicas, permitindo que a navegação ultrapassasse o viewport e disparasse erro `OutOfBounds` no backend ao confirmar. Ajustado para passo `1 / escala` e limite `geometria.width / escala - 1`.
4. **Resiliência do aviso de atalho indisponível (`main.rs`):** O evento `shortcut-unavailable` era emitido apenas no `.setup()` antes do webview da janela principal terminar de carregar `index.html` e montar o listener do React. Adicionado re-envio em background com retentativas (200ms, 600ms, 1200ms) via `std::thread::spawn` garantindo entrega confiável sem race condition.
5. **Ocultação imediata de janela remanescente (`commands.rs`):** Em `start_pick`, chamada `existing.hide()` antes de `existing.close()` para retirar imediatamente a sobreposição anterior do compositor DWM do Windows antes da captura pelo `xcap`.
6. **Acessibilidade e testes de degradês (`Colors.tsx`, `Colors.test.tsx`):** Adicionado `aria-label={`Copiar o degradê ${degrade.nome}`}` para acessibilidade WAI-ARIA e disambiguação de múltiplos botões "Copiar" no DOM. Adicionados testes unitários cobrindo cópia de CSS do degradê via `writeText`, remoção de degradê e confirmação por clique exato e barra de espaço em `Overlay.test.tsx`.

### Arquivos Modificados
- `src-tauri/src/commands.rs`
- `src-tauri/src/main.rs`
- `ui/src/overlay/Overlay.tsx`
- `ui/src/overlay/Overlay.test.tsx`
- `ui/src/screens/Colors.tsx`
- `ui/src/screens/Colors.test.tsx`
- `README.md`
- `claude.md`

### Resultados dos Testes
- UI tests: 94 testes passando (4 novos), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 161 testes passando no workspace, 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)
- Total combinado: 271 testes passando com zero falhas e zero avisos.

## Rodada de Revisão Adversarial 2 (2026-09-22)

### Resumo
Identificação e correção de 6 defeitos críticos e de robustez na captura, ciclo de vida da sobreposição e contagem de testes:
1. **Loop recursivo / autodestruição da janela de sobreposição (`commands.rs`, `picker.rs`, `main.rs`):** Ao ser aberta, a sobreposição `<Overlay />` invoca `api.startPick()` na montagem para obter `OverlayGeometry`. Anteriormente, `start_pick` encontrava a própria janela aberta via `app.get_webview_window("overlay")` e chamava `existing.hide(); existing.close()` antes de recapturar, destruindo a janela que acabava de montar em loop contínuo. Corrigido: `Picker` armazena `OverlayGeometry` em `arm`, expõe `current_geometry()`, e `start_pick` retorna imediatamente a geometria já armada caso a sobreposição já esteja aberta. No `main.rs`, caso `Ctrl+Alt+C` seja pressionado com a sobreposição aberta, fecha e desarma explicitamente antes do novo disparo para garantir recaptura limpa.
2. **Vazamento de memória em fechamento via Alt+F4 (`main.rs`):** Ao fechar a sobreposição por comando do sistema operacional (Alt+F4 ou fechamento de janela), `picker.disarm()` não era executado, retendo o bitmap descompactado de dezenas de megabytes em memória até uma próxima captura. Corrigido: registrado listener `on_window_event` no Tauri Builder que executa `picker.disarm()` no evento `WindowEvent::Destroyed` da janela `overlay`.
3. **Inacessibilidade do pixel da borda em telas com escala de DPI > 100% (`Overlay.tsx`):** O limite `maxCssX` calculava `geometria.width / escala - 1` (subtraindo 1 pixel CSS em vez de 1 pixel físico). Em escala 200%, isso limitava a coordenada em 959.0 (pixel físico 1918), tornando inalcançável o pixel da borda 1919 (CSS 959.5). Corrigido para `(geometria.width - 1) / escala`.
4. **Travamento de seleção após clique na borda externa (`Overlay.tsx`):** Cliques na borda extrema da janela geravam `clientX * escala >= width`, disparando `OutOfBounds` no backend. A rejeição da Promise não era tratada, deixando `escolhendo.current = true` permanentemente e congelando a sobreposição. Corrigido aplicando `Math.min(Math.max(clientX, 0), maxCssX)` no clique e tratamento em `escolher` resetando `escolhendo.current = false`.
5. **Lupa em branco na montagem inicial (`Overlay.tsx`):** O efeito do canvas da lupa abortava se o elemento `<img>` ainda estivesse decodificando o base64 assincronamente (`imagem.complete === false`) e não redesenhava quando a imagem completava. Corrigido adicionando `onLoad` na tag `<img>` e estado reativo garantindo o desenho inicial da lupa.
6. **Auditoria e retificação da contagem de testes (`README.md`, `claude.md`):** Retificada a contagem de testes para os números reais medidos no workspace: 159 testes em Rust (+2 testes cobrindo `current_geometry` em `picker_test.rs`), 97 testes na interface (+3 testes cobrindo limites de alta escala, bordas e retentativas em `Overlay.test.tsx`), e 16 testes em scripts de release, totalizando 272 testes automatizados.

### Arquivos Modificados
- `src-tauri/src/picker.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/main.rs`
- `src-tauri/tests/picker_test.rs`
- `ui/src/overlay/Overlay.tsx`
- `ui/src/overlay/Overlay.test.tsx`
- `README.md`
- `claude.md`

### Resultados dos Testes
- UI tests: 97 testes passando (3 novos), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 159 testes passando no workspace (2 novos), 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)
- Total combinado: 272 testes passando com zero falhas e zero avisos.

## Rodada de Revisão Adversarial 3 (2026-09-22)

### Resumo
Identificação e correção de 2 vulnerabilidades em aritmética de subpixels e tratamento de erro de acionamento:
1. **Deriva e salto de pixels por acúmulo de ponto flutuante em escalas fracionárias (`Overlay.tsx`, `screen.rs`):** Em escalas fracionárias de DPI (ex: 150% ou 175%), navegar com teclas de seta acumulava `atual.x + 1 / escala` em ponto flutuante binário IEEE 754. Devido a imprecisões de representação, somas repetidas geravam valores como `6.999999999999999` que, sob `.floor()`, resultavam em saltos de pixel (pixel 7 pulado no avanço) ou pixels duplicados. Além disso, no backend (`screen.rs`), `(61.0 / 1.75 * 1.75)` produzia `60.99999999999999`, que sob `floor()` resolvia 60 em vez de 61. Corrigido: `Overlay.tsx` agora rastreia e incrementa o pixel físico inteiro exato `nextPx = Math.min(Math.max(curPx + delta, 0), geometria.width - 1)` e projeta `nextPx / escala`, garantindo avanço unitário e estrito a cada toque. Em `screen.rs` e no canvas da lupa de `Overlay.tsx`, adicionado epsilon `1e-6` para absorver erros de divisão fracionária sob `floor()`.
2. **Rejeição silenciosa e falha sem feedback no botão "Capturar cor" (`Colors.tsx`):** O botão "Capturar cor" executava `onClick={() => void api.startPick()}` sem captura de erros via `catch`. Caso `startPick` falhasse (ex: sem monitores ou recusa de permissão de gravação de tela pelo sistema operacional), a Promise rejeitada disparava um Unhandled Promise Rejection silencioso no console e deixava o usuário sem qualquer resposta visual ou alerta na interface. Corrigido: implementado `startPick` com `try/catch` reportando erro em `<p className="colors-error" role="alert">`.
3. **Novos testes automatizados e contagem final:**
   - Adicionado teste `it_resolves_fractional_scale_boundary_coordinates_without_underflow` em `screen.rs` (Rust: 160 testes).
   - Adicionado teste `it("avança pixel a pixel sem saltos nem duplicatas em escala fracionária (150%)")` em `Overlay.test.tsx` e teste `it("mostra alerta quando a captura falha ao clicar em Capturar cor")` em `Colors.test.tsx` (UI: 99 testes).
   - Total verificado de testes no workspace: 275 testes (160 Rust, 99 UI, 16 scripts).

### Arquivos Modificados
- `src-tauri/src/screen.rs`
- `ui/src/overlay/Overlay.tsx`
- `ui/src/overlay/Overlay.test.tsx`
- `ui/src/screens/Colors.tsx`
- `ui/src/screens/Colors.test.tsx`
- `README.md`
- `claude.md`

### Resultados dos Testes
- UI tests: 99 testes passando (2 novos), 0 falhas (`npm --prefix ui run test`)
- UI TypeScript check: limpo (`npm --prefix ui run lint`)
- Rust unit/integration tests: 160 testes passando no workspace (1 novo), 0 falhas (`cargo test --all -j 2`)
- Rust clippy e fmt: `cargo fmt --all --check` limpo, `cargo clippy --all-targets -- -D warnings` limpo
- Scripts tests: 16 testes passando, 0 falhas (`npm run test:scripts`)
- Total combinado: 275 testes passando com zero falhas e zero avisos.

