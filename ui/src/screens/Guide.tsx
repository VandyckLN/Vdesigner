/** Guia curto de uso, aberto pela tela Sobre. Texto fixo, no mesmo registro
 *  do LEIAME: direto, com as ressalvas onde a pessoa tropeça. A versão longa
 *  vive em docs/tutorial.md; o que mudar aqui deve mudar lá também. */
export function Guide() {
  return (
    <section className="guide" aria-label="Como usar o Vdesigner">
      <h3>Imagem</h3>
      <dl>
        <dt>Converter</dt>
        <dd>
          Abra a imagem e escolha o formato. WebP para a web em geral, AVIF quando o peso importa
          mais que o tempo de exportar, PNG para transparência sem perda, ICO para favicon.
        </dd>
        <dt>Redimensionar</dt>
        <dd>
          Contain cabe no alvo e preenche a sobra, sem cortar. Cover preenche e corta o excesso pelo
          centro. Stretch deforma e fica travado até você destravar. Para telas de alta densidade,
          exporte com o dobro do tamanho que a imagem ocupa no CSS.
        </dd>
        <dt>Melhorar</dt>
        <dd>
          Nitidez para foto reduzida (acima de 2 costuma criar contorno), redução de ruído para
          foto escura ou print comprimido, e brilho, contraste e saturação de −1 a 1.
        </dd>
        <dt>SVG</dt>
        <dd>
          Entra como vetor e sai como bitmap em qualquer tamanho. Texto dentro do SVG ainda não é
          desenhado: converta em contorno antes.
        </dd>
        <dt>Prévia e tamanho</dt>
        <dd>
          A prévia muda enquanto você mexe, e a estimativa mostra quanto o arquivo vai pesar.
          Ajuste a qualidade até o número servir, e só então exporte.
        </dd>
      </dl>

      <h3>Cores</h3>
      <dl>
        <dt>Paleta</dt>
        <dd>
          Digite um hex e veja a escala de 50 a 900 e as harmonias. Clique num tom para copiar em
          hex, RGB, HSL ou OKLCH. Dê um nome (minúsculas, números e hífen) e grave numa pasta de
          projeto.
        </dd>
        <dt>Onde fica salva</dt>
        <dd>
          Na pasta que você escolheu: <code>vdesigner-cores.json</code>, para reabrir depois, e{" "}
          <code>cores.css</code>, com as variáveis prontas (<code>--nome-500</code>). Ficam no
          projeto para entrar no Git junto com o código.
        </dd>
        <dt>Conta-gotas</dt>
        <dd>
          <kbd>Ctrl+Alt+C</kbd> ou o botão Capturar cor congela a tela e abre uma lupa. Aponte e
          clique. Funciona sobre qualquer programa, em qualquer monitor.
        </dd>
        <dt>Para onde vai a cor capturada</dt>
        <dd>
          Vai na hora para a área de transferência e para a faixa Capturadas, que guarda as 12
          últimas. Elas somem ao fechar o app. Para guardar uma, clique em Usar, dê um nome e grave
          na paleta.
        </dd>
        <dt>Degradês</dt>
        <dd>
          Com duas cores na paleta, escolha as pontas e dê um nome. Copiar leva a regra CSS pronta.
          A mistura é feita em OKLCH, então o meio não fica acinzentado.
        </dd>
      </dl>

      <h3>Atalhos</h3>
      <dl>
        <dt>
          <kbd>Ctrl+Alt+C</kbd>
        </dt>
        <dd>Capturar cor, com o app aberto.</dd>
        <dt>Setas</dt>
        <dd>Na lupa, movem um pixel por vez.</dd>
        <dt>
          <kbd>Enter</kbd> ou <kbd>Espaço</kbd>
        </dt>
        <dd>Captura o pixel sob a mira.</dd>
        <dt>
          <kbd>Esc</kbd>
        </dt>
        <dd>Cancela sem mexer na área de transferência.</dd>
      </dl>

      <h3>O que ainda não existe</h3>
      <p>
        Processar várias imagens de uma vez, guardar as capturadas depois de fechar, trocar o atalho
        ou usá-lo com o app fechado, e versões para Linux e macOS.
      </p>
    </section>
  );
}
