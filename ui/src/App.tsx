import { open } from "@tauri-apps/plugin-dialog";
import { Editor } from "./screens/Editor";

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg"];

export function App() {
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

  return <Editor onOpenFile={pickFile} onPickDirectory={pickDirectory} />;
}
