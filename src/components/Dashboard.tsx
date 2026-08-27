import { useEffect, useState } from "react";

import { addProject, listProjects } from "../lib/ipc";
import type { ProjectRecord } from "../lib/types";
import { ProjectCard } from "./ProjectCard";

interface DashboardProps {
  onOpenProject: (project: ProjectRecord) => void;
}

export function Dashboard({ onOpenProject }: DashboardProps) {
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    listProjects()
      .then(setProjects)
      .catch((err) => setError(String(err)))
      .finally(() => setLoading(false));
  };

  useEffect(refresh, []);

  const handleAdd = async () => {
    const path = window.prompt("Absolute path to a local Git repository:");
    if (!path) return;

    try {
      setError(null);
      await addProject(path);
      refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  if (loading) {
    return <p className="dashboard__status">Loading projects…</p>;
  }

  return (
    <div className="dashboard">
      <div className="dashboard__header">
        <h1>Projects</h1>
        <button onClick={handleAdd}>Add project</button>
      </div>

      {error && <p className="dashboard__error">{error}</p>}

      {projects.length === 0 ? (
        <p className="dashboard__status">
          No projects registered yet. Add a local Git repository to get started.
        </p>
      ) : (
        <div className="dashboard__grid">
          {projects.map((project) => (
            <ProjectCard key={project.id} project={project} onOpen={() => onOpenProject(project)} />
          ))}
        </div>
      )}
    </div>
  );
}
