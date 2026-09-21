import type { GraphWorkflowRun } from "@ora/workflow-runtime";

/** Theater follows backend membership; Overview retains the unmodified authoring snapshot. */
export function executableRun(
  run: GraphWorkflowRun,
  unusedNodeIds: readonly string[],
): GraphWorkflowRun {
  if (unusedNodeIds.length === 0) return run;
  const unused = new Set(unusedNodeIds);
  return {
    ...run,
    definitionSnapshot: {
      ...run.definitionSnapshot,
      nodes: run.definitionSnapshot.nodes.filter(
        (node) => !unused.has(node.id),
      ),
      edges: run.definitionSnapshot.edges.filter(
        (edge) => !unused.has(edge.source) && !unused.has(edge.target),
      ),
    },
    nodeStates: Object.fromEntries(
      Object.entries(run.nodeStates).filter(([id]) => !unused.has(id)),
    ),
  };
}
