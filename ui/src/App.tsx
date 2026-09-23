import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { Sidebar, panelId, tabId, type Screen } from "./components/Sidebar";
import { Colors } from "./screens/Colors";
import { Editor } from "./screens/Editor";
import { About } from "./screens/About";
import { UpdateBanner } from "./components/UpdateBanner";

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg"];

export function App() {
  const [screen, setScreen] = useState<Screen>("imagem");
  const [atalhoIndisponivel, setAtalhoIndisponivel] = useState<string | null>(null);

  useEffect(() => {
    let parar: (() => void) | undefined;
    void api.onShortcutUnavailable((atalho) => setAtalhoIndisponivel(atalho)).then((f) => {
      parar = f;
    });
    return () => parar?.();
  }, []);

  const pickFile = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Imagens", extensions: IMAGE_EXTENSIONS }],
    });
    return typeof selected === "string" ? selected : null;
  };

  const pickDirectory = async () => {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  };

  return (
    <div className="app-shell">
      {atalhoIndisponivel && (
        <p className="shortcut-warning" role="alert">
          O atalho {atalhoIndisponivel} já está em uso por outro programa. Use o botão
          “Capturar cor” na tela de Cores.
        </p>
      )}
      <UpdateBanner />
      <Sidebar current={screen} onChange={setScreen} />
      {/* All panels stay mounted and the inactive ones carry `hidden`,
          because a tab's aria-controls must point at an element that exists:
          with only the active screen rendered, an inactive tab referenced a
          missing id, which is invalid per WAI-ARIA. `hidden` keeps it out of
          the accessibility tree and off the screen just the same. */}
      <div role="tabpanel" id={panelId("imagem")} aria-labelledby={tabId("imagem")} hidden={screen !== "imagem"}>
        <Editor onOpenFile={pickFile} onPickDirectory={pickDirectory} />
      </div>
      <div role="tabpanel" id={panelId("cores")} aria-labelledby={tabId("cores")} hidden={screen !== "cores"}>
        <Colors onPickDirectory={pickDirectory} />
      </div>
      <div role="tabpanel" id={panelId("sobre")} aria-labelledby={tabId("sobre")} hidden={screen !== "sobre"}>
        <About />
      </div>
    </div>
  );
}
