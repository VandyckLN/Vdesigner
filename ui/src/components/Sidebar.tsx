import { useRef } from "react";
import { VdkMark } from "./VdkMark";

export type Screen = "imagem" | "cores" | "sobre";

const SCREENS: { id: Screen; label: string; glyph: string }[] = [
  { id: "imagem", label: "Imagem", glyph: "▣" },
  { id: "cores", label: "Cores", glyph: "◑" },
  { id: "sobre", label: "Sobre", glyph: "◇" },
];

/** id of the tab button for a given screen — shared with App.tsx so each
 *  panel's aria-labelledby points at the exact tab that owns it. */
export const tabId = (screen: Screen) => `tab-${screen}`;
/** id of the tabpanel for a given screen — shared with App.tsx so each tab's
 *  aria-controls points at the exact panel it owns. */
export const panelId = (screen: Screen) => `panel-${screen}`;

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
  // Refs keyed by screen id so arrow-key navigation can move DOM focus to the
  // newly selected tab, as the WAI-ARIA tabs pattern requires (focus follows
  // selection, it does not merely follow a later re-render).
  const tabRefs = useRef<Partial<Record<Screen, HTMLButtonElement>>>({});

  const moveTo = (index: number) => {
    const target = SCREENS[(index + SCREENS.length) % SCREENS.length];
    onChange(target.id);
    tabRefs.current[target.id]?.focus();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    switch (event.key) {
      case "ArrowLeft":
      case "ArrowUp":
        event.preventDefault();
        moveTo(index - 1);
        break;
      case "ArrowRight":
      case "ArrowDown":
        event.preventDefault();
        moveTo(index + 1);
        break;
      case "Home":
        event.preventDefault();
        moveTo(0);
        break;
      case "End":
        event.preventDefault();
        moveTo(SCREENS.length - 1);
        break;
      default:
        break;
    }
  };

  return (
    <nav className="sidebar" role="tablist" aria-orientation="vertical" aria-label="Ferramentas">
      <span className="sidebar-mark" aria-hidden="true">
        <VdkMark />
      </span>
      {SCREENS.map((screen, index) => (
        <button
          key={screen.id}
          ref={(el) => {
            if (el) tabRefs.current[screen.id] = el;
          }}
          id={tabId(screen.id)}
          type="button"
          role="tab"
          aria-selected={current === screen.id}
          aria-controls={panelId(screen.id)}
          // Roving tabindex: only the selected tab is in the Tab order, the
          // rest are reached with the arrow keys per the WAI-ARIA pattern.
          tabIndex={current === screen.id ? 0 : -1}
          className="sidebar-tab"
          // Re-selecting the open screen would reset its state for no reason.
          onClick={() => current !== screen.id && onChange(screen.id)}
          onKeyDown={(event) => onKeyDown(event, index)}
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
