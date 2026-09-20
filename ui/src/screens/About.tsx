import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { procurarAtualizacao, checagemAutomaticaLigada, definirChecagemAutomatica } from "../update";

export function About() {
  const [versao, setVersao] = useState("");
  const [automatica, setAutomatica] = useState(checagemAutomaticaLigada());
  const [resultado, setResultado] = useState<string | null>(null);
  const [procurando, setProcurando] = useState(false);

  useEffect(() => {
    void getVersion().then(setVersao);
  }, []);

  const procurar = async () => {
    setProcurando(true);
    const encontrada = await procurarAtualizacao();
    setResultado(
      encontrada
        ? `Versão ${encontrada.versao} disponível. Reabra o aplicativo para atualizar.`
        : "Você já está na versão mais recente.",
    );
    setProcurando(false);
  };

  const alternar = (ligada: boolean) => {
    setAutomatica(ligada);
    definirChecagemAutomatica(ligada);
  };

  return (
    <section className="about">
      <h2>Sobre</h2>
      <p className="about-version">Vdesigner {versao}</p>
      <button type="button" onClick={() => void procurar()} disabled={procurando}>
        Procurar atualizações
      </button>
      {resultado && <p className="about-result">{resultado}</p>}
      <label className="about-toggle">
        <input type="checkbox" checked={automatica} onChange={(e) => alternar(e.target.checked)} />
        Procurar atualizações ao abrir
      </label>
      <p className="about-note">
        Com essa opção ligada, o aplicativo consulta um arquivo de versão no GitHub ao abrir.
        Nenhuma imagem sua sai da máquina.
      </p>
    </section>
  );
}
