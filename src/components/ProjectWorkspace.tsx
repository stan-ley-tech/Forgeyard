import { useState } from "react";

import type { ProjectRecord } from "../lib/types";
import { AutomationPanel } from "./AutomationPanel";
import { DependencyPanel } from "./DependencyPanel";
import { GitPanel } from "./GitPanel";
import { LogsPanel } from "./LogsPanel";
import { ProcessPanel } from "./ProcessPanel";
import { TaskPanel } from "./TaskPanel";

const TABS = ["Processes", "Git", "Dependencies", "Logs", "Tasks", "Automation"] as const;
type Tab = (typeof TABS)[number];

interface ProjectWorkspaceProps {
  project: ProjectRecord;
  onBack: () => void;
}

export function ProjectWorkspace({ project, onBack }: ProjectWorkspaceProps) {
  const [tab, setTab] = useState<Tab>("Processes");

  return (
    <div className="workspace">
      <div className="workspace__header">
        <button className="workspace__back" onClick={onBack}>
          ← Projects
        </button>
        <h1>{project.name}</h1>
      </div>

      <nav className="workspace__tabs">
        {TABS.map((t) => (
          <button
            key={t}
            className={t === tab ? "workspace__tab workspace__tab--active" : "workspace__tab"}
            onClick={() => setTab(t)}
          >
            {t}
          </button>
        ))}
      </nav>

      <div className="workspace__content">
        {tab === "Processes" && <ProcessPanel project={project} />}
        {tab === "Git" && <GitPanel project={project} />}
        {tab === "Dependencies" && <DependencyPanel project={project} />}
        {tab === "Logs" && <LogsPanel project={project} />}
        {tab === "Tasks" && <TaskPanel project={project} />}
        {tab === "Automation" && <AutomationPanel project={project} />}
      </div>
    </div>
  );
}
