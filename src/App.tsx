import { useState } from "react";
import "./App.css";

/**
 * Root shell. Real panels (dashboard, process manager, git, etc.) are
 * wired in as their backend commands land, one module at a time.
 */
export default function App() {
  const [ready] = useState(true);

  return (
    <div className="app-shell">
      <header className="app-shell__header">
        <span className="app-shell__brand">Forgeyard</span>
        <span className="app-shell__status">{ready ? "ready" : "starting…"}</span>
      </header>
      <main className="app-shell__main">
        <p>Project workspace loads here once a repository is registered.</p>
      </main>
    </div>
  );
}
