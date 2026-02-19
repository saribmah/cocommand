import React from "react";
import * as ReactJsxRuntime from "react/jsx-runtime";
import ReactDOM from "react-dom/client";
import { create, useStore } from "zustand";
import * as CocommandUI from "@cocommand/ui";
import * as CocommandSdk from "@cocommand/sdk";
import { BrowserRouter } from "react-router-dom";
import App from "./App";

// Expose host dependencies for dynamic extension views
(window as any).__ext_react = React;
(window as any).__ext_react_jsx = ReactJsxRuntime;
(window as any).__ext_zustand = { create, useStore };
(window as any).__ext_cocommand_ui = CocommandUI;
(window as any).__ext_cocommand_sdk = CocommandSdk;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>,
);
