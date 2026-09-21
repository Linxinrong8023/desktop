import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import {
  WORKFLOW_ITERATION_ENTRY_HANDLE_Y,
  WORKFLOW_ITERATION_MEMBER_LEFT,
  WORKFLOW_ITERATION_MEMBER_TOP,
  WORKFLOW_ITERATION_NODE_WIDTH,
  WORKFLOW_NODE_ANCHOR_Y,
  WORKFLOW_NODE_WIDTH,
  type WorkflowNodeData,
} from "@ora/workflow-mock";
import {
  containWorkflowCanvasNodes,
  nodePositionAt,
  organizeWorkflowNodes,
  shouldPersistWorkflowNodeChanges,
  snapNodePosition,
} from "./layout";

/** Creates the smallest executable node needed to exercise layout behavior. */
function workflowNode(
  id: string,
  x: number,
  y: number,
): Node<WorkflowNodeData, "workflow"> {
  return {
    id,
    type: "workflow",
    position: { x, y },
    data: { kind: "output", title: id, description: "", instruction: "" },
  };
}

describe("workflow-flow layout", () => {
  it("centers a dropped card around the pointer at handle height", () => {
    expect(nodePositionAt({ x: 400, y: 300 })).toEqual({
      x: 400 - WORKFLOW_NODE_WIDTH / 2,
      y: 239,
    });
  });

  it("aligns new node positions to the canvas grid", () => {
    expect(snapNodePosition({ x: 253, y: 207 })).toEqual({ x: 260, y: 200 });
  });

  it("persists explicit resize completion without treating measurement as an edit", () => {
    expect(
      shouldPersistWorkflowNodeChanges([
        {
          id: "loop",
          type: "dimensions",
          dimensions: { width: 800, height: 420 },
        },
      ]),
    ).toBe(false);
    expect(
      shouldPersistWorkflowNodeChanges([
        {
          id: "loop",
          type: "dimensions",
          dimensions: { width: 800, height: 420 },
          resizing: false,
        },
      ]),
    ).toBe(true);
  });

  it("contains Loop children and allows the parent to expand around them", () => {
    const loop = workflowNode("loop", 200, 0);
    loop.data = { ...loop.data, kind: "loop" };
    const child = workflowNode("child", 500, 140);
    child.data = { ...child.data, containerId: loop.id };

    expect(containWorkflowCanvasNodes([child, loop])).toEqual([
      loop,
      {
        ...child,
        parentId: loop.id,
        extent: "parent",
        expandParent: true,
      },
    ]);
  });

  it("places dependency layers left-to-right and preserves branch order", () => {
    const nodes = [
      workflowNode("start", 900, 300),
      workflowNode("top", 20, 40),
      workflowNode("bottom", 20, 400),
      workflowNode("output", 0, 0),
    ];
    const edges: Edge[] = [
      { id: "e1", source: "start", target: "top" },
      { id: "e2", source: "start", target: "bottom" },
      { id: "e3", source: "top", target: "output" },
      { id: "e4", source: "bottom", target: "output" },
    ];

    const organized = organizeWorkflowNodes(nodes, edges);
    const positions = Object.fromEntries(
      organized.map((node) => [node.id, node.position]),
    );

    expect(positions.start!.x).toBeLessThan(positions.top!.x);
    expect(positions.top!.x).toBe(positions.bottom!.x);
    expect(positions.top!.y).toBeLessThan(positions.bottom!.y);
    expect(positions.bottom!.x).toBeLessThan(positions.output!.x);
  });

  it("preserves Loop child positions while organizing the root graph", () => {
    const loop = workflowNode("loop", 900, 300);
    loop.data = { ...loop.data, kind: "loop" };
    const childStart = workflowNode("child-start", 40, 145);
    childStart.parentId = loop.id;
    childStart.data = {
      ...childStart.data,
      kind: "start",
      containerId: loop.id,
    };
    const childAgent = workflowNode("child-agent", 350, 145);
    childAgent.parentId = loop.id;
    childAgent.data = {
      ...childAgent.data,
      kind: "agent",
      containerId: loop.id,
    };
    const nodes = [workflowNode("start", 500, 0), loop, childStart, childAgent];

    const organized = organizeWorkflowNodes(nodes, [
      { id: "root", source: "start", target: "loop" },
      { id: "child", source: "child-start", target: "child-agent" },
    ]);

    expect(organized.slice(2)).toEqual([childStart, childAgent]);
    expect(organized[0]!.position.x).toBeLessThan(organized[1]!.position.x);
  });

  it("lays out an iteration DAG independently and reserves its fitted outer width", () => {
    const iteration = {
      ...workflowNode("iter", 400, 200),
      data: {
        kind: "iteration" as const,
        title: "iter",
        description: "",
      },
    };
    const first = {
      ...workflowNode("first", 0, 0),
      parentId: "iter",
      data: { kind: "agent" as const, title: "first", description: "" },
    };
    const second = {
      ...workflowNode("second", 0, 0),
      parentId: "iter",
      data: { kind: "agent" as const, title: "second", description: "" },
    };
    const output = workflowNode("output", 0, 0);
    const organized = organizeWorkflowNodes(
      [iteration, first, second, output],
      [
        {
          id: "entry",
          source: "iter",
          sourceHandle: "iteration-entry",
          target: "first",
        },
        { id: "internal", source: "first", target: "second" },
        { id: "exit", source: "iter", target: "output" },
      ],
    );
    const byId = new Map(organized.map((node) => [node.id, node]));

    expect(byId.get("first")?.position).toEqual({
      x: WORKFLOW_ITERATION_MEMBER_LEFT,
      y: WORKFLOW_ITERATION_MEMBER_TOP,
    });
    expect(byId.get("first")!.position.y + WORKFLOW_NODE_ANCHOR_Y).toBe(
      WORKFLOW_ITERATION_ENTRY_HANDLE_Y,
    );
    expect(byId.get("first")?.position.x).toBeLessThan(
      byId.get("second")!.position.x,
    );
    expect(byId.get("iter")?.initialWidth).toBeGreaterThanOrEqual(
      WORKFLOW_ITERATION_NODE_WIDTH,
    );
    expect(byId.get("output")!.position.x).toBeGreaterThanOrEqual(
      byId.get("iter")!.position.x + byId.get("iter")!.initialWidth!,
    );
  });
});
