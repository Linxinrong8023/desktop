import type { Edge, Node } from "@xyflow/react";
import {
  WORKFLOW_ITERATION_COLLAPSED_HEIGHT,
  WORKFLOW_ITERATION_COLLAPSED_WIDTH,
  WORKFLOW_NODE_INITIAL_HEIGHT,
  WORKFLOW_NODE_WIDTH,
  type WorkflowNodeData,
} from "@ora/workflow-mock";
import {
  expandIterationFrames,
  iterationExpandedSize,
} from "./workflow-iteration-graph";

/** The editable workflow draft shape the drag guard works over. */
export interface ContainmentWorkflow {
  nodes: Node<WorkflowNodeData, "workflow">[];
  edges: Edge[];
}

export interface IterationDragResult<TWorkflow extends ContainmentWorkflow> {
  workflow: TWorkflow;
  rejectedNodeIds: string[];
}

/**
 * Enforces authored iteration membership after a drag.
 *
 * Membership is never inferred from geometry. Existing members keep their `parentId` and may
 * expand their owner; outer nodes whose center was dropped over a region return to their drag
 * start position so the canvas cannot imply membership that the graph does not contain.
 */
export function applyIterationDragRules<TWorkflow extends ContainmentWorkflow>(
  workflow: TWorkflow,
  beforeDrag: ContainmentWorkflow,
  draggedNodeIds: readonly string[],
): IterationDragResult<TWorkflow> {
  const draggedIds = new Set(draggedNodeIds);
  const originalById = new Map(
    beforeDrag.nodes.map((node) => [node.id, node] as const),
  );
  const frames = workflow.nodes
    .filter((node) => node.data.kind === "iteration")
    .map((node) => ({
      node,
      ...iterationExpandedSize(node),
      visibleWidth:
        node.data.collapsed === true
          ? WORKFLOW_ITERATION_COLLAPSED_WIDTH
          : iterationExpandedSize(node).width,
      visibleHeight:
        node.data.collapsed === true
          ? WORKFLOW_ITERATION_COLLAPSED_HEIGHT
          : iterationExpandedSize(node).height,
    }));
  const rejectedNodeIds: string[] = [];
  let changed = false;
  const nodes = workflow.nodes.map((node) => {
    if (!draggedIds.has(node.id) || node.parentId !== undefined) {
      return node;
    }
    const center = {
      x: node.position.x + nodeWidth(node) / 2,
      y: node.position.y + nodeHeight(node) / 2,
    };
    const overlapsRegion = frames.some(
      (frame) =>
        frame.node.id !== node.id &&
        center.x >= frame.node.position.x &&
        center.x <= frame.node.position.x + frame.visibleWidth &&
        center.y >= frame.node.position.y &&
        center.y <= frame.node.position.y + frame.visibleHeight,
    );
    if (!overlapsRegion) {
      return node;
    }
    const original = originalById.get(node.id);
    if (original === undefined) {
      return node;
    }
    rejectedNodeIds.push(node.id);
    changed = true;
    return { ...node, position: { ...original.position } };
  });
  const withRejectedDropsRestored = changed
    ? ({ ...workflow, nodes } as TWorkflow)
    : workflow;
  const affectedIterationIds = workflow.nodes
    .filter(
      (node) =>
        draggedIds.has(node.id) &&
        node.parentId !== undefined &&
        frames.some((frame) => frame.node.id === node.parentId),
    )
    .map((node) => node.parentId!);
  return {
    workflow: expandIterationFrames(
      withRejectedDropsRestored,
      affectedIterationIds,
    ),
    rejectedNodeIds,
  };
}

/** Returns the measured member width used only for overlap rejection. */
function nodeWidth(node: Node<WorkflowNodeData, "workflow">): number {
  const width = node.measured?.width ?? node.width ?? node.initialWidth;
  return width !== undefined && Number.isFinite(width) && width > 0
    ? width
    : WORKFLOW_NODE_WIDTH;
}

/** Returns the measured member height used only for overlap rejection. */
function nodeHeight(node: Node<WorkflowNodeData, "workflow">): number {
  const height = node.measured?.height ?? node.height ?? node.initialHeight;
  return height !== undefined && Number.isFinite(height) && height > 0
    ? height
    : WORKFLOW_NODE_INITIAL_HEIGHT;
}
