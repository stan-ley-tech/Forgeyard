import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AutomationRule,
  AutomationRunRecord,
  BranchInfo,
  CommitInfo,
  DependencyReport,
  GitStatus,
  LogEntry,
  LogFilter,
  ProcessInfo,
  ProcessLogEvent,
  ProjectHealth,
  ProjectRecord,
  StageResult,
  TaskGroup,
  TaskRef,
} from "./types";

// Projects
export const listProjects = () => invoke<ProjectRecord[]>("list_projects");
export const addProject = (path: string, name?: string) =>
  invoke<ProjectRecord>("add_project", { path, name });
export const removeProject = (id: string) => invoke<void>("remove_project", { id });
export const getProjectHealth = (id: string) => invoke<ProjectHealth>("get_project_health", { id });

// Dependencies
export const listDependencies = (id: string) => invoke<DependencyReport>("list_dependencies", { id });

// Processes
export const startProcess = (projectId: string, name: string, command: string) =>
  invoke<ProcessInfo>("start_process", { projectId, name, command });
export const stopProcess = (projectId: string, name: string) =>
  invoke<void>("stop_process", { projectId, name });
export const killProcess = (projectId: string, name: string) =>
  invoke<void>("kill_process", { projectId, name });
export const restartProcess = (projectId: string, name: string, command: string) =>
  invoke<ProcessInfo>("restart_process", { projectId, name, command });
export const listProcesses = (projectId: string) => invoke<ProcessInfo[]>("list_processes", { projectId });

export const onProcessLog = (handler: (event: ProcessLogEvent) => void): Promise<UnlistenFn> =>
  listen<ProcessLogEvent>("process:log", (event) => handler(event.payload));
export const onProcessStatus = (handler: (event: ProcessInfo) => void): Promise<UnlistenFn> =>
  listen<ProcessInfo>("process:status", (event) => handler(event.payload));

// Git
export const gitStatus = (projectId: string) => invoke<GitStatus>("git_status", { projectId });
export const gitBranches = (projectId: string) => invoke<BranchInfo[]>("git_branches", { projectId });
export const gitCheckout = (projectId: string, branch: string, create: boolean) =>
  invoke<void>("git_checkout", { projectId, branch, create });
export const gitStage = (projectId: string, files: string[]) =>
  invoke<void>("git_stage", { projectId, files });
export const gitUnstage = (projectId: string, files: string[]) =>
  invoke<void>("git_unstage", { projectId, files });
export const gitCommit = (projectId: string, message: string) =>
  invoke<void>("git_commit", { projectId, message });
export const gitDiff = (projectId: string, file: string | null, staged: boolean) =>
  invoke<string>("git_diff", { projectId, file, staged });
export const gitLog = (projectId: string, limit: number) =>
  invoke<CommitInfo[]>("git_log", { projectId, limit });
export const gitStash = (projectId: string, message?: string) =>
  invoke<void>("git_stash", { projectId, message });
export const gitStashPop = (projectId: string) => invoke<void>("git_stash_pop", { projectId });
export const gitMerge = (projectId: string, branch: string) =>
  invoke<void>("git_merge", { projectId, branch });

// Tasks
export const listTasks = (projectId: string) => invoke<TaskGroup[]>("list_tasks", { projectId });
export const runTask = (projectId: string, group: string, name: string) =>
  invoke<ProcessInfo>("run_task", { projectId, group, name });

// Logs
export const getLogs = (projectId: string, filter?: LogFilter) =>
  invoke<LogEntry[]>("get_logs", { projectId, filter });

// Automation
export const listAutomationRules = (projectId: string) =>
  invoke<AutomationRule[]>("list_automation_rules", { projectId });
export const createAutomationRule = (
  projectId: string,
  name: string,
  watchGlobs: string[],
  tasks: TaskRef[],
) => invoke<AutomationRule>("create_automation_rule", { projectId, name, watchGlobs, tasks });
export const deleteAutomationRule = (id: string) => invoke<void>("delete_automation_rule", { id });
export const toggleAutomationRule = (id: string, enabled: boolean) =>
  invoke<void>("toggle_automation_rule", { id, enabled });
export const runAutomationRuleNow = (id: string) =>
  invoke<StageResult[]>("run_automation_rule_now", { id });
export const listAutomationRuns = (ruleId: string) =>
  invoke<AutomationRunRecord[]>("list_automation_runs", { ruleId });
