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
