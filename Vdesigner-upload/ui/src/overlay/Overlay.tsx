import { useCallback, useEffect, useRef, useState } from "react";
import { api, type OverlayGeometry } from "../api";
import "./overlay.css";

/** Side of the magnifier in source pixels: an odd number so there is a single
 *  centre pixel to aim at. */
const LUPA_PIXELS = 15;
/** How much each source pixel is blown up in the magnifier. */
const LUPA_ZOOM = 12;

export function Overlay() {
  const [geometria, setGeometria] = useState<OverlayGeometry | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [imagemCarregada, setImagemCarregada] = useState(false);
  // Position in CSS pixels.
  const [posicao, setPosicao] = useState({ x: 0, y: 0 });
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const imagemRef = useRef<HTMLImageElement | null>(null);
  // Guards against a second pick while the first is still in flight: the
  // window is closing, and a second call would resolve against a disarmed
  // picker and surface an error the person cannot act on.
  const escolhendo = useRef(false);

  useEffect(() => {
    let vivo = true;
    api
      .startPick()
      .then((g) => {
        if (vivo) setGeometria(g);
      })
      .catch((e) => {
        if (vivo) setErro(String(e));
      });
    return () => {
      vivo = false;
    };
  }, []);

  const cancelar = useCallback(() => {
    if (escolhendo.current) return;
    escolhendo.current = true;
    void api.cancelPick();
  }, []);

  const escolher = useCallback(
    (x: number, y: number) => {
      if (escolhendo.current || !geometria) return;
      escolhendo.current = true;
      void api.pickAt(x, y).catch(() => {
        escolhendo.current = false;
      });
    },
    [geometria],
  );

  useEffect(() => {
    if (!geometria) return;

    const escala = window.devicePixelRatio || 1;

    const aoTeclar = (evento: KeyboardEvent) => {
      if (evento.key === "Escape") {
        evento.preventDefault();
        cancelar();
        return;
      }
      if (evento.key === "Enter" || evento.key === " ") {
        evento.preventDefault();
        escolher(posicao.x, posicao.y);
        return;
      }
      // Move by 1 physical pixel per press. Using integer pixel coordinates
      // avoids binary floating-point roundoff accumulation on fractional scales
      // (e.g. 150% or 175%), ensuring every keystroke advances exactly one physical pixel.
      const deltas: Record<string, [number, number]> = {
        ArrowLeft: [-1, 0],
        ArrowRight: [1, 0],
        ArrowUp: [0, -1],
        ArrowDown: [0, 1],
      };
      const delta = deltas[evento.key];
      if (!delta) return;
      evento.preventDefault();
      setPosicao((atual) => {
        const curPx = Math.round(atual.x * escala);
        const curPy = Math.round(atual.y * escala);
        const nextPx = Math.min(Math.max(curPx + delta[0], 0), geometria.width - 1);
        const nextPy = Math.min(Math.max(curPy + delta[1], 0), geometria.height - 1);
        return {
          x: nextPx / escala,
          y: nextPy / escala,
        };
      });
    };

    window.addEventListener("keydown", aoTeclar);
    window.addEventListener("blur", cancelar);
    return () => {
      window.removeEventListener("keydown", aoTeclar);
      window.removeEventListener("blur", cancelar);
    };
  }, [geometria, posicao, cancelar, escolher]);

  // Redraws the magnifier from the frozen picture. The canvas is preview
  // only — the committed colour comes from Rust — but both read the same
  // bitmap in the same coordinate space, so they agree by construction.
  useEffect(() => {
    const canvas = canvasRef.current;
    const imagem = imagemRef.current;
    if (!canvas || !imagem || !imagem.complete) return;
    const contexto = canvas.getContext("2d");
    if (!contexto) return;
    const meio = Math.floor(LUPA_PIXELS / 2);
    contexto.imageSmoothingEnabled = false;
    contexto.clearRect(0, 0, canvas.width, canvas.height);
    const escala = window.devicePixelRatio || 1;
    // Floor matches screen.rs (css * scale + 1e-6).floor() so the reticle never drifts by a pixel.
    const sx = Math.floor(posicao.x * escala + 1e-6);
    const sy = Math.floor(posicao.y * escala + 1e-6);
    contexto.drawImage(
      imagem,
      sx - meio,
      sy - meio,
      LUPA_PIXELS,
      LUPA_PIXELS,
      0,
      0,
      canvas.width,
      canvas.height,
    );
  }, [posicao, geometria, imagemCarregada]);

  if (erro) {
    return (
      <div className="overlay-erro" role="alert">
        Não foi possível capturar a tela: {erro}
      </div>
    );
  }

  if (!geometria) return <div className="overlay-vazio" />;

  const aoMover = (evento: React.MouseEvent<HTMLImageElement>) => {
    const escala = window.devicePixelRatio || 1;
    const maxCssX = Math.max(0, (geometria.width - 1) / escala);
    const maxCssY = Math.max(0, (geometria.height - 1) / escala);
    setPosicao({
      x: Math.min(Math.max(evento.clientX, 0), maxCssX),
      y: Math.min(Math.max(evento.clientY, 0), maxCssY),
    });
  };

  return (
    <div className="overlay">
      <img
        ref={imagemRef}
        className="overlay-retrato"
        src={`data:image/png;base64,${geometria.png_base64}`}
        alt="Retrato da tela"
        onLoad={() => setImagemCarregada(true)}
        onMouseMove={aoMover}
        onClick={(evento) => {
          const escala = window.devicePixelRatio || 1;
          const maxCssX = Math.max(0, (geometria.width - 1) / escala);
          const maxCssY = Math.max(0, (geometria.height - 1) / escala);
          const cx = Math.min(Math.max(evento.clientX, 0), maxCssX);
          const cy = Math.min(Math.max(evento.clientY, 0), maxCssY);
          setPosicao({ x: cx, y: cy });
          escolher(cx, cy);
        }}
        draggable={false}
      />
      <div
        className="overlay-lupa"
        style={{ left: posicao.x, top: posicao.y }}
      >
        <canvas ref={canvasRef} width={LUPA_PIXELS * LUPA_ZOOM} height={LUPA_PIXELS * LUPA_ZOOM} />
        <span className="overlay-mira" aria-hidden="true" />
      </div>
      <p className="overlay-ajuda">
        Clique para capturar. Setas movem um pixel. Esc cancela.
      </p>
    </div>
  );
}
