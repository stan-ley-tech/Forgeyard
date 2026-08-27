import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { DependencyReport, ProjectRecord } from "../lib/types";
import { DependencyPanel } from "./DependencyPanel";

const listDependencies = vi.fn();
vi.mock("../lib/ipc", () => ({
  listDependencies: (...args: unknown[]) => listDependencies(...args),
}));

const project: ProjectRecord = {
  id: "p1",
  name: "demo",
  path: "/tmp/demo",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const report: DependencyReport = {
  dependencies: [
    { name: "react", version_spec: "^18.0.0", kind: "runtime", ecosystem: "npm" },
    { name: "vite", version_spec: "^5.0.0", kind: "dev", ecosystem: "npm" },
  ],
  conflicts: [],
};

beforeEach(() => {
  listDependencies.mockReset();
  listDependencies.mockResolvedValue(report);
});

describe("DependencyPanel", () => {
  it("lists every dependency once loaded", async () => {
    render(<DependencyPanel project={project} />);

    expect(await screen.findByText("react")).toBeInTheDocument();
    expect(screen.getByText("vite")).toBeInTheDocument();
  });

  it("filters the list by name", async () => {
    render(<DependencyPanel project={project} />);
    await screen.findByText("react");

    await userEvent.type(screen.getByPlaceholderText("Filter by name…"), "vi");

    expect(screen.queryByText("react")).not.toBeInTheDocument();
    expect(screen.getByText("vite")).toBeInTheDocument();
  });

  it("surfaces cross-manifest version conflicts", async () => {
    listDependencies.mockResolvedValue({
      dependencies: [],
      conflicts: [
        {
          name: "requests",
          sources: [
            ["requirements.txt", "==2.31.0"],
            ["pyproject.toml", "==2.28.0"],
          ],
        },
      ],
    });

    render(<DependencyPanel project={project} />);

    expect(await screen.findByText(/Potential conflicts/)).toBeInTheDocument();
    expect(screen.getByText(/requests/)).toBeInTheDocument();
  });
});
