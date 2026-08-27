import { useEffect, useState } from "react";

import {
  gitBranches,
  gitCheckout,
  gitCommit,
  gitDiff,
  gitLog,
  gitStage,
  gitStash,
  gitStashPop,
  gitStatus,
  gitUnstage,
} from "../lib/ipc";
import type { BranchInfo, CommitInfo, FileChange, GitStatus, ProjectRecord } from "../lib/types";

interface GitPanelProps {
  project: ProjectRecord;
}

export function GitPanel({ project }: GitPanelProps) {
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [commits, setCommits] = useState<CommitInfo[]>([]);
  const [diff, setDiff] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    gitStatus(project.id).then(setStatus).catch((err) => setError(String(err)));
    gitBranches(project.id).then(setBranches).catch(() => setBranches([]));
    gitLog(project.id, 20).then(setCommits).catch(() => setCommits([]));
  };

  useEffect(refresh, [project.id]);

  const run = async (action: Promise<unknown>) => {
    try {
      setError(null);
      await action;
      refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  const showDiff = async (file: FileChange, staged: boolean) => {
    try {
      setDiff(await gitDiff(project.id, file.path, staged));
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="git-panel">
      {error && <p className="git-panel__error">{error}</p>}

      <div className="git-panel__columns">
        <div className="git-panel__column">
          <h3>Branch: {status?.branch ?? "detached"}</h3>
          {status && (status.ahead > 0 || status.behind > 0) && (
            <p className="git-panel__ahead-behind">
              ↑{status.ahead} ↓{status.behind}
            </p>
          )}

          <h4>Staged</h4>
          <ul className="git-panel__file-list">
            {status?.staged.map((f) => (
              <li key={f.path}>
                <button onClick={() => showDiff(f, true)}>
                  [{f.status}] {f.path}
                </button>
                <button onClick={() => run(gitUnstage(project.id, [f.path]))}>Unstage</button>
              </li>
            ))}
          </ul>

          <h4>Unstaged</h4>
          <ul className="git-panel__file-list">
            {status?.unstaged.map((f) => (
              <li key={f.path}>
                <button onClick={() => showDiff(f, false)}>
                  [{f.status}] {f.path}
                </button>
                <button onClick={() => run(gitStage(project.id, [f.path]))}>Stage</button>
              </li>
            ))}
            {status?.untracked.map((path) => (
              <li key={path}>
                <span>[U] {path}</span>
                <button onClick={() => run(gitStage(project.id, [path]))}>Stage</button>
              </li>
            ))}
          </ul>

          <div className="git-panel__commit">
            <input
              placeholder="Commit message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
            />
            <button
              onClick={() => {
                if (!message.trim()) return;
                run(gitCommit(project.id, message.trim())).then(() => setMessage(""));
              }}
            >
              Commit
            </button>
          </div>

          <div className="git-panel__stash">
            <button onClick={() => run(gitStash(project.id))}>Stash</button>
            <button onClick={() => run(gitStashPop(project.id))}>Pop stash</button>
          </div>
        </div>

        <div className="git-panel__column">
          <h4>Branches</h4>
          <ul className="git-panel__branch-list">
            {branches.map((b) => (
              <li key={b.name}>
                <span className={b.is_current ? "git-panel__branch--current" : ""}>{b.name}</span>
                {!b.is_current && !b.is_remote && (
                  <button onClick={() => run(gitCheckout(project.id, b.name, false))}>Checkout</button>
                )}
              </li>
            ))}
          </ul>

          <h4>Recent commits</h4>
          <ul className="git-panel__commit-list">
            {commits.map((c) => (
              <li key={c.hash}>
                <code>{c.short_hash}</code> {c.message}
                <span className="git-panel__commit-meta">
                  {c.author} · {new Date(c.date).toLocaleString()}
                </span>
              </li>
            ))}
          </ul>
        </div>
      </div>

      {diff !== null && (
        <div className="git-panel__diff">
          <div className="git-panel__diff-header">
            <h4>Diff</h4>
            <button onClick={() => setDiff(null)}>Close</button>
          </div>
          <pre>{diff || "No changes."}</pre>
        </div>
      )}
    </div>
  );
}
