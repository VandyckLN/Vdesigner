# Roteiro manual — vetorizar para SVG

O CI não confere como o SVG aparece fora do app (navegador, Figma), nem o
tempo de uma foto grande numa máquina real. Este roteiro cobre isso. Rode-o
antes de liberar qualquer versão que mexa no vetorizador ou nos controles de
vetorização.

Anote o resultado de cada item. Um item que não deu para executar é anotado
como não executado, nunca como passou.

| # | Situação | O que deve acontecer |
|---|---|---|
| 1 | Logo em JPG com fundo branco e artefato de compressão, modo Cores 8 com Remover fundo | Poucos caminhos, fundo transparente e sem filete claro na borda. Abra o `.svg` no navegador e no Figma: os dois mostram o mesmo. |
| 2 | PNG transparente com borda suave | Nenhum anel escuro em volta das formas. |
| 3 | Assinatura fotografada com o celular em papel acinzentado, modo Uma cor | Traço limpo. Colar o código num HTML com `color: #c00` deixa a assinatura vermelha. |
| 4 | Logo branco em PNG transparente, modo Uma cor | A silhueta aparece na prévia. |
| 5 | Foto de 4000 px, modo Cores 16 | `vetorizando…` aparece, a prévia chega em até uns 40 s e a janela não congela. Arrastar Cores várias vezes seguidas não acumula espera: só a última vetorização termina. Exportar não trava a janela. |
| 6 | Abrir um `.svg` | A opção SVG (vetorizado) aparece desabilitada. |
| 7 | Mudar Cores e clicar em Copiar código SVG na hora | O botão espera a prévia nova. |
| 8 | Logo de 1000 px ou mais, exportado a 32 px de largura | O `.svg` sai com `width="32"`. Aberto no navegador com zoom de 800%, a borda continua lisa, sem degrau de miniatura. |
| 9 | Favicon de 64 a 128 px com borda suavizada, modo Cores 8 | Sem filete de cor de mistura entre duas cores vizinhas e sem halo em volta das formas. A 64 px, o contorno grosseiro é esperado. |
| 10 | Foto de uns 1500 px (como a `27.png`), modo Foto em Normal, Alto e Máximo | Escolher SVG (vetorizado) já abre em Foto (fiel), sem Cores nem Remover fundo à vista. A prévia sai fiel, sem pôster nem cor estourada, e o fundo continua. Em versão de release, por volta de 6,5 s e 4,6 MB no Normal, 8 s e 5 MB no Alto, 14 s e 5,5 MB no Máximo. |
