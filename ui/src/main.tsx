import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
// Empacotadas em vez de vir da CDN: a CSP do app é `default-src 'self'`
// e ele precisa abrir sem rede.
import "@fontsource-variable/gabarito";
import "@fontsource-variable/spline-sans-mono";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
