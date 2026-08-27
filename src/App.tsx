import { useState } from "react";

import "./App.css";
import { Dashboard } from "./components/Dashboard";
import { ProjectWorkspace } from "./components/ProjectWorkspace";
import type { ProjectRecord } from "./lib/types";

export default function App() {
  const [activeProject, setActiveProject] = useState<ProjectRecord | null>(null);

  return (
    <div className="app-shell">
      {activeProject ? (
        <ProjectWorkspace project={activeProject} onBack={() => setActiveProject(null)} />
      ) : (
        <Dashboard onOpenProject={setActiveProject} />
      )}
    </div>
  );
}
