import { useEffect, useState } from "react";
import { procurarAtualizacao, instalar, checagemAutomaticaLigada, type Atualizacao } from "../update";

/** `role="status"` and not `alert`: a new version is information, not an
 *  emergency, so a screen reader announces it without interrupting. */
export function UpdateBanner() {
  const [atualizacao, setAtualizacao] = useState<Atualizacao | null>(null);
  const [dispensada, setDispensada] = useState(false);
  const [progresso, setProgresso] = useState<number | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    if (!checagemAutomaticaLigada()) return;
    let vivo = true;
    void procurarAtualizacao().then((encontrada) => {
      if (vivo) setAtualizacao(encontrada);
    });
    return () => {
      vivo = false;
    };
  }, []);

  if (!atualizacao || dispensada) return null;

  const atualizar = async () => {
    setErro(null);
    setProgresso(0);
    try {
      await instalar(atualizacao, setProgresso);
    } catch (falha) {
      setProgresso(null);
      setErro(falha instanceof Error ? falha.message : String(falha));
    }
  };

  return (
    <div className="update-banner" role="status">
      <span className="update-banner-text">Versão {atualizacao.versao} disponível.</span>
      {erro && <span className="update-banner-error">Falhou: {erro}</span>}
      {progresso !== null && <span className="update-banner-progress">{progresso}%</span>}
      <button type="button" onClick={() => void atualizar()} disabled={progresso !== null}>
        Atualizar
      </button>
      <button type="button" onClick={() => setDispensada(true)}>
        Agora não
      </button>
    </div>
  );
}
