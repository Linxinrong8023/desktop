import { createContext, useContext } from "react";
import type { Edge } from "@xyflow/react";
import type { WorkflowNodeKind, WorkflowNodeType } from "@ora/workflow-mock";
import type { IterationInsertion } from "../workflow-iteration-graph";

export interface WorkflowIterationActions {
  nodeTypes: WorkflowNodeType[];
  readOnly: boolean;
  insertionForEdge: (
    edge: Pick<Edge, "id" | "source" | "sourceHandle" | "target">,
  ) => IterationInsertion | null;
  outputInsertion: (
    nodeId: string,
    sourceHandle?: string | null,
  ) => IterationInsertion | null;
  insert: (kind: WorkflowNodeKind, insertion: IterationInsertion) => void;
  toggleCollapsed: (iterationId: string) => void;
}

export const WorkflowIterationActionsContext =
  createContext<WorkflowIterationActions | null>(null);

/** Reads iteration graph actions from the workflow canvas. */
export function useWorkflowIterationActions(): WorkflowIterationActions {
  const value = useContext(WorkflowIterationActionsContext);
  if (value === null) {
    throw new Error(
      "useWorkflowIterationActions requires WorkflowIterationActionsProvider",
    );
  }
  return value;
}
