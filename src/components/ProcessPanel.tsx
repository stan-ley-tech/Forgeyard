import { useEffect, useRef, useState } from "react";

import {
  killProcess,
  listProcesses,
  onProcessLog,
  onProcessStatus,
  restartProcess,
  startProcess,
  stopProcess,
} from "../lib/ipc";
import type { ProcessInfo, ProjectRecord } from "../lib/types";
import { TerminalView, type TerminalLine } from "./TerminalView";

interface ProcessPanelProps {
  project: ProjectRecord;
}

export function ProcessPanel({ project }: ProcessPanelProps) {
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [logs, setLogs] = useState<Record<string, TerminalLine[]>>({});
  const [newName, setNewName] = useState("");
  const [newCommand, setNewCommand] = useState("");
  const [error, setError] = useState<string | null>(null);
  const nextLogId = useRef(0);

  useEffect(() => {
    listProcesses(project.id).then(setProcesses);

    const unlistenLog = onProcessLog((event) => {
      if (event.project_id !== project.id) return;
      setLogs((prev) => {
        const existing = prev[event.name] ?? [];
        const line: TerminalLine = {
          id: `${event.name}-${nextLogId.current++}`,
          text: event.line,
          variant: event.stream,
        };
        return { ...prev, [event.name]: [...existing.slice(-499), line] };
      });
    });

    const unlistenStatus = onProcessStatus((info) => {
      if (info.project_id !== project.id) return;
      setProcesses((prev) => {
        const others = prev.filter((p) => p.name !== info.name);
        return [...others, info];
      });
    });

    return () => {
      unlistenLog.then((fn) => fn());
      unlistenStatus.then((fn) => fn());
    };
  }, [project.id]);

  const handleStart = async () => {
    if (!newName.trim() || !newCommand.trim()) return;
    try {
      setError(null);
      await startProcess(project.id, newName.trim(), newCommand.trim());
      setNewName("");
      setNewCommand("");
    } catch (err) {
      setError(String(err));
    }
  };

  const runAction = async (action: Promise<unknown>) => {
    try {
      setError(null);
      await action;
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="process-panel">
      <div className="process-panel__new">
        <input
          placeholder="Name (e.g. API)"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <input
          placeholder="Command (e.g. npm run dev)"
          value={newCommand}
          onChange={(e) => setNewCommand(e.target.value)}
        />
        <button onClick={handleStart}>Start</button>
      </div>

      {error && <p className="process-panel__error">{error}</p>}

      <table className="process-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Status</th>
            <th>PID</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {processes.map((proc) => (
            <tr
              key={proc.name}
              className={proc.name === selected ? "process-table__row--selected" : ""}
              onClick={() => setSelected(proc.name)}
            >
              <td>{proc.name}</td>
              <td>
                <span className={`status-dot status-dot--${proc.status}`} />
                {proc.status}
                {proc.exit_code !== null && ` (exit ${proc.exit_code})`}
              </td>
              <td>{proc.pid ?? "—"}</td>
              <td className="process-table__actions">
                <button onClick={() => runAction(stopProcess(project.id, proc.name))}>Stop</button>
                <button onClick={() => runAction(restartProcess(project.id, proc.name, proc.command))}>
                  Restart
                </button>
                <button onClick={() => runAction(killProcess(project.id, proc.name))}>Kill</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {selected && (
        <div className="process-panel__logs">
          <h3>{selected} — output</h3>
          <TerminalView lines={logs[selected] ?? []} />
        </div>
      )}
    </div>
  );
}
