import { useCallback, useEffect, useRef, useState } from "react";
import { api, type OutputFormat, type WheelProgress, type WheelState } from "../api";
import "./wheel.css";

/** Label on each slice. Short because it is rendered inside an arc. */
const ROTULOS: Record<OutputFormat, string> = {
  WebP: "WebP",
  Avif: "AVIF",
  Png: "PNG",
  Jpeg: "JPG",
  Tiff: "TIFF",
  Ico: "ICO",
  Svg: "SVG",
};

/** Geometry of the dial, in the SVG's own user units. */
const RAIO_EXTERNO = 190;
const RAIO_INTERNO = 74;
const CENTRO = 200;

/** Point on a circle, with 0 at twelve o'clock so the first slice sits at the
 *  top where the eye lands. */
function ponto(angulo: number, raio: number) {
  const radianos = ((angulo - 90) * Math.PI) / 180;
  return {
    x: CENTRO + raio * Math.cos(radianos),
    y: CENTRO + raio * Math.sin(radianos),
  };
}

/** Path for one slice of an annulus. Exported for the test, which checks the
 *  arcs close rather than eyeballing a screenshot. */
export function fatia(indice: number, total: number) {
  const passo = 360 / total;
  const inicio = indice * passo;
  const fim = inicio + passo;
  const externoInicio = ponto(inicio, RAIO_EXTERNO);
  const externoFim = ponto(fim, RAIO_EXTERNO);
  const internoFim = ponto(fim, RAIO_INTERNO);
  const internoInicio = ponto(inicio, RAIO_INTERNO);
  // A single slice can exceed a half turn when there are only two targets,
  // and an SVG arc needs to be told that.
  const arcoGrande = passo > 180 ? 1 : 0;
  return [
    `M ${externoInicio.x} ${externoInicio.y}`,
    `A ${RAIO_EXTERNO} ${RAIO_EXTERNO} 0 ${arcoGrande} 1 ${externoFim.x} ${externoFim.y}`,
    `L ${internoFim.x} ${internoFim.y}`,
    `A ${RAIO_INTERNO} ${RAIO_INTERNO} 0 ${arcoGrande} 0 ${internoInicio.x} ${internoInicio.y}`,
    "Z",
  ].join(" ");
}

/** Centre of a slice, where its label goes. */
export function centroDaFatia(indice: number, total: number) {
  const passo = 360 / total;
  return ponto(indice * passo + passo / 2, (RAIO_EXTERNO + RAIO_INTERNO) / 2);
}

export function Wheel() {
  const [estado, setEstado] = useState<WheelState | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [foco, setFoco] = useState(0);
  const [progresso, setProgresso] = useState<WheelProgress | null>(null);
  // Guards a second commit while the first is in flight: the window is
  // closing, and the second call would meet a disarmed wheel and raise an
  // error the person cannot act on. Same guard the eyedropper overlay makes.
  const convertendo = useRef(false);

  useEffect(() => {
    let vivo = true;
    api
      .startWheel()
      .then((s) => {
        if (vivo) setEstado(s);
      })
      .catch((e: unknown) => {
        if (vivo) setErro(String(e));
      });
    return () => {
      vivo = false;
    };
  }, []);

  useEffect(() => {
    let parar: (() => void) | undefined;
    api
      .onWheelProgress((p) => setProgresso(p))
      .then((un) => {
        parar = un;
      })
      .catch(() => {
        // Losing the progress stream costs the counter, not the conversion.
      });
    return () => parar?.();
  }, []);

  const cancelar = useCallback(() => {
    void api.cancelWheel().catch(() => {
      // The window is going away either way; a failure here has nobody left
      // to tell.
    });
  }, []);

  const confirmar = useCallback(
    (target: OutputFormat) => {
      if (convertendo.current) return;
      convertendo.current = true;
      void api.convertSelection(target).catch((e: unknown) => {
        convertendo.current = false;
        setErro(String(e));
      });
    },
    [],
  );

  const alvos = estado?.targets ?? [];

  useEffect(() => {
    function aoTeclar(evento: KeyboardEvent) {
      if (evento.key === "Escape") {
        evento.preventDefault();
        cancelar();
        return;
      }
      if (alvos.length === 0) return;
      if (evento.key === "ArrowRight" || evento.key === "ArrowDown") {
        evento.preventDefault();
        setFoco((f) => (f + 1) % alvos.length);
        return;
      }
      if (evento.key === "ArrowLeft" || evento.key === "ArrowUp") {
        evento.preventDefault();
        setFoco((f) => (f - 1 + alvos.length) % alvos.length);
        return;
      }
      if (evento.key === "Enter" || evento.key === " ") {
        evento.preventDefault();
        const alvo = alvos[foco];
        if (alvo) confirmar(alvo);
      }
    }
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, [alvos, foco, cancelar, confirmar]);

  if (erro) {
    return (
      <div className="roda-aviso" role="alert">
        <p>{erro}</p>
        <button type="button" onClick={cancelar}>
          Fechar
        </button>
      </div>
    );
  }

  if (!estado) {
    return (
      <div className="roda-aviso" aria-busy="true">
        <p>Lendo a seleção…</p>
      </div>
    );
  }

  if (progresso) {
    return (
      <div className="roda-aviso" role="status">
        <p>
          Convertendo {progresso.current} de {progresso.total}
        </p>
        {progresso.error ? <p className="roda-falha">{progresso.error}</p> : null}
      </div>
    );
  }

  const total = alvos.length;

  return (
    <div className="roda">
      <svg viewBox="0 0 400 400" role="menu" aria-label="Formato de saída">
        {alvos.map((alvo, indice) => {
          const rotulo = ROTULOS[alvo];
          const centro = centroDaFatia(indice, total);
          return (
            <g
              key={alvo}
              role="menuitem"
              aria-label={rotulo}
              tabIndex={-1}
              className={indice === foco ? "fatia fatia-ativa" : "fatia"}
              onMouseEnter={() => setFoco(indice)}
              onClick={() => confirmar(alvo)}
            >
              <path d={fatia(indice, total)} />
              <text x={centro.x} y={centro.y} textAnchor="middle" dominantBaseline="middle">
                {rotulo}
              </text>
            </g>
          );
        })}
        {/* Filled so the count stays legible over a light desktop. */}
        <circle className="roda-miolo" cx={CENTRO} cy={CENTRO} r={RAIO_INTERNO - 8} />
        <text className="roda-contagem" x={CENTRO} y={CENTRO - 10} textAnchor="middle">
          {estado.files.length}
        </text>
        <text className="roda-unidade" x={CENTRO} y={CENTRO + 10} textAnchor="middle">
          {estado.files.length === 1 ? "arquivo" : "arquivos"}
        </text>
        {alvos[foco] ? (
          <text className="roda-destino" x={CENTRO} y={CENTRO + 32} textAnchor="middle">
            para <tspan>{ROTULOS[alvos[foco]]}</tspan>
          </text>
        ) : null}
      </svg>

      <p className="roda-dica">Enter converte · Esc fecha</p>
      {estado.skipped.length > 0 ? (
        <p className="roda-nota">
          {estado.skipped.length === 1
            ? "1 arquivo fora: o Vdesigner não abre esse formato."
            : `${estado.skipped.length} arquivos fora: o Vdesigner não abre esses formatos.`}
        </p>
      ) : null}
      {estado.truncated ? (
        <p className="roda-nota">
          A seleção passou do limite; só os primeiros {estado.files.length} entraram.
        </p>
      ) : null}
    </div>
  );
}
