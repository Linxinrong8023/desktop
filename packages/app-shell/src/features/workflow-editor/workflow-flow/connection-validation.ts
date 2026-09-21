import type { Connection, Edge, Node } from "@xyflow/react";
import type { WorkflowNodeData } from "@ora/workflow-mock";

export interface WorkflowConnectionValidationInput {
  connection: Connection | Edge;
  nodes: readonly Node<WorkflowNodeData, "workflow">[];
  edges: readonly Edge[];
  reconnectingEdgeId: string | null;
}

/** Enforces workflow DAG and iteration-region connection rules at the canvas seam. */
export function isValidWorkflowConnection({
  connection,
  nodes,
  edges,
  reconnectingEdgeId,
}: WorkflowConnectionValidationInput): boolean {
  if (
    connection.source === null ||
    connection.target === null ||
    connection.source === connection.target
  ) {
    return false;
  }
  const sourceLoop = nodes.find((node) => node.id === connection.source)?.data
    .containerId;
  const targetLoop = nodes.find((node) => node.id === connection.target)?.data
    .containerId;
  if (sourceLoop !== targetLoop) {
    return false;
  }
  const iterationIds = new Set(
    nodes
      .filter((node) => node.data.kind === "iteration")
      .map((node) => node.id),
  );
  const parentIdOf = (nodeId: string): string | null => {
    const parent = nodes.find((node) => node.id === nodeId)?.parentId;
    return parent !== undefined && iterationIds.has(parent) ? parent : null;
  };
  const sourceOwner = parentIdOf(connection.source);
  const targetOwner = parentIdOf(connection.target);
  const isIterationEntry = connection.sourceHandle === "iteration-entry";

  // Members are closed over their owner, while the internal start may only enter that owner.
  if (sourceOwner !== null && targetOwner !== sourceOwner) {
    return false;
  }
  if (
    targetOwner !== null &&
    sourceOwner !== targetOwner &&
    !(
      connection.source === targetOwner &&
      isIterationEntry &&
      iterationIds.has(connection.source)
    )
  ) {
    return false;
  }
  // The decorative start handle is not the container's outer output and cannot leave the region.
  if (isIterationEntry && targetOwner !== connection.source) {
    return false;
  }

  const duplicate = edges.find(
    (edge) =>
      edge.source === connection.source && edge.target === connection.target,
  );
  return duplicate === undefined || duplicate.id === reconnectingEdgeId;
}
