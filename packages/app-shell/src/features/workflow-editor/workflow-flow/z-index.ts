/**
 * Layering contract for elements rendered inside the React Flow viewport.
 *
 * React Flow positions the edge `<svg>` layer, the edge-label layer, and the node
 * layer as siblings, and it elevates an edge above the node layer whenever that edge
 * touches a parented member (see `getElevatedEdgeZIndex` in @xyflow/system). The
 * values below therefore encode two rules:
 *
 * - Annotations stay beneath executable nodes, and selecting a node lifts it above
 *   every unselected element including its own container frame.
 * - Edge-owned controls (the iteration insert buttons rendered through
 *   `EdgeLabelRenderer`) must stay above every node and edge elevation React Flow can
 *   compute, otherwise the edge hit target swallows their clicks. A parented member of
 *   a selected container stacks at `WORKFLOW_SELECTED_NODE_Z_INDEX + 2`, so the edge
 *   control band keeps a full order of magnitude of headroom above it.
 */
export const WORKFLOW_ANNOTATION_Z_INDEX = 0;
export const WORKFLOW_NODE_Z_INDEX = 1;
export const WORKFLOW_SELECTED_NODE_Z_INDEX = 1_000;
export const WORKFLOW_EDGE_CONTROL_Z_INDEX = 2_000;
