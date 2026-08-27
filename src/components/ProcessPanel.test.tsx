import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ProcessInfo, ProjectRecord } from "../lib/types";
import { ProcessPanel } from "./ProcessPanel";

const listProcesses = vi.fn();
const stopProcess = vi.fn();
const killProcess = vi.fn();
const restartProcess = vi.fn();
const startProcess = vi.fn();
const onProcessLog = vi.fn();
const onProcessStatus = vi.fn();

vi.mock("../lib/ipc", () => ({
  listProcesses: (...args: unknown[]) => listProcesses(...args),
  stopProcess: (...args: unknown[]) => stopProcess(...args),
  killProcess: (...args: unknown[]) => killProcess(...args),
  restartProcess: (...args: unknown[]) => restartProcess(...args),
  startProcess: (...args: unknown[]) => startProcess(...args),
  onProcessLog: (...args: unknown[]) => onProcessLog(...args),
  onProcessStatus: (...args: unknown[]) => onProcessStatus(...args),
}));

const project: ProjectRecord = {
  id: "p1",
  name: "demo",
  path: "/tmp/demo",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const apiProcess: ProcessInfo = {
  project_id: "p1",
  name: "API",
  command: "npm run dev",
  status: "running",
  pid: 4242,
  started_at: "2026-01-01T00:00:00Z",
  exit_code: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  listProcesses.mockResolvedValue([apiProcess]);
  stopProcess.mockResolvedValue(undefined);
  killProcess.mockResolvedValue(undefined);
  restartProcess.mockResolvedValue(apiProcess);
  onProcessLog.mockResolvedValue(() => {});
  onProcessStatus.mockResolvedValue(() => {});
});

describe("ProcessPanel", () => {
  it("lists running processes with their pid", async () => {
    render(<ProcessPanel project={project} />);

    expect(await screen.findByText("API")).toBeInTheDocument();
    expect(screen.getByText("4242")).toBeInTheDocument();
    expect(screen.getByText(/running/)).toBeInTheDocument();
  });

  it("stops a process by name when Stop is clicked", async () => {
    render(<ProcessPanel project={project} />);
    await screen.findByText("API");

    await userEvent.click(screen.getByText("Stop"));

    expect(stopProcess).toHaveBeenCalledWith("p1", "API");
  });

  it("kills a process by name when Kill is clicked", async () => {
    render(<ProcessPanel project={project} />);
    await screen.findByText("API");

    await userEvent.click(screen.getByText("Kill"));

    expect(killProcess).toHaveBeenCalledWith("p1", "API");
  });
});
