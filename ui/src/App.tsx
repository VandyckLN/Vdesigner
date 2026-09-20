import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Sidebar, panelId, tabId, type Screen } from "./components/Sidebar";
import { Colors } from "./screens/Colors";
import { Editor } from "./screens/Editor";

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg"];

export function App() {
  const [screen, setScreen] = useState<Screen>("imagem");

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
      <Sidebar current={screen} onChange={setScreen} />
      {/* Each screen is wrapped rather than given the role itself, so the
          panel's own top-level element (main.editor / main.colors) keeps its
          existing props untouched while still satisfying the tabs pattern:
          one tabpanel per tab, wired back to it with aria-labelledby. Only
          the active screen is mounted, same as before this change, so
          switching screens still resets the inactive one's local state. */}
      {screen === "imagem" ? (
        <div role="tabpanel" id={panelId("imagem")} aria-labelledby={tabId("imagem")}>
          <Editor onOpenFile={pickFile} onPickDirectory={pickDirectory} />
        </div>
      ) : (
        <div role="tabpanel" id={panelId("cores")} aria-labelledby={tabId("cores")}>
          <Colors onPickDirectory={pickDirectory} />
        </div>
      )}
    </div>
  );
}
