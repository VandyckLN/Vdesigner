import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import {
  procurarAtualizacaoManual,
  instalar,
  checagemAutomaticaLigada,
  definirChecagemAutomatica,
  type Atualizacao,
} from "../update";

export function About() {
  const [versao, setVersao] = useState("");
  const [automatica, setAutomatica] = useState(checagemAutomaticaLigada());
  const [resultado, setResultado] = useState<string | null>(null);
  const [encontrada, setEncontrada] = useState<Atualizacao | null>(null);
  const [procurando, setProcurando] = useState(false);
  const [progresso, setProgresso] = useState<number | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    void getVersion().then(setVersao);
  }, []);

  const procurar = async () => {
    setProcurando(true);
    setErro(null);
    setEncontrada(null);
    const resposta = await procurarAtualizacaoManual();
    if (resposta.estado === "encontrada") {
      setEncontrada(resposta.atualizacao);
      setResultado(`Versão ${resposta.atualizacao.versao} disponível.`);
    } else if (resposta.estado === "atualizado") {
      setResultado("Você já está na versão mais recente.");
    } else {
      // Never "você já está na versão mais recente" here: with the network
      // down that would be an affirmative lie about something the person
      // just asked to have checked.
      setResultado("Não foi possível verificar se há atualização. Tente de novo mais tarde.");
    }
    setProcurando(false);
  };

  // The banner is the only other thing that can install, and it returns early
  // when automatic checking is off — so without this button a person who
  // unticked the option and then searched manually would be told to reopen
  // the app and, on reopening, see nothing at all. Same `instalar` the banner
  // uses: one download/verify/relaunch path, not two.
  const atualizar = async () => {
    if (!encontrada) return;
    setErro(null);
    setProgresso(0);
    try {
      await instalar(encontrada, setProgresso);
    } catch (falha) {
      setProgresso(null);
      setErro(falha instanceof Error ? falha.message : String(falha));
    }
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
      {encontrada && (
        <>
          {erro && <span className="about-error">Falhou: {erro}</span>}
          {progresso !== null && <span className="about-progress">{progresso}%</span>}
          <button type="button" onClick={() => void atualizar()} disabled={progresso !== null}>
            Atualizar agora
          </button>
        </>
      )}
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
