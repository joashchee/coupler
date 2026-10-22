import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { WEB } from "./lib/web";
import { WebWelcome } from "./web/WebWelcome";
import "./index.css";
import "./ansiapps-theme.css";
import "./phone.css";
import { applyFormFactor } from "./lib/formFactor";

applyFormFactor();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {WEB ? (
      <WebWelcome>
        <App />
      </WebWelcome>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
