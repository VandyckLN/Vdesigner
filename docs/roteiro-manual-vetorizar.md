# Roteiro manual — bitmap para SVG

O CI não confere como o SVG aparece fora do app (navegador, Figma), nem o
tempo de uma foto grande numa máquina real. Este roteiro cobre isso. Rode-o
antes de liberar qualquer versão que mexa no vetorizador, na Foto original ou
nos controles de SVG.

Anote o resultado de cada item. Um item que não deu para executar é anotado
como não executado, nunca como passou.

| # | Situação | O que deve acontecer |
|---|---|---|
| 1 | Logo em JPG com fundo branco e artefato de compressão, modo Cores 8 com Remover fundo | Poucos caminhos, fundo transparente e sem filete claro na borda. Abra o `.svg` no navegador e no Figma: os dois mostram o mesmo. |
| 2 | PNG transparente com borda suave | Nenhum anel escuro em volta das formas. |
| 3 | Assinatura fotografada com o celular em papel acinzentado, modo Uma cor | Traço limpo. Colar o código num HTML com `color: #c00` deixa a assinatura vermelha. |
| 4 | Logo branco em PNG transparente, modo Uma cor | A silhueta aparece na prévia. |
| 5 | Foto de 4000 px, modo Cores 16 | `vetorizando…` aparece, a prévia chega em até uns 40 s e a janela não congela. Arrastar Cores várias vezes seguidas não acumula espera: só a última vetorização termina. Exportar não trava a janela. |
| 6 | Abrir um `.svg` | A opção SVG aparece desabilitada. |
| 7 | Mudar Cores e clicar em Copiar código SVG na hora | O botão espera a prévia nova. |
| 8 | Logo de 1000 px ou mais, exportado a 32 px de largura | O `.svg` sai com `width="32"`. Aberto no navegador com zoom de 800%, a borda continua lisa, sem degrau de miniatura. |
| 9 | Favicon de 64 a 128 px com borda suavizada, modo Cores 8 | Sem filete de cor de mistura entre duas cores vizinhas e sem halo em volta das formas. A 64 px, o contorno grosseiro é esperado. |
| 10 | Foto de uns 1500 px (como a `27.png`), modo Foto em vetor em Normal, Alto e Máximo | Aparece Detalhe, sem Cores, Remover fundo nem Compressão. A prévia sai sem pôster nem cor estourada, e o fundo continua; pele e textura fina com cara de pintura são esperadas. Em versão de release, por volta de 6 s e 4,6 MB no Normal, 8 s e 5,1 MB no Alto, 11 s e 5,5 MB no Máximo. |
| 11 | Abrir de novo a mesma foto e escolher SVG | Já abre em Foto original, sem Detalhe, Cores nem Remover fundo, e com Compressão. A prévia é igual à foto e o Tamanho mostra uns 333 KB com Compressão 82, em menos de meio segundo na versão de release; em 60, uns 225 KB. O `.svg` exportado abre com a foto no navegador, no Inkscape e no Figma. Com zoom de 400% no navegador ele perde definição, borra ou pixela, o que é esperado. Exportar com 300 px de largura sai com `width="300"`, e o Tamanho quase não muda: a foto guardada continua com 1563 px. |
| 12 | PNG com fundo transparente, Foto original | O fundo continua transparente na prévia, no navegador e no Figma. Mexer na Compressão não muda o Tamanho, porque a foto vai em PNG. |
