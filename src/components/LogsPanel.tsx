import { useEffect, useMemo, useState } from "react";

import { getLogs, onProcessLog } from "../lib/ipc";
import type { LogEntry, LogLevel, ProjectRecord } from "../lib/types";

interface LogsPanelProps {
  project: ProjectRecord;
}

export function LogsPanel({ project }: LogsPanelProps) {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [service, setService] = useState("");
  const [level, setLevel] = useState<LogLevel | "">("");
  const [search, setSearch] = useState("");
  const [live, setLive] = useState(true);

  useEffect(() => {
    getLogs(project.id, {
      service: service || null,
      level: level || null,
      search: search || null,
      limit: 500,
    }).then(setEntries);
  }, [project.id, service, level, search]);

  useEffect(() => {
    if (!live) return undefined;

    const unlisten = onProcessLog((event) => {
      if (event.project_id !== project.id) return;
      setEntries((prev) => [
        ...prev.slice(-999),
        {
          id: -Date.now(),
          project_id: event.project_id,
          service: event.name,
          level: event.stream === "stderr" ? "warn" : "info",
          message: event.line,
          created_at: event.timestamp,
        },
      ]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [project.id, live]);

  const services = useMemo(() => Array.from(new Set(entries.map((e) => e.service))), [entries]);

  return (
    <div className="logs-panel">
      <div className="logs-panel__filters">
        <select value={service} onChange={(e) => setService(e.target.value)}>
          <option value="">All services</option>
          {services.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>

        <select value={level} onChange={(e) => setLevel(e.target.value as LogLevel | "")}>
          <option value="">All levels</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </select>

        <input placeholder="Search…" value={search} onChange={(e) => setSearch(e.target.value)} />

        <label className="logs-panel__live-toggle">
          <input type="checkbox" checked={live} onChange={(e) => setLive(e.target.checked)} />
          Live
        </label>
      </div>

      <div className="logs-panel__list">
        {entries.map((entry) => (
          <div key={entry.id} className={`logs-panel__row logs-panel__row--${entry.level}`}>
            <span className="logs-panel__time">
              {new Date(entry.created_at).toLocaleTimeString()}
            </span>
            <span className="logs-panel__service">{entry.service.toUpperCase()}</span>
            <span className="logs-panel__message">{entry.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
