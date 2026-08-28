export interface WorkspaceSelection {
  projectId: string | null;
  taskId: string | null;
  sessionId: string | null;
  /** Graph workflow run; mutually exclusive with task/session legs. */
  workflowRunId: string | null;
  /** Client-only new-chat row; mutually exclusive with a persisted session. */
  draftId: string | null;
}

export const EMPTY_WORKSPACE_SELECTION: WorkspaceSelection = {
  projectId: null,
  taskId: null,
  sessionId: null,
  workflowRunId: null,
  draftId: null,
};

/** True when every selection leg is unset. */
export function isWorkspaceSelectionEmpty(
  selection: WorkspaceSelection,
): boolean {
  return (
    selection.projectId === null &&
    selection.taskId === null &&
    selection.sessionId === null &&
    selection.workflowRunId === null &&
    selection.draftId === null
  );
}

/** Accepts only non-empty strings; anything else becomes null. */
function optionalId(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

/**
 * Treats disk / merge payloads as untrusted and rebuilds a selection that
 * obeys the same mutual-exclusion rules as the live store actions.
 *
 * Conversation legs are exclusive: draft, session, and workflow run cannot
 * coexist. A missing projectId collapses everything to empty.
 */
export function sanitizeWorkspaceSelection(value: unknown): WorkspaceSelection {
  if (typeof value !== "object" || value === null) {
    return EMPTY_WORKSPACE_SELECTION;
  }
  const raw = value as Record<string, unknown>;
  const projectId = optionalId(raw.projectId);
  if (projectId === null) return EMPTY_WORKSPACE_SELECTION;

  let taskId = optionalId(raw.taskId);
  let sessionId = optionalId(raw.sessionId);
  let workflowRunId = optionalId(raw.workflowRunId);
  let draftId = optionalId(raw.draftId);

  // Prefer draft, then workflow run, then session — matching the exclusivity
  // each select* action enforces when navigating.
  if (draftId !== null) {
    sessionId = null;
    workflowRunId = null;
  } else if (workflowRunId !== null) {
    taskId = null;
    sessionId = null;
    draftId = null;
  } else if (sessionId !== null) {
    workflowRunId = null;
    draftId = null;
  }

  return {
    projectId,
    taskId,
    sessionId,
    workflowRunId,
    draftId,
  };
}
