import { type ReactNode } from "react";
import {
  TaskChangesNavigationContext,
  type FileNavigationLocation,
} from "./task-changes-navigation-context";

interface TaskChangesNavigationProviderProps {
  children: ReactNode;
  onOpenDiff: (path: string, location?: FileNavigationLocation) => void;
  onOpenWorkspaceFile: (
    path: string,
    location?: FileNavigationLocation,
  ) => void;
  onOpenWorkspaceDirectory?: (path: string) => void;
  onOpenWorkspaceArtifact?: (
    path: string,
    line?: number,
    column?: number,
  ) => void;
}

/** Shares Diff and Files navigation actions with nested conversation content. */
export function TaskChangesNavigationProvider({
  children,
  onOpenDiff,
  onOpenWorkspaceFile,
  onOpenWorkspaceDirectory,
  onOpenWorkspaceArtifact,
}: TaskChangesNavigationProviderProps) {
  return (
    <TaskChangesNavigationContext.Provider
      value={{
        openDiff: onOpenDiff,
        openWorkspaceFile: onOpenWorkspaceFile,
        openWorkspaceDirectory: onOpenWorkspaceDirectory,
        openWorkspaceArtifact: onOpenWorkspaceArtifact,
      }}
    >
      {children}
    </TaskChangesNavigationContext.Provider>
  );
}
