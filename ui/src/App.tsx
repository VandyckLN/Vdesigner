import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Sidebar, type Screen } from "./components/Sidebar";
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
      {screen === "imagem" ? (
        <Editor onOpenFile={pickFile} onPickDirectory={pickDirectory} />
      ) : (
        <Colors onPickDirectory={pickDirectory} />
      )}
    </div>
  );
}
