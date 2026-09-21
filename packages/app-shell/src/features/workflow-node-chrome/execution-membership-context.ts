import { createContext, useContext } from "react";

export const UnusedNodes = createContext<readonly string[]>([]);

/** Shared presentation state for ordinary cards and container frames. */
export function useWorkflowNodeUnused(nodeId: string): boolean {
  return useContext(UnusedNodes).includes(nodeId);
}
