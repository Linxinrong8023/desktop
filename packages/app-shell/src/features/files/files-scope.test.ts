import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { invalidateScopedFileQueries, resolveFilesScope } from "./files-scope";
import { queryKeys } from "../../state/hooks/query-keys";

describe("resolveFilesScope", () => {
  it("prefers the task worktree when both ids are present", () => {
    expect(resolveFilesScope("project-1", "task-1")).toEqual({
      kind: "task",
      taskId: "task-1",
    });
  });

  it("falls back to the project checkout without a task", () => {
    expect(resolveFilesScope("project-1", undefined)).toEqual({
      kind: "project",
      projectId: "project-1",
    });
  });
});

describe("invalidateScopedFileQueries", () => {
  it("invalidates project file and directory keys for a modified path", async () => {
    const queryClient = new QueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");
    const scope = resolveFilesScope("project-1", undefined);

    await invalidateScopedFileQueries(queryClient, scope, [
      { kind: "modified", path: "src/main.rs" },
    ]);

    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: queryKeys.projectDirectory("project-1", "src"),
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: queryKeys.projectFile("project-1", "src/main.rs"),
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["project-files", "project-1", "search"],
    });
  });

  it("invalidates the whole project files prefix on rescanRequired", async () => {
    const queryClient = new QueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");
    const scope = resolveFilesScope("project-1", undefined);

    await invalidateScopedFileQueries(queryClient, scope, [
      { kind: "rescanRequired" },
    ]);

    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: queryKeys.projectFiles("project-1"),
    });
    expect(invalidateQueries).toHaveBeenCalledTimes(1);
  });
});
