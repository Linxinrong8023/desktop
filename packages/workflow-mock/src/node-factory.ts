import type { Edge, Node, XYPosition } from "@xyflow/react";
import {
  createMockWorkflowCapabilities,
  createMockWorkflowNodeType,
} from "./capabilities";
import type {
  WorkflowAgentConfig,
  WorkflowNodeData,
  WorkflowNodeKind,
} from "./node-data";
import { DEFAULT_ITERATION_MAX_ITERATIONS } from "./iteration-defaults";

export const WORKFLOW_LOOP_NODE_WIDTH = 620;
export const WORKFLOW_LOOP_NODE_HEIGHT = 300;

/** Nodes and internal edge created atomically for one executable Loop container. */
export interface WorkflowLoopGroup {
  nodes: Node<WorkflowNodeData, "workflow">[];
  edges: Edge[];
}

/** Creates a catalog item as a native React Flow node with business data in `data`. */
export function createMockWorkflowNode({
  kind,
  sequence,
  position,
  locale,
  agentConfig,
}: {
  kind: WorkflowNodeKind;
  sequence: number;
  position: XYPosition;
  locale: "zh-CN" | "en-US";
  agentConfig?: WorkflowAgentConfig;
}): Node<WorkflowNodeData, "workflow"> {
  const nodeType = createMockWorkflowNodeType(kind, locale);
  return {
    id: `${kind}-${sequence}`,
    type: "workflow",
    ...(kind === "start" ? { deletable: false } : {}),
    position: { ...position },
    data: {
      kind,
      title: `${nodeType.label} ${sequence}`,
      description: nodeType.description,
      ...createMockNodeExecutionData(kind, locale, agentConfig),
    },
  };
}

/** Provides deterministic values for React Flow's node-data execution extension. */
function createMockNodeExecutionData(
  kind: WorkflowNodeKind,
  locale: "zh-CN" | "en-US",
  agentConfig: WorkflowAgentConfig | undefined,
): Pick<
  WorkflowNodeData,
  | "agentConfig"
  | "input"
  | "instruction"
  | "tool"
  | "condition"
  | "cases"
  | "waitStrategy"
  | "failureStrategy"
  | "maxAttempts"
  | "exitCondition"
  | "iterationConfig"
> {
  const capabilities = createMockWorkflowCapabilities(locale);
  switch (kind) {
    case "start":
      return { input: "" };
    case "output":
      return {};
    case "iteration":
      // Selectors start empty; the inspector requires an array-typed iterator source and a
      // region-internal collect target before the graph becomes executable.
      return {
        iterationConfig: {
          iteratorSelector: [],
          collectSelector: [],
          errorStrategy: "fail",
          maxIterations: DEFAULT_ITERATION_MAX_ITERATIONS,
        },
      };
    case "human":
    case "subflow":
      return {};
    case "agent":
      return {
        agentConfig: structuredClone(
          agentConfig ?? capabilities.defaultAgentConfig,
        ),
      };
    case "condition":
      return {
        condition: locale === "zh-CN" ? "满足条件" : "Condition is met",
        // Keeping the first IF branch explicit makes its output handle stable before rules exist.
        cases: [{ id: "case-1", logic: "and", conditions: [] }],
      };
    case "tool":
      return { tool: capabilities.defaultTool };
    case "junction":
      return { waitStrategy: "all", failureStrategy: "fail" };
    case "loop":
      return {};
  }
}

/**
 * Creates a valid bounded Loop with one child Start and one child Agent.
 *
 * Keeping the complete group in this factory ensures every editor entry point emits the same
 * container ownership, feedback, termination, and output contract expected by the backend.
 */
export function createMockWorkflowLoopGroup({
  sequence,
  position,
  locale,
  agentConfig,
}: {
  sequence: number;
  position: XYPosition;
  locale: "zh-CN" | "en-US";
  agentConfig?: WorkflowAgentConfig;
}): WorkflowLoopGroup {
  const loop = createMockWorkflowNode({
    kind: "loop",
    sequence,
    position,
    locale,
  });
  const childStartId = `${loop.id}-start`;
  const childAgentId = `${loop.id}-agent`;
  const childStart = createMockWorkflowNode({
    kind: "start",
    sequence,
    position: { x: 40, y: 145 },
    locale,
  });
  childStart.id = childStartId;
  childStart.parentId = loop.id;
  childStart.data = {
    ...childStart.data,
    title: locale === "zh-CN" ? "轮次开始" : "Round start",
    containerId: loop.id,
  };
  const childAgent = createMockWorkflowNode({
    kind: "agent",
    sequence,
    position: { x: 350, y: 145 },
    locale,
    agentConfig,
  });
  childAgent.id = childAgentId;
  childAgent.parentId = loop.id;
  childAgent.data = {
    ...childAgent.data,
    title: locale === "zh-CN" ? "循环 Agent" : "Loop Agent",
    containerId: loop.id,
    agentConfig: {
      ...childAgent.data.agentConfig!,
      prompt:
        locale === "zh-CN"
          ? `改进当前值并输出新值：{{#${loop.id}.value#}}`
          : `Improve the current value and output the new value: {{#${loop.id}.value#}}`,
    },
  };
  loop.initialWidth = WORKFLOW_LOOP_NODE_WIDTH;
  loop.initialHeight = WORKFLOW_LOOP_NODE_HEIGHT;
  loop.data = {
    ...loop.data,
    loopConfig: {
      maxIterations: 3,
      variables: [
        {
          name: "value",
          valueType: "string",
          initial: { kind: "constant", value: "" },
          feedback: [childAgentId, "output"],
        },
      ],
      until: {
        logic: "and",
        conditions: [
          {
            variableSelector: [childAgentId, "output"],
            operator: "not_empty",
          },
        ],
      },
      outputs: [
        {
          name: "result",
          variableSelector: [childAgentId, "output"],
        },
      ],
    },
  };
  return {
    nodes: [loop, childStart, childAgent],
    edges: [
      {
        id: `e-${childStartId}-${childAgentId}`,
        source: childStartId,
        target: childAgentId,
        type: "workflow",
      },
    ],
  };
}
