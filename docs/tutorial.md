# Tutorial — como tirar proveito do Vdesigner

Para quem desenha interface ou escreve front end. Cada seção responde a uma
pergunta: o que você quer fazer e como fazer.

O Vdesigner tem duas telas: **Imagem** e **Cores**. Tudo roda na sua máquina.
Nenhuma imagem sai dela.

---

## Imagem

### Converter para um formato mais leve

1. Abra a imagem. Entram JPG, PNG, WebP, GIF, BMP, TIFF e SVG.
2. Escolha o formato de saída: WebP, AVIF, PNG, JPG, TIFF ou ICO.
3. Veja na faixa de dados a estimativa de tamanho do arquivo final. Ajuste a
   qualidade até o número servir e só então exporte.

Para web, comece por **WebP**. Ele é aceito em todo navegador atual e costuma
cortar bastante do peso de um JPG. O **AVIF** comprime mais, mas leva mais
tempo para exportar. Vale para imagens grandes de destaque, não para ícones.
Use o **PNG** quando precisar de transparência sem perda nenhuma. O **ICO** é
para favicon.

### Redimensionar sem distorcer

Informe a largura e a altura do alvo e escolha o modo:

- **Contain**: a imagem inteira cabe no alvo e a sobra é preenchida. Nada é
  cortado. Serve para logo e produto.
- **Cover**: a imagem preenche o alvo e o excedente é cortado a partir do
  centro. Serve para capa, banner e miniatura de card.
- **Stretch**: ignora a proporção e deforma. Fica travado até você destravar,
  porque quase nunca é o que você quer.

O limite é de 16384 px por lado. A reamostragem é Lanczos3, que mantém a
nitidez ao reduzir.

Dica de front end: para telas de alta densidade, exporte com o dobro do
tamanho em que a imagem aparece no CSS. Um card de 400 px pede um arquivo de
800 px.

### Melhorar uma imagem fraca

Os três filtros são independentes, e a prévia mostra o efeito ao vivo:

- **Nitidez**, de 0 a 5, com raio próprio. Ajuda depois de reduzir uma foto.
  Passar de 2 costuma criar contorno visível.
- **Redução de ruído**, por mediana. Serve para foto escura ou print
  comprimido. Em excesso, apaga detalhe fino.
- **Brilho, contraste e saturação**, de −1 a 1. Zero deixa como está.

### Transformar SVG em bitmap

Abra o SVG e exporte no tamanho que precisar. Como o vetor é desenhado na
dimensão final, ele não perde definição ao aumentar. É útil para gerar PNG de
ícone em vários tamanhos ou imagem de compartilhamento a partir de um vetor.

Limitação: texto dentro do SVG (`<text>`) ainda não é desenhado. Converta o
texto em contorno no seu editor antes de trazer o arquivo.

---

## Cores

### Montar a paleta do projeto

1. Digite uma cor em hex, com ou sem `#`.
2. Veja a escala tonal de 50 a 900 e as harmonias: análogas, tríade e
   complementar.
3. Clique em qualquer tom para copiar em hex, RGB, HSL ou OKLCH.
4. Dê um nome à cor. Valem letras minúsculas, números e hífen: `primaria`,
   `cinza-texto`.
5. Escolha se ela entra como **escala** (os 10 tons) ou como **valor único**.
6. Escolha a pasta do projeto e grave.

### Onde a paleta fica salva

Na pasta do projeto que você escolheu, o Vdesigner grava dois arquivos:

- **`vdesigner-cores.json`**: a paleta em si. Reabrir a mesma pasta no
  Vdesigner traz tudo de volta.
- **`cores.css`**: as variáveis prontas para usar.

Uma cor em escala vira dez variáveis. Uma cor de valor único vira uma só:

```css
:root {
  --primaria-50: #...;
  --primaria-100: #...;
  /* ... até */
  --primaria-900: #...;
  --destaque: #...;
}
```

No seu código:

```css
@import "./cores.css";

.botao { background: var(--primaria-500); color: var(--primaria-50); }
```

Os dois arquivos ficam dentro do projeto de propósito. Assim entram no Git
junto com o código, e quem trabalha com você recebe a mesma paleta.

### Pegar uma cor de qualquer lugar da tela

1. Aperte **`Ctrl+Alt+C`**, ou clique em **Capturar cor** na tela de Cores.
2. A tela congela e aparece uma lupa. Aponte o pixel. As **setas** do teclado
   movem um pixel por vez, para acertar borda fina.
3. Clique, ou aperte **Enter** ou **Espaço**. **Esc** cancela sem mexer na área
   de transferência.

Funciona sobre qualquer programa, em qualquer monitor: um site aberto, um
mockup no Figma, um vídeo pausado ou não.

### Para onde vai a cor capturada

Aqui é onde mais gente se confunde:

1. **Para a área de transferência, na hora.** Basta colar no editor de código
   ou no Figma. Sai em hex.
2. **Para a faixa "Capturadas", na tela de Cores.** Ela guarda as 12 mais
   recentes, com a última na frente e sem repetir.
3. **Para o disco, só se você mandar.** A faixa vive na memória e **se apaga
   quando você fecha o Vdesigner**. Para guardar uma cor de verdade, clique em
   **Usar** ao lado dela, dê um nome e grave na paleta. Aí ela vai para o
   `vdesigner-cores.json` e para o `cores.css` da pasta do projeto.

A captura não grava sozinha para não encher a paleta do projeto de cores que
você só queria conferir.

### Criar degradês

Com pelo menos duas cores na paleta, escolha as duas pontas, dê um nome ao
degradê e adicione. O botão **Copiar** leva a regra CSS pronta. No
`cores.css`, ele vira uma variável:

```css
--hero: linear-gradient(90deg, #... , #...);
```

A mistura é calculada em OKLCH. Na prática, o meio do degradê não fica
acinzentado, como costuma acontecer ao misturar direto em RGB.

---

## Fluxos que funcionam bem

**Designer entregando para o front end.** Monte a paleta, grave na pasta do
repositório e faça commit do `cores.css`. Quem implementa usa as variáveis
direto, sem copiar hex de um print.

**Front end copiando uma cor de uma referência.** Aperte `Ctrl+Alt+C`, aponte
e cole. Se a cor for ficar no projeto, use **Usar** antes de fechar o app.

**Otimizar as imagens de uma página.** Para cada imagem: abra, aplique
**Cover** no tamanho do componente (em dobro para telas de alta densidade),
exporte em WebP e confira a estimativa antes de exportar.

---

## O que ainda não existe

- Processamento em lote. É uma imagem por vez.
- Histórico das cores capturadas depois de fechar o app.
- Trocar o atalho `Ctrl+Alt+C`. Se outro programa já usa essa combinação, o
  Vdesigner avisa, e o botão **Capturar cor** continua funcionando.
- Atalho com o app fechado. Não há ícone de bandeja nem início junto com o
  Windows.
- Versão para Linux e macOS. Hoje é só Windows.
