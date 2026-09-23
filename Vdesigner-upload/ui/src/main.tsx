import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App } from "./App";
import { Overlay } from "./overlay/Overlay";
// Empacotadas em vez de vir da CDN: a CSP do app é `default-src 'self'`
// e ele precisa abrir sem rede.
import "@fontsource-variable/gabarito";
import "@fontsource-variable/spline-sans-mono";
import "./styles.css";

const ehSobreposicao = getCurrentWindow().label === "overlay";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {ehSobreposicao ? <Overlay /> : <App />}
  </React.StrictMode>,
);
