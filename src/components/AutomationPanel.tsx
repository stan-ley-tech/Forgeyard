import { useEffect, useState } from "react";

import {
  createAutomationRule,
  deleteAutomationRule,
  listAutomationRules,
  listAutomationRuns,
  listTasks,
  runAutomationRuleNow,
  toggleAutomationRule,
} from "../lib/ipc";
import type { AutomationRule, AutomationRunRecord, ProjectRecord, TaskGroup, TaskRef } from "../lib/types";

interface AutomationPanelProps {
  project: ProjectRecord;
}

export function AutomationPanel({ project }: AutomationPanelProps) {
  const [rules, setRules] = useState<AutomationRule[]>([]);
  const [taskGroups, setTaskGroups] = useState<TaskGroup[]>([]);
  const [runs, setRuns] = useState<Record<string, AutomationRunRecord[]>>({});
  const [name, setName] = useState("");
  const [globs, setGlobs] = useState("");
  const [selectedTasks, setSelectedTasks] = useState<TaskRef[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    listAutomationRules(project.id).then(setRules).catch((err) => setError(String(err)));
  };

  useEffect(refresh, [project.id]);
  useEffect(() => {
    listTasks(project.id).then(setTaskGroups).catch(() => setTaskGroups([]));
  }, [project.id]);

  const loadRuns = async (ruleId: string) => {
    const history = await listAutomationRuns(ruleId);
    setRuns((prev) => ({ ...prev, [ruleId]: history }));
  };

  const toggleTaskSelection = (ref: TaskRef) => {
    setSelectedTasks((prev) => {
      const exists = prev.some((t) => t.group === ref.group && t.name === ref.name);
      return exists
        ? prev.filter((t) => !(t.group === ref.group && t.name === ref.name))
        : [...prev, ref];
    });
  };

  const handleCreate = async () => {
    if (!name.trim() || selectedTasks.length === 0) return;
    const watchGlobs = globs
      .split(",")
      .map((g) => g.trim())
      .filter(Boolean);

    try {
      setError(null);
      await createAutomationRule(project.id, name.trim(), watchGlobs, selectedTasks);
      setName("");
      setGlobs("");
      setSelectedTasks([]);
      refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="automation-panel">
      {error && <p className="automation-panel__error">{error}</p>}

      <div className="automation-panel__new">
        <h3>New rule</h3>
        <input placeholder="Rule name" value={name} onChange={(e) => setName(e.target.value)} />
        <input
          placeholder="Watch globs (comma-separated, e.g. *.rs, src/*.ts)"
          value={globs}
          onChange={(e) => setGlobs(e.target.value)}
        />
        <div className="automation-panel__task-picker">
          {taskGroups.flatMap((group) =>
            group.tasks.map((task) => {
              const ref: TaskRef = { group: group.name, name: task.name };
              const checked = selectedTasks.some((t) => t.group === ref.group && t.name === ref.name);
              return (
                <label key={`${ref.group}:${ref.name}`}>
                  <input type="checkbox" checked={checked} onChange={() => toggleTaskSelection(ref)} />
                  {ref.group} / {ref.name}
                </label>
              );
            }),
          )}
        </div>
        <button onClick={handleCreate}>Create rule</button>
      </div>

      <div className="automation-panel__rules">
        {rules.map((rule) => (
          <div key={rule.id} className="automation-panel__rule">
            <div className="automation-panel__rule-header">
              <strong>{rule.name}</strong>
              <span>{rule.watch_globs.join(", ") || "any file change"}</span>
              <label>
                <input
                  type="checkbox"
                  checked={rule.enabled}
                  onChange={(e) =>
                    toggleAutomationRule(rule.id, e.target.checked).then(refresh)
                  }
                />
                Enabled
              </label>
              <button onClick={() => runAutomationRuleNow(rule.id).then(() => loadRuns(rule.id))}>
                Run now
              </button>
              <button onClick={() => loadRuns(rule.id)}>History</button>
              <button onClick={() => deleteAutomationRule(rule.id).then(refresh)}>Delete</button>
            </div>

            {runs[rule.id] && (
              <ul className="automation-panel__runs">
                {runs[rule.id].map((run) => (
                  <li key={run.id}>
                    <span className={`automation-panel__run-status--${run.status}`}>{run.status}</span>
                    {run.stages.map((stage) => (
                      <span
                        key={`${stage.group}:${stage.name}`}
                        className={`automation-panel__stage automation-panel__stage--${stage.status}`}
                      >
                        {stage.name}
                      </span>
                    ))}
                  </li>
                ))}
              </ul>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
