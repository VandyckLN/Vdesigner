import { VdkMark } from "./VdkMark";

export type Screen = "imagem" | "cores";

const SCREENS: { id: Screen; label: string; glyph: string }[] = [
  { id: "imagem", label: "Imagem", glyph: "▣" },
  { id: "cores", label: "Cores", glyph: "◑" },
];

/** A coluna de ferramentas. Papel `tablist` porque é exatamente isso que ela
 *  é para quem navega por teclado ou leitor de tela, mesmo desenhada como
 *  coluna de ícones. */
export function Sidebar({
  current,
  onChange,
}: {
  current: Screen;
  onChange: (screen: Screen) => void;
}) {
  return (
    <nav className="sidebar" role="tablist" aria-orientation="vertical" aria-label="Ferramentas">
      <span className="sidebar-mark" aria-hidden="true">
        <VdkMark />
      </span>
      {SCREENS.map((screen) => (
        <button
          key={screen.id}
          type="button"
          role="tab"
          aria-selected={current === screen.id}
          className="sidebar-tab"
          // Re-selecting the open screen would reset its state for no reason.
          onClick={() => current !== screen.id && onChange(screen.id)}
        >
          <span className="sidebar-glyph" aria-hidden="true">
            {screen.glyph}
          </span>
          {screen.label}
        </button>
      ))}
    </nav>
  );
}
