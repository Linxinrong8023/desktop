import { create } from "zustand";
import { useDraftSessionsStore } from "./draft-sessions-store";
import {
  EMPTY_WORKSPACE_SELECTION,
  type WorkspaceSelection,
} from "./sanitize-workspace-selection";

export type { WorkspaceSelection };

// Selection no longer survives a restart: every launch opens the default empty
// chat. Wipe the key older versions wrote so a stale candidate never lingers
// unread on disk.
const LEGACY_WORKSPACE_SELECTION_STORAGE_KEY = "ora.workspace-selection.v1";
if (typeof window !== "undefined") {
  // Storage can throw (blocked/disabled); a wipe failure must never block the
  // module loading and take the store down with it on startup.
  try {
    window.localStorage.removeItem(LEGACY_WORKSPACE_SELECTION_STORAGE_KEY);
  } catch {
    // Ignore: the stale key simply stays until a later run wipes it.
  }
}

/**
 * Where global New chat / Ctrl+N should create a draft. Ephemeral: not written
 * to disk. Updated by tree row clicks (without changing the composer selection)
 * and kept in sync when the live selection moves to a leaf.
 */
export interface WorkspaceCreateFocus {
  projectId: string;
  taskId: string | null;
}

interface WorkspaceSelectionState {
  selection: WorkspaceSelection;
  /**
   * Last project/worktree the user pointed at for creating a chat. Independent
   * of `selection` so expand/collapse clicks can retarget New chat without
   * yanking the composer off the current session.
   */
  createFocus: WorkspaceCreateFocus | null;
  /** Records where the next global New chat should land without changing selection. */
  setCreateFocus: (focus: WorkspaceCreateFocus) => void;
  /** Drops create focus when its project is deleted and selection did not move there. */
  clearCreateFocusForProject: (projectId: string) => void;
  /**
   * Drops or demotes create focus when its task is deleted. A worktree focus
   * becomes the project's main Workspace so New chat still has a valid home.
   */
  clearCreateFocusForTask: (taskId: string) => void;
  /** Selects a project and clears any task/session/run underneath. */
  selectProject: (projectId: string) => void;
  /** Selects a task under a project and clears any session/run underneath. */
  selectTask: (taskId: string, projectId: string) => void;
  /**
   * Selects a session whose owning task is still being created.
   *
   * A direct chat gets its session before its task, because the task's title
   * comes from the first message. The session id is already final, so this only
   * leaves the task leg empty until the task exists.
   */
  selectSessionBeforeTask: (sessionId: string, projectId: string) => void;
  /** Selects a specific session, recording its owning task and project. */
  selectSession: (sessionId: string, taskId: string, projectId: string) => void;
  /**
   * Selects a client-only draft chat. Session and run legs clear so the
   * composer stays on the landing surface until the first send attaches.
   */
  selectDraft: (
    draftId: string,
    taskId: string | null,
    projectId: string,
  ) => void;
  /** Selects a graph workflow run under its project and optional task workspace. */
  selectWorkflowRun: (
    workflowRunId: string,
    projectId: string,
    taskId?: string,
  ) => void;
  /** Clears the entire selection. */
  clearSelection: () => void;
  /** Clears only the session leg (used after the selected session is deleted). */
  clearSessionSelection: () => void;
  /** Clears the task and session legs (used after the selected task is deleted). */
  clearTaskSelection: (projectId: string) => void;
  /** Clears the workflow-run leg while keeping the project. */
  clearWorkflowRunSelection: (projectId: string) => void;
  /** Replaces the project leg, clearing task/session/run (used after the selected project is deleted). */
  setProject: (projectId: string | null) => void;
}

/** Maps a live selection onto the New-chat create target when a project is set. */
function createFocusFromSelection(
  selection: WorkspaceSelection,
): WorkspaceCreateFocus | null {
  if (selection.projectId === null) return null;
  return { projectId: selection.projectId, taskId: selection.taskId };
}

/**
 * Owns the workspace tree selection without coupling to query data. Callers pass
 * the owning project/task ids they already have from react-query results, which
 * keeps this store a pure state machine that is trivial to unit-test.
 *
 * The selection is memory-only: a restart always opens the default empty chat
 * and never resurrects the last session. `createFocus` follows the same rule so
 * restart does not resurrect a stale create target.
 */
export const useWorkspaceSelectionStore = create<WorkspaceSelectionState>()((
  set,
  get,
) => {
  /**
   * Replaces the complete selection before settling the draft being left.
   * Navigation must remain responsive even if draft persistence cleanup fails.
   * Create focus follows the new selection so New chat stays coherent after
   * opening a leaf; empty selection clears focus.
   */
  const replaceSelection = (selection: WorkspaceSelection): void => {
    const previousDraftId = get().selection.draftId;
    set({
      selection,
      createFocus: createFocusFromSelection(selection),
    });
    if (previousDraftId !== null && previousDraftId !== selection.draftId) {
      useDraftSessionsStore.getState().discardIfEmpty(previousDraftId);
    }
  };

  return {
    selection: EMPTY_WORKSPACE_SELECTION,
    createFocus: null,
    setCreateFocus: (focus) => {
      const current = get().createFocus;
      if (
        current !== null &&
        current.projectId === focus.projectId &&
        current.taskId === focus.taskId
      ) {
        return;
      }
      set({ createFocus: focus });
    },
    clearCreateFocusForProject: (projectId) => {
      const focus = get().createFocus;
      if (focus?.projectId === projectId) set({ createFocus: null });
    },
    clearCreateFocusForTask: (taskId) => {
      const focus = get().createFocus;
      if (focus === null || focus.taskId !== taskId) return;
      set({
        createFocus: { projectId: focus.projectId, taskId: null },
      });
    },
    selectProject: (projectId) => {
      replaceSelection({
        projectId,
        taskId: null,
        sessionId: null,
        workflowRunId: null,
        draftId: null,
      });
    },
    selectTask: (taskId, projectId) => {
      replaceSelection({
        projectId,
        taskId,
        sessionId: null,
        workflowRunId: null,
        draftId: null,
      });
    },
    selectSessionBeforeTask: (sessionId, projectId) => {
      replaceSelection({
        projectId,
        taskId: null,
        sessionId,
        workflowRunId: null,
        draftId: null,
      });
    },
    selectSession: (sessionId, taskId, projectId) => {
      replaceSelection({
        projectId,
        taskId,
        sessionId,
        workflowRunId: null,
        draftId: null,
      });
    },
    selectDraft: (draftId, taskId, projectId) => {
      replaceSelection({
        projectId,
        taskId,
        sessionId: null,
        workflowRunId: null,
        draftId,
      });
    },
    selectWorkflowRun: (workflowRunId, projectId, taskId) => {
      replaceSelection({
        projectId,
        taskId: taskId ?? null,
        sessionId: null,
        workflowRunId,
        draftId: null,
      });
    },
    clearSelection: () => replaceSelection(EMPTY_WORKSPACE_SELECTION),
    clearSessionSelection: () => {
      const current = get().selection;
      replaceSelection({
        projectId: current.projectId,
        taskId: current.taskId,
        sessionId: null,
        workflowRunId: current.workflowRunId,
        draftId: null,
      });
    },
    clearTaskSelection: (projectId) => {
      replaceSelection({
        projectId,
        taskId: null,
        sessionId: null,
        workflowRunId: null,
        draftId: null,
      });
    },
    clearWorkflowRunSelection: (projectId) => {
      replaceSelection({
        projectId,
        taskId: null,
        sessionId: null,
        workflowRunId: null,
        draftId: null,
      });
    },
    setProject: (projectId) => {
      replaceSelection(
        projectId === null
          ? EMPTY_WORKSPACE_SELECTION
          : {
              projectId,
              taskId: null,
              sessionId: null,
              workflowRunId: null,
              draftId: null,
            },
      );
    },
  };
});
