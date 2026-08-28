import { createContext, useContext } from "react";

/**
 * Optional locate target for Files/Changes navigation. Omitted fields stay
 * unset so callers never pass positional `undefined` holes.
 */
export interface FileNavigationLocation {
  line?: number;
  /** Inclusive end of a cited range; omitted for a single-line jump. */
  endLine?: number;
  column?: number;
  /** Diff quotes: which patch side the line numbers belong to. */
  side?: "old" | "new";
}

/**
 * Drops undefined fields so a path-only jump stays `openX(path)`-shaped and a
 * line jump is `{ line }` rather than `{ line: 12, column: undefined }`.
 */
export function fileNavigationLocation(
  fields: FileNavigationLocation,
): FileNavigationLocation | undefined {
  const location: FileNavigationLocation = {};
  if (fields.line !== undefined) location.line = fields.line;
  if (fields.endLine !== undefined) location.endLine = fields.endLine;
  if (fields.column !== undefined) location.column = fields.column;
  if (fields.side !== undefined) location.side = fields.side;
  return Object.keys(location).length === 0 ? undefined : location;
}

export interface TaskChangesNavigation {
  openDiff: (path: string, location?: FileNavigationLocation) => void;
  openWorkspaceFile: (path: string, location?: FileNavigationLocation) => void;
  openWorkspaceDirectory?: (path: string) => void;
  openWorkspaceArtifact?: (
    path: string,
    line?: number,
    column?: number,
  ) => void;
}

export const TaskChangesNavigationContext =
  createContext<TaskChangesNavigation | null>(null);

/** Returns the nearest task Changes navigator when the conversation belongs to a task. */
export function useTaskChangesNavigation(): TaskChangesNavigation | null {
  return useContext(TaskChangesNavigationContext);
}
