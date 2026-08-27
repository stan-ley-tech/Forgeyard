export interface ProjectRecord {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
}

export type Language = "node" | "python" | "go" | "rust";

export type PackageManager =
  | "npm"
  | "yarn"
  | "pnpm"
  | "bun"
  | "pip"
  | "poetry"
  | "pipenv"
  | "cargo"
  | "go";

export interface DockerInfo {
  has_dockerfile: boolean;
  has_compose: boolean;
  compose_services: string[];
}

export interface EnvFileInfo {
  file_name: string;
  key_count: number;
}

export interface ProjectHealth {
  languages: Language[];
  package_managers: PackageManager[];
  docker: DockerInfo;
  env_files: EnvFileInfo[];
  has_git: boolean;
}

export type Ecosystem = "npm" | "pip" | "go" | "cargo";
export type DependencyKind = "runtime" | "dev" | "build";

export interface Dependency {
  name: string;
  version_spec: string;
  kind: DependencyKind;
  ecosystem: Ecosystem;
}

export interface DependencyConflict {
  name: string;
  sources: [string, string][];
}

export interface DependencyReport {
  dependencies: Dependency[];
  conflicts: DependencyConflict[];
}

export type ProcessStatus = "running" | "stopping" | "stopped" | "crashed";

export interface ProcessInfo {
  project_id: string;
  name: string;
  command: string;
  status: ProcessStatus;
  pid: number | null;
  started_at: string;
  exit_code: number | null;
}

export interface ProcessLogEvent {
  project_id: string;
  name: string;
  stream: "stdout" | "stderr";
  line: string;
  timestamp: string;
}

export interface FileChange {
  path: string;
  status: string;
}

export interface GitStatus {
  branch: string | null;
  ahead: number;
  behind: number;
  staged: FileChange[];
  unstaged: FileChange[];
  untracked: string[];
}

export interface BranchInfo {
  name: string;
  is_current: boolean;
  is_remote: boolean;
}

export interface CommitInfo {
  hash: string;
  short_hash: string;
  author: string;
  date: string;
  message: string;
}

export interface TaskEntry {
  name: string;
  command: string;
}

export interface TaskGroup {
  name: string;
  tasks: TaskEntry[];
}

export type LogLevel = "info" | "warn" | "error";

export interface LogEntry {
  id: number;
  project_id: string;
  service: string;
  level: LogLevel;
  message: string;
  created_at: string;
}

export interface LogFilter {
  service?: string | null;
  level?: LogLevel | null;
  search?: string | null;
  limit?: number | null;
}

export interface TaskRef {
  group: string;
  name: string;
}

export interface AutomationRule {
  id: string;
  project_id: string;
  name: string;
  watch_globs: string[];
  tasks: TaskRef[];
  enabled: boolean;
  created_at: string;
}

export type StageStatus = "passed" | "failed" | "skipped";

export interface StageResult {
  group: string;
  name: string;
  status: StageStatus;
  exit_code: number | null;
}

export interface AutomationRunRecord {
  id: string;
  rule_id: string;
  trigger_path: string | null;
  stages: StageResult[];
  status: string;
  started_at: string;
  finished_at: string | null;
}
