import { describe, expect, it } from "vitest";
import type { Node } from "@xyflow/react";
import type { WorkflowNodeData } from "@ora/workflow-mock";
import {
  applyIterationDragRules,
  type ContainmentWorkflow,
} from "./workflow-iteration-containment";

function workflowNode(
  id: string,
  kind: WorkflowNodeData["kind"],
  position: { x: number; y: number },
  parentId?: string,
): Node<WorkflowNodeData, "workflow"> {
  return {
    id,
    type: "workflow",
    position,
    data: { kind, title: id, description: "" },
    ...(parentId === undefined ? {} : { parentId }),
  };
}

function workflowOf(
  nodes: Node<WorkflowNodeData, "workflow">[],
): ContainmentWorkflow {
  return { nodes, edges: [] };
}

describe("iteration drag rules", () => {
  it("restores an outer node dropped over an expanded iteration region", () => {
    const before = workflowOf([
      workflowNode("iter", "iteration", { x: 0, y: 0 }),
      workflowNode("fix", "agent", { x: 700, y: 40 }),
    ]);
    const after = workflowOf([
      before.nodes[0]!,
      workflowNode("fix", "agent", { x: 100, y: 180 }),
    ]);

    const result = applyIterationDragRules(after, before, ["fix"]);

    expect(result.rejectedNodeIds).toEqual(["fix"]);
    expect(result.workflow.nodes[1]?.position).toEqual({ x: 700, y: 40 });
    expect(result.workflow.nodes[1]?.parentId).toBeUndefined();
  });

  it("never changes membership when a node moves inside a frame", () => {
    const before = workflowOf([
      workflowNode("iter", "iteration", { x: 0, y: 0 }),
      workflowNode("fix", "agent", { x: 700, y: 40 }),
    ]);
    const after = workflowOf([
      before.nodes[0]!,
      workflowNode("fix", "agent", { x: 40, y: 20 }),
    ]);

    const result = applyIterationDragRules(after, before, ["fix"]);

    expect(result.workflow.nodes[1]?.parentId).toBeUndefined();
  });

  it("keeps a member in its owner and expands the frame after member movement", () => {
    const before = workflowOf([
      workflowNode("iter", "iteration", { x: 0, y: 0 }),
      workflowNode("fix", "agent", { x: 60, y: 180 }, "iter"),
    ]);
    const after = workflowOf([
      before.nodes[0]!,
      workflowNode("fix", "agent", { x: 620, y: 400 }, "iter"),
    ]);

    const result = applyIterationDragRules(after, before, ["fix"]);
    const frame = result.workflow.nodes[0];

    expect(result.rejectedNodeIds).toEqual([]);
    expect(result.workflow.nodes[1]?.parentId).toBe("iter");
    expect(frame?.initialWidth).toBeGreaterThan(560);
    expect(frame?.initialHeight).toBeGreaterThan(340);
  });

  it("does not treat a collapsed frame as an active region drop target", () => {
    const frame = workflowNode("iter", "iteration", { x: 0, y: 0 });
    frame.data = { ...frame.data, collapsed: true };
    const before = workflowOf([
      frame,
      workflowNode("fix", "agent", { x: 700, y: 40 }),
    ]);
    const after = workflowOf([
      frame,
      workflowNode("fix", "agent", { x: 100, y: 180 }),
    ]);

    const result = applyIterationDragRules(after, before, ["fix"]);

    expect(result.rejectedNodeIds).toEqual([]);
    expect(result.workflow.nodes[1]?.position).toEqual({ x: 100, y: 180 });
  });
});
