import { useEffect, useState } from "react";

import { listTasks, runTask } from "../lib/ipc";
import type { ProjectRecord, TaskGroup } from "../lib/types";

interface TaskPanelProps {
  project: ProjectRecord;
}

export function TaskPanel({ project }: TaskPanelProps) {
  const [groups, setGroups] = useState<TaskGroup[]>([]);
  const [running, setRunning] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listTasks(project.id).then(setGroups).catch((err) => setError(String(err)));
  }, [project.id]);

  const handleRun = async (group: string, name: string) => {
    const key = `${group}:${name}`;
    setRunning(key);
    setError(null);
    try {
      await runTask(project.id, group, name);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(null);
    }
  };

  if (groups.length === 0) {
    return (
      <p className="task-panel__empty">
        No tasks found. Add a <code>forgeyard.toml</code> to this project, or make sure its{" "}
        <code>package.json</code> defines a <code>scripts</code> section.
      </p>
    );
  }

  return (
    <div className="task-panel">
      {error && <p className="task-panel__error">{error}</p>}

      {groups.map((group) => (
        <div key={group.name} className="task-panel__group">
          <h3>{group.name}</h3>
          <div className="task-panel__tasks">
            {group.tasks.map((task) => {
              const key = `${group.name}:${task.name}`;
              return (
                <button
                  key={key}
                  className="task-panel__task"
                  disabled={running === key}
                  onClick={() => handleRun(group.name, task.name)}
                  title={task.command}
                >
                  {running === key ? "Running…" : task.name}
                </button>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
