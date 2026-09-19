/** O disco lunar do VDK, construído sobre o terminador (DESIGN.md, direção
 *  03 "A Fase"). Corte compacto — traço de 7u — porque é o desenhado para
 *  32px e abaixo; o corte fino não deve ser reduzido. */
export function VdkMark() {
  return (
    <svg viewBox="0 0 120 120" aria-hidden="true" focusable="false">
      <circle cx="60" cy="60" r="44.5" fill="none" stroke="currentColor" strokeWidth="7" />
      <path d="M60 15.5 A44.5 44.5 0 0 1 60 104.5 A23.9 44.5 0 0 0 60 15.5 Z" fill="currentColor" />
    </svg>
  );
}
