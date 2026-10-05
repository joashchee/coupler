import { useState } from "react";
import { Dialog } from "../components/Dialog";

/** The browser keeps the answer for this site, as the desktop keeps its marker file. */
const ACCEPTED = "coupler.web.accepted";

function accepted(): boolean {
  try {
    return localStorage.getItem(ACCEPTED) === "1";
  } catch {
    return false;
  }
}

/**
 * The first-run warning every ansiapps app shows (src-tauri/src/first_run.rs
 * on the desktop), in the page: a browser has no native dialog with these
 * buttons. Nothing of Coupler starts until OK, which is remembered. A page
 * can't close its tab, so "I'll Be Back." goes to ansiapps.com.
 */
export function WebWelcome({ children }: { children: React.ReactNode }) {
  const [ok, setOk] = useState(accepted);
  if (ok) return <>{children}</>;
  const accept = () => {
    try {
      localStorage.setItem(ACCEPTED, "1");
    } catch {
      // Not kept: it asks again next visit.
    }
    setOk(true);
  };
  return (
    <Dialog
      open
      onClose={() => window.location.assign("https://ansiapps.com/")}
      title="Welcome to Coupler"
      actions={
        <>
          <button type="button" onClick={() => window.location.assign("https://ansiapps.com/")}>
            I'll Be Back.
          </button>
          <button type="button" className="primary" onClick={accept}>
            OK
          </button>
        </>
      }
    >
      <p>Coupler is still in development. Some features may be missing or not work as expected. Use it at your own risk.</p>
    </Dialog>
  );
}
