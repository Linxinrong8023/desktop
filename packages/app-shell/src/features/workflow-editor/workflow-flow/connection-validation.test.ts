import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { WorkflowNodeData } from "@ora/workflow-mock";
import { isValidWorkflowConnection } from "./connection-validation";

function node(
  id: string,
  kind: WorkflowNodeData["kind"],
  parentId?: string,
): Node<WorkflowNodeData, "workflow"> {
  return {
    id,
    type: "workflow",
    position: { x: 0, y: 0 },
    data: { kind, title: id, description: "" },
    ...(parentId === undefined ? {} : { parentId }),
  };
}

const nodes = [
  node("iter", "iteration"),
  node("first", "agent", "iter"),
  node("second", "agent", "iter"),
  node("outside", "agent"),
];

function valid(connection: Edge, edges: Edge[] = []): boolean {
  return isValidWorkflowConnection({
    connection,
    nodes,
    edges,
    reconnectingEdgeId: null,
  });
}

describe("workflow connection validation", () => {
  it.each([
    ["child-a", "child-b", true],
    ["child-a", "outside", false],
    ["outside", "child-a", false],
    ["loop", "child-a", false],
    ["loop", "outside", true],
  ] as const)(
    "checks Loop boundary for %s → %s",
    (source, target, expected) => {
      const loopNodes = [
        node("loop", "loop"),
        node("outside", "agent"),
        ...["child-a", "child-b"].map((id) => ({
          ...node(id, "agent", "loop"),
          data: {
            kind: "agent" as const,
            title: id,
            description: "",
            containerId: "loop",
          },
        })),
      ];
      expect(
        isValidWorkflowConnection({
          connection: { id: "edge", source, target },
          nodes: loopNodes,
          edges: [],
          reconnectingEdgeId: null,
        }),
      ).toBe(expected);
    },
  );
  it("allows multiple internal-start branches into the owning region", () => {
    expect(
      valid(
        {
          id: "second-entry",
          source: "iter",
          sourceHandle: "iteration-entry",
          target: "second",
        },
        [
          {
            id: "first-entry",
            source: "iter",
            sourceHandle: "iteration-entry",
            target: "first",
          },
        ],
      ),
    ).toBe(true);
  });

  it("rejects the container output as a region entry", () => {
    expect(valid({ id: "invalid", source: "iter", target: "first" })).toBe(
      false,
    );
  });

  it("rejects an internal-start edge that leaves its region", () => {
    expect(
      valid({
        id: "invalid",
        source: "iter",
        sourceHandle: "iteration-entry",
        target: "outside",
      }),
    ).toBe(false);
  });
});
