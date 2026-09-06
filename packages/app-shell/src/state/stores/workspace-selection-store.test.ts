import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { useDraftSessionsStore } from "./draft-sessions-store";
import { useWorkspaceSelectionStore } from "./workspace-selection-store";
import { EMPTY_WORKSPACE_SELECTION } from "./sanitize-workspace-selection";

const empty = EMPTY_WORKSPACE_SELECTION;

beforeEach(() => {
  window.localStorage.clear();
  useDraftSessionsStore.getState().clear();
  useWorkspaceSelectionStore.setState({
    selection: empty,
    createFocus: null,
  });
});

afterEach(() => {
  window.localStorage.clear();
  useWorkspaceSelectionStore.setState({
    selection: empty,
    createFocus: null,
  });
});

describe("useWorkspaceSelectionStore", () => {
  it("starts empty", () => {
    expect(useWorkspaceSelectionStore.getState().selection).toEqual(empty);
  });

  it("selectProject sets project and clears task/session/run", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().selectProject("p2");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p2",
      taskId: null,
      sessionId: null,
      workflowRunId: null,
      draftId: null,
    });
  });

  it("selectTask records the owning project and clears session/run", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().selectTask("t2", "p1");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: "t2",
      sessionId: null,
      workflowRunId: null,
      draftId: null,
    });
  });

  it("selectSession records project, task, and session together", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: "t1",
      sessionId: "s1",
      workflowRunId: null,
      draftId: null,
    });
  });

  it("selectSessionBeforeTask records a project-scoped session before a task exists", () => {
    useWorkspaceSelectionStore
      .getState()
      .selectSessionBeforeTask("draft-1", "p1");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: null,
      sessionId: "draft-1",
      workflowRunId: null,
      draftId: null,
    });
  });

  it("selectDraft records a client-only chat and clears session/run", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().selectDraft("d1", null, "p1");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: null,
      sessionId: null,
      workflowRunId: null,
      draftId: "d1",
    });
  });

  it("selectWorkflowRun clears task and session under the project", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().selectWorkflowRun("gwr-1", "p1");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: null,
      sessionId: null,
      workflowRunId: "gwr-1",
      draftId: null,
    });
  });

  it("selectSession clears an active workflow run", () => {
    useWorkspaceSelectionStore.getState().selectWorkflowRun("gwr-1", "p1");
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    expect(
      useWorkspaceSelectionStore.getState().selection.workflowRunId,
    ).toBeNull();
  });

  it("clearSelection empties all legs", () => {
    useWorkspaceSelectionStore.getState().selectWorkflowRun("gwr-1", "p1");
    useWorkspaceSelectionStore.getState().clearSelection();
    expect(useWorkspaceSelectionStore.getState().selection).toEqual(empty);
  });

  it("clearSessionSelection keeps project and task", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().clearSessionSelection();
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: "t1",
      sessionId: null,
      workflowRunId: null,
      draftId: null,
    });
  });

  it("clearTaskSelection keeps the project leg only", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().clearTaskSelection("p1");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p1",
      taskId: null,
      sessionId: null,
      workflowRunId: null,
      draftId: null,
    });
  });

  it("setProject(null) empties the whole selection", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().setProject(null);
    expect(useWorkspaceSelectionStore.getState().selection).toEqual(empty);
  });

  it("setProject(id) switches project and clears children", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore.getState().setProject("p2");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p2",
      taskId: null,
      sessionId: null,
      workflowRunId: null,
      draftId: null,
    });
  });

  it("leaving an empty draft discards it; a typed draft stays", () => {
    const emptyId = useDraftSessionsStore
      .getState()
      .ensureEmptyDraft({ projectId: "p1", taskId: null });
    useWorkspaceSelectionStore.getState().selectDraft(emptyId, null, "p1");
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    expect(useDraftSessionsStore.getState().drafts).toHaveLength(0);

    const typedId = useDraftSessionsStore
      .getState()
      .ensureEmptyDraft({ projectId: "p1", taskId: null });
    useDraftSessionsStore.getState().updateContent(typedId, { text: "keep" });
    useWorkspaceSelectionStore.getState().selectDraft(typedId, null, "p1");
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    expect(
      useDraftSessionsStore.getState().drafts.map((draft) => draft.id),
    ).toEqual([typedId]);
  });

  it("updates selection before a failing draft cleanup", () => {
    const draftId = useDraftSessionsStore
      .getState()
      .ensureEmptyDraft({ projectId: "p1", taskId: null });
    useWorkspaceSelectionStore.getState().selectDraft(draftId, null, "p1");
    const cleanup = vi
      .spyOn(useDraftSessionsStore.getState(), "discardIfEmpty")
      .mockImplementation(() => {
        throw new Error("persistence failed");
      });

    expect(() =>
      useWorkspaceSelectionStore.getState().selectProject("p2"),
    ).toThrow("persistence failed");
    expect(useWorkspaceSelectionStore.getState().selection).toEqual({
      projectId: "p2",
      taskId: null,
      sessionId: null,
      workflowRunId: null,
      draftId: null,
    });
    cleanup.mockRestore();
  });

  it("setCreateFocus records a create target without changing selection", () => {
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p2", taskId: null });
    expect(useWorkspaceSelectionStore.getState().selection.sessionId).toBe(
      "s1",
    );
    expect(useWorkspaceSelectionStore.getState().createFocus).toEqual({
      projectId: "p2",
      taskId: null,
    });
  });

  it("selectSession syncs createFocus to the session's project and task", () => {
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p-old", taskId: null });
    useWorkspaceSelectionStore.getState().selectSession("s1", "t1", "p1");
    expect(useWorkspaceSelectionStore.getState().createFocus).toEqual({
      projectId: "p1",
      taskId: "t1",
    });
  });

  it("setCreateFocus is a no-op when the focus is unchanged", () => {
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p1", taskId: null });
    const before = useWorkspaceSelectionStore.getState().createFocus;
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p1", taskId: null });
    expect(useWorkspaceSelectionStore.getState().createFocus).toBe(before);
  });

  it("clearCreateFocusForTask demotes a worktree focus to the project", () => {
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p1", taskId: "t1" });
    useWorkspaceSelectionStore.getState().clearCreateFocusForTask("t1");
    expect(useWorkspaceSelectionStore.getState().createFocus).toEqual({
      projectId: "p1",
      taskId: null,
    });
  });

  it("clearCreateFocusForProject drops focus on the deleted project", () => {
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p1", taskId: null });
    useWorkspaceSelectionStore.getState().clearCreateFocusForProject("p1");
    expect(useWorkspaceSelectionStore.getState().createFocus).toBeNull();
  });

  it("clearCreateFocusForProject leaves focus on a different project untouched", () => {
    useWorkspaceSelectionStore
      .getState()
      .setCreateFocus({ projectId: "p2", taskId: null });
    useWorkspaceSelectionStore.getState().clearCreateFocusForProject("p1");
    expect(useWorkspaceSelectionStore.getState().createFocus).toEqual({
      projectId: "p2",
      taskId: null,
    });
  });
});
