# Roteiro manual — conta-gotas

Três coisas não são testáveis no CI: a captura de tela real, a sobreposição
com lupa e atalho global, e dois monitores com DPI diferentes. Este roteiro é
o que verifica essa parte. Rode-o antes de liberar qualquer versão que mexa
em `screen.rs`, `picker.rs` ou na sobreposição.

Anote o resultado de cada item. Um item que não deu para executar — máquina
com um monitor só, por exemplo — é anotado como não executado, nunca como
passou.

| # | Situação | O que deve acontecer |
|---|---|---|
| 1 | Capturar com um monitor só | A cor sob o cursor vai para a área de transferência e aparece na faixa Capturadas. |
| 2 | Capturar no monitor secundário | A mesma coisa, e a cor é a do pixel apontado — não a do pixel correspondente no monitor principal. |
| 3 | Capturar com escala de 150% | A lupa mostra o pixel sob a mira, e a cor capturada é a que a lupa mostrava. |
| 4 | Capturar sobre um vídeo rodando | A imagem congela no instante do atalho; a cor é a do quadro congelado, não a de um quadro posterior. |
| 5 | Cancelar com `Esc` | A sobreposição fecha e a área de transferência continua com o que tinha antes. |
| 6 | Atalho já tomado por outro programa | Ao abrir, o app mostra a faixa avisando, e o botão “Capturar cor” funciona. |

Para o item 6, registre `Ctrl+Alt+C` em outro programa antes de abrir o
Vdesigner. O PowerToys permite isso pelo Gerenciador de Teclado.

Para o item 4, qualquer vídeo em reprodução serve. O ponto é capturar uma cor
que muda: se o resultado for a cor de um quadro diferente do que estava na
tela ao apertar o atalho, o congelamento está quebrado.
