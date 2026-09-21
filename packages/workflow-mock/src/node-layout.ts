import { Position, type NodeHandle } from "@xyflow/react";

export const WORKFLOW_NODE_WIDTH = 230;
/** Iteration container frame and its compact editor-only chrome. */
export const WORKFLOW_ITERATION_NODE_WIDTH = 560;
export const WORKFLOW_ITERATION_NODE_HEIGHT = 340;
export const WORKFLOW_ITERATION_HEADER_HEIGHT = 52;
export const WORKFLOW_ITERATION_COLLAPSED_WIDTH = 320;
export const WORKFLOW_ITERATION_COLLAPSED_HEIGHT = 56;
/** Shared origin for insertion, automatic layout, and the internal-start affordance. */
export const WORKFLOW_ITERATION_MEMBER_LEFT = 120;
export const WORKFLOW_ITERATION_MEMBER_TOP = 100;
export const WORKFLOW_NODE_INITIAL_HEIGHT = 98;
export const WORKFLOW_NODE_HANDLE_SIZE = 10;
export const WORKFLOW_NODE_ANCHOR_Y = 61;
/** Aligns the internal start with the input handle of the first member row. */
export const WORKFLOW_ITERATION_ENTRY_HANDLE_Y =
  WORKFLOW_ITERATION_MEMBER_TOP + WORKFLOW_NODE_ANCHOR_Y;

/** Provides React Flow with initial handle bounds until the browser measures the custom node. */
export const WORKFLOW_NODE_INITIAL_HANDLES = [
  {
    type: "target",
    position: Position.Left,
    x: -WORKFLOW_NODE_HANDLE_SIZE / 2,
    y: WORKFLOW_NODE_ANCHOR_Y - WORKFLOW_NODE_HANDLE_SIZE / 2,
    width: WORKFLOW_NODE_HANDLE_SIZE,
    height: WORKFLOW_NODE_HANDLE_SIZE,
  },
  {
    type: "source",
    position: Position.Right,
    x: WORKFLOW_NODE_WIDTH - WORKFLOW_NODE_HANDLE_SIZE / 2,
    y: WORKFLOW_NODE_ANCHOR_Y - WORKFLOW_NODE_HANDLE_SIZE / 2,
    width: WORKFLOW_NODE_HANDLE_SIZE,
    height: WORKFLOW_NODE_HANDLE_SIZE,
  },
] satisfies NodeHandle[];
