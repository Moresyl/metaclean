import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { I18nProvider } from "./lib/i18n";
import { UpdateProvider } from "./contexts/UpdateContext";
import { initializeTheme, ThemeProvider } from "./contexts/ThemeContext";

const initialTheme = initializeTheme();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider initialMode={initialTheme}><I18nProvider><UpdateProvider><App /></UpdateProvider></I18nProvider></ThemeProvider>
  </React.StrictMode>,
);
