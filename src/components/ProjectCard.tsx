import { useEffect, useState } from "react";

import { getProjectHealth } from "../lib/ipc";
import type { ProjectHealth, ProjectRecord } from "../lib/types";

interface ProjectCardProps {
  project: ProjectRecord;
  onOpen: () => void;
}

export function ProjectCard({ project, onOpen }: ProjectCardProps) {
  const [health, setHealth] = useState<ProjectHealth | null>(null);

  useEffect(() => {
    getProjectHealth(project.id)
      .then(setHealth)
      .catch(() => setHealth(null));
  }, [project.id]);

  return (
    <button className="project-card" onClick={onOpen}>
      <div className="project-card__name">{project.name}</div>
      <div className="project-card__path">{project.path}</div>

      {health && (
        <div className="project-card__badges">
          {health.languages.map((lang) => (
            <span key={lang} className="badge badge--language">
              {lang}
            </span>
          ))}
          {health.docker.has_dockerfile || health.docker.has_compose ? (
            <span className="badge badge--docker">docker</span>
          ) : null}
          {health.has_git && <span className="badge badge--git">git</span>}
          {health.env_files.length > 0 && (
            <span className="badge badge--env">{health.env_files.length} env file(s)</span>
          )}
        </div>
      )}
    </button>
  );
}
