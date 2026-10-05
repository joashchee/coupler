import { useEffect, useState } from "react";
import { Dialog } from "./Dialog";

interface LicensesDialogProps {
  open: boolean;
  onClose: () => void;
}

/**
 * Every third-party license Coupler ships under, as the licenses ask
 * (written by scripts/third-party-licenses.py into public/, so it's in
 * the app). Opened from About, over it. Read on first open: it's large.
 */
export function LicensesDialog({ open, onClose }: LicensesDialogProps) {
  const [text, setText] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!open || text !== null) return;
    fetch("/third-party-licenses.txt")
      .then((r) => (r.ok ? r.text() : Promise.reject(new Error(String(r.status)))))
      .then(setText)
      .catch(() => setFailed(true));
  }, [open, text]);

  return (
    <Dialog open={open} onClose={onClose} title="Open-Source Licenses" className="dialog-wide">
      <p>Coupler is built with the open-source software below, each used under the license that follows its name.</p>
      {failed ? (
        <p>The licenses couldn't be read. Reinstalling Coupler should bring them back.</p>
      ) : (
        <pre className="licenses-text" data-testid="licenses-text" tabIndex={0} aria-label="Licenses: each package's name, then its license">
          {text ?? "Reading the licenses…"}
        </pre>
      )}
    </Dialog>
  );
}
