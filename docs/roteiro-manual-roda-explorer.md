# Roteiro manual — roda de conversão a partir do Explorer

A leitura da seleção do Explorer passa por COM (`IShellWindows` →
`IShellBrowser` → `IFolderView`). Isso não roda no CI: não existe Explorer
numa máquina de build, e não há como simular uma janela em primeiro plano com
arquivos destacados. Este roteiro é o que verifica essa parte. Rode-o antes de
liberar qualquer versão que mexa em `explorer.rs`, em `wheel.rs` ou no atalho
`Ctrl+Alt+V`.

A decisão de quais arquivos entram e quais formatos a roda oferece **não**
está aqui: está em `crates/core/src/wheel.rs` e é coberta por
`crates/core/tests/wheel_test.rs`, que roda em toda plataforma. O que este
roteiro cobre é só a ponte com o Windows.

Anote o resultado de cada item. Um item que não deu para executar é anotado
como não executado, nunca como passou.

| # | Situação | O que deve acontecer |
|---|---|---|
| 1 | Selecionar 2 PNGs no Explorer e apertar `Ctrl+Alt+V` | A roda abre centralizada mostrando `2 arquivos` e as fatias WebP, AVIF, PNG, JPG, TIFF e SVG. |
| 2 | Clicar em WebP | A roda fecha, o andamento aparece, e ao lado de cada original surge um `<nome>-convertido.webp`. Os originais continuam intactos. |
| 3 | Repetir o item 2 na mesma pasta | O segundo passe grava `<nome>-convertido-1.webp`. O arquivo do primeiro passe não é sobrescrito. |
| 4 | Selecionar 1 PNG e 1 SVG | A roda abre com 5 fatias: SVG não é oferecido, porque metade da seleção não pode virar SVG. |
| 5 | Selecionar 1 PNG e 1 `.txt` | A roda abre com `1 arquivo` e a faixa “1 arquivo fora: o Vdesigner não abre esse formato.” |
| 6 | Selecionar só arquivos `.txt` | A roda não abre; a janela principal mostra o aviso de que nada na seleção é imagem, nomeando o primeiro arquivo. |
| 7 | Apertar `Ctrl+Alt+V` com o Explorer aberto e **nada** selecionado | Aviso de que nenhum arquivo está selecionado. Nenhuma janela de roda fica aberta. |
| 8 | Apertar `Ctrl+Alt+V` com o navegador em primeiro plano | Aviso de que a janela em primeiro plano não é do Explorer. **Não** deve converter a seleção de alguma outra janela do Explorer que esteja aberta atrás. |
| 9 | Navegar pelas fatias com as setas e confirmar com `Enter` | O foco anda uma fatia por tecla, dá a volta nas pontas, e `Enter` converte no formato em foco. |
| 10 | Apertar `Esc` com a roda aberta | A roda fecha e nenhum arquivo é gravado. |
| 11 | Selecionar mais de 100 arquivos | A roda mostra 100 e a faixa avisando que a seleção passou do limite. |
| 12 | Selecionar um PNG truncado junto de 2 PNGs bons | Os 2 bons são convertidos; o andamento mostra o motivo da falha do truncado. O lote não para no primeiro erro. |
| 13 | Selecionar **só** um SVG | 5 fatias, sem SVG. Converter em PNG dá um PNG no tamanho próprio do vetor. |
| 14 | Selecionar uma foto e converter em **SVG** | Sai um `.svg` que abre no navegador idêntico à foto, e com peso parecido — é o modo Foto original, não vetorização. Não deve demorar mais que os outros formatos nem sair posterizado. |
| 15 | Selecionar um arquivo de resultado de busca do Explorer | Ou converte normalmente, ou é reportado como fora da seleção. O que não pode é a roda abrir vazia sem explicação. |
| 16 | `Ctrl+Alt+V` já tomado por outro programa | Ao abrir, o app mostra a faixa avisando qual atalho não foi registrado. |

Para o item 3, faça o item 2 duas vezes sem apagar nada entre as duas.

Para o item 12, trunque uma cópia de um PNG cortando o arquivo pela metade
(`fsutil file seteof` ou qualquer editor hexadecimal serve).

Para o item 14, o ponto é confirmar que a roda usa `TraceMode::Embed` e não o
`TraceSpec::default()`, que é `Color`. Se o SVG sair com cara de pintura, com
poucas cores ou com o fundo removido, a roda está vetorizando quando não
devia — e numa pasta com vinte fotos isso levaria minutos.

Para o item 16, registre `Ctrl+Alt+V` em outro programa antes de abrir o
Vdesigner. O PowerToys permite isso pelo Gerenciador de Teclado. Confirme
também que o `Ctrl+Alt+C` do conta-gotas continua funcionando: os dois
atalhos são registrados de forma independente, e um falhar não pode derrubar
o outro.

Para o item 8, deixe uma janela do Explorer com arquivos selecionados atrás do
navegador. O comportamento correto é recusar: converter arquivos que a pessoa
não está vendo é pior que não fazer nada.
