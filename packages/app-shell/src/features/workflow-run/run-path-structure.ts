import {
  workflowPathOrder,
  type WorkflowDefinition,
  type WorkflowDefinitionNode,
} from "@ora/workflow-runtime";

export interface RunPathNodeStage {
  type: "node";
  nodeId: string;
}

export interface RunPathRegionPhase {
  id: string;
  kind: "single" | "parallel" | "conditional";
  nodeIds: string[];
}

export interface RunPathRegionStage {
  type: "region";
  nodeId: string;
  memberCount: number;
  phases: RunPathRegionPhase[];
}

export type RunPathStage = RunPathNodeStage | RunPathRegionStage;

/** Projects a frozen graph into outer stages with nested iteration phases. */
export function projectRunPathStructure(
  definition: WorkflowDefinition,
): RunPathStage[] {
  const nodeById = new Map(definition.nodes.map((node) => [node.id, node]));
  const membersByRegion = new Map<string, WorkflowDefinitionNode[]>();
  for (const node of definition.nodes) {
    if (node.parentId === undefined) {
      continue;
    }
    const members = membersByRegion.get(node.parentId) ?? [];
    members.push(node);
    membersByRegion.set(node.parentId, members);
  }

  return workflowPathOrder(definition).flatMap<RunPathStage>((nodeId) => {
    const node = nodeById.get(nodeId);
    if (
      node === undefined ||
      (node.parentId !== undefined &&
        nodeById.get(node.parentId)?.data.kind === "iteration")
    ) {
      return [];
    }
    const members = membersByRegion.get(nodeId);
    if (node.data.kind !== "iteration" || members === undefined) {
      return [{ type: "node" as const, nodeId }];
    }
    return [
      {
        type: "region" as const,
        nodeId,
        memberCount: members.length,
        phases: projectRegionPhases(definition, nodeId, members),
      },
    ];
  });
}

/** Groups one iteration's DAG into deterministic topological frontiers. */
function projectRegionPhases(
  definition: WorkflowDefinition,
  regionId: string,
  members: WorkflowDefinitionNode[],
): RunPathRegionPhase[] {
  const memberById = new Map(members.map((node) => [node.id, node]));
  const indegree = new Map(members.map((node) => [node.id, 0]));
  const adjacency = new Map(members.map((node) => [node.id, [] as string[]]));
  const incoming = new Map(members.map((node) => [node.id, [] as string[]]));
  for (const edge of definition.edges) {
    if (!memberById.has(edge.source) || !memberById.has(edge.target)) {
      continue;
    }
    adjacency.get(edge.source)!.push(edge.target);
    incoming.get(edge.target)!.push(edge.source);
    indegree.set(edge.target, (indegree.get(edge.target) ?? 0) + 1);
  }

  const remaining = new Set(memberById.keys());
  const phases: RunPathRegionPhase[] = [];
  while (remaining.size > 0) {
    const ready = [...remaining]
      .filter((nodeId) => (indegree.get(nodeId) ?? 0) === 0)
      .sort((left, right) => compareRegionPosition(memberById, left, right));
    const phaseNodeIds = ready.length > 0 ? ready : [...remaining].sort();
    phases.push({
      id: `${regionId}:phase:${phases.length}`,
      kind: resolvePhaseKind(phaseNodeIds, incoming, memberById),
      nodeIds: phaseNodeIds,
    });
    for (const nodeId of phaseNodeIds) {
      remaining.delete(nodeId);
      for (const targetId of adjacency.get(nodeId) ?? []) {
        indegree.set(targetId, (indegree.get(targetId) ?? 1) - 1);
      }
    }
  }
  return phases;
}

/** Distinguishes a condition's mutually exclusive fan-out from true parallel readiness. */
function resolvePhaseKind(
  nodeIds: string[],
  incoming: ReadonlyMap<string, string[]>,
  nodeById: ReadonlyMap<string, WorkflowDefinitionNode>,
): RunPathRegionPhase["kind"] {
  if (nodeIds.length < 2) {
    return "single";
  }
  const sources = nodeIds.map((nodeId) => incoming.get(nodeId) ?? []);
  const commonSource = sources[0]?.length === 1 ? sources[0][0] : undefined;
  if (
    commonSource !== undefined &&
    sources.every(
      (nodeSources) =>
        nodeSources.length === 1 && nodeSources[0] === commonSource,
    ) &&
    nodeById.get(commonSource)?.data.kind === "condition"
  ) {
    return "conditional";
  }
  return "parallel";
}

/** Keeps parallel peers aligned with their top-to-bottom editor placement. */
function compareRegionPosition(
  nodeById: ReadonlyMap<string, WorkflowDefinitionNode>,
  left: string,
  right: string,
): number {
  const leftNode = nodeById.get(left);
  const rightNode = nodeById.get(right);
  const yDelta = (leftNode?.position.y ?? 0) - (rightNode?.position.y ?? 0);
  if (yDelta !== 0) {
    return yDelta;
  }
  const xDelta = (leftNode?.position.x ?? 0) - (rightNode?.position.x ?? 0);
  return xDelta === 0 ? left.localeCompare(right) : xDelta;
}
