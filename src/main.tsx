import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { I18nProvider } from "./lib/i18n";
import { UpdateProvider } from "./contexts/UpdateContext";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <I18nProvider><UpdateProvider><App /></UpdateProvider></I18nProvider>
  </React.StrictMode>,
);
