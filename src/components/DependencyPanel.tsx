import { useEffect, useState } from "react";

import { listDependencies } from "../lib/ipc";
import type { DependencyReport, ProjectRecord } from "../lib/types";

interface DependencyPanelProps {
  project: ProjectRecord;
}

export function DependencyPanel({ project }: DependencyPanelProps) {
  const [report, setReport] = useState<DependencyReport | null>(null);
  const [filter, setFilter] = useState("");

  useEffect(() => {
    listDependencies(project.id).then(setReport);
  }, [project.id]);

  if (!report) {
    return <p>Loading dependencies…</p>;
  }

  const filtered = report.dependencies.filter((dep) =>
    dep.name.toLowerCase().includes(filter.toLowerCase()),
  );

  return (
    <div className="dependency-panel">
      <div className="dependency-panel__header">
        <h3>Dependencies ({report.dependencies.length})</h3>
        <input
          placeholder="Filter by name…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>

      {report.conflicts.length > 0 && (
        <div className="dependency-panel__conflicts">
          <h4>Potential conflicts ({report.conflicts.length})</h4>
          <ul>
            {report.conflicts.map((c) => (
              <li key={c.name}>
                <strong>{c.name}</strong>:{" "}
                {c.sources.map(([source, version]) => `${source} → ${version}`).join(", ")}
              </li>
            ))}
          </ul>
        </div>
      )}

      <table className="dependency-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Version</th>
            <th>Kind</th>
            <th>Ecosystem</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((dep) => (
            <tr key={`${dep.ecosystem}-${dep.name}-${dep.kind}`}>
              <td>{dep.name}</td>
              <td>{dep.version_spec}</td>
              <td>{dep.kind}</td>
              <td>{dep.ecosystem}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
