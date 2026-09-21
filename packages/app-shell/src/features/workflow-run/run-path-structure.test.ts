import { describe, expect, it } from "vitest";
import type { WorkflowDefinition } from "@ora/workflow-runtime";
import { projectRunPathStructure } from "./run-path-structure";

const definition: WorkflowDefinition = {
  id: "snapshot",
  name: "Grouped iteration",
  description: "",
  updatedAt: "2026-09-17T12:00:00+08:00",
  viewport: { x: 0, y: 0, zoom: 1 },
  nodes: [
    {
      id: "out",
      type: "workflow",
      position: { x: 1_300, y: 200 },
      data: { kind: "output", title: "Output", description: "" },
    },
    {
      id: "merge",
      type: "workflow",
      parentId: "iter",
      position: { x: 420, y: 170 },
      data: { kind: "agent", title: "Merge", description: "" },
    },
    {
      id: "body-b",
      type: "workflow",
      parentId: "iter",
      position: { x: 120, y: 260 },
      data: { kind: "agent", title: "Agent B", description: "" },
    },
    {
      id: "iter",
      type: "workflow",
      position: { x: 300, y: 200 },
      data: { kind: "iteration", title: "Iteration", description: "" },
    },
    {
      id: "body-a",
      type: "workflow",
      parentId: "iter",
      position: { x: 120, y: 80 },
      data: { kind: "agent", title: "Agent A", description: "" },
    },
    {
      id: "start",
      type: "workflow",
      position: { x: 0, y: 200 },
      data: { kind: "start", title: "Start", description: "" },
    },
  ],
  edges: [
    { id: "start-iter", source: "start", target: "iter" },
    {
      id: "entry-a",
      source: "iter",
      sourceHandle: "iteration-entry",
      target: "body-a",
    },
    {
      id: "entry-b",
      source: "iter",
      sourceHandle: "iteration-entry",
      target: "body-b",
    },
    { id: "a-merge", source: "body-a", target: "merge" },
    { id: "b-merge", source: "body-b", target: "merge" },
    { id: "iter-out", source: "iter", target: "out" },
  ],
};

describe("projectRunPathStructure", () => {
  it("keeps Loop children reachable in the path instead of treating them as foreach members", () => {
    const loopDefinition: WorkflowDefinition = {
      ...definition,
      nodes: definition.nodes.map((node) =>
        node.id === "iter"
          ? { ...node, data: { ...node.data, kind: "loop" } }
          : node.parentId === "iter"
            ? { ...node, data: { ...node.data, containerId: "iter" } }
            : node,
      ),
    };
    const stages = projectRunPathStructure(loopDefinition);
    expect(
      stages.filter((stage) =>
        ["body-a", "body-b", "merge"].includes(stage.nodeId),
      ),
    ).toEqual([
      { type: "node", nodeId: "body-a" },
      { type: "node", nodeId: "body-b" },
      { type: "node", nodeId: "merge" },
    ]);
  });
  it("keeps an iteration on the outer path and groups its entry targets in parallel", () => {
    expect(projectRunPathStructure(definition)).toEqual([
      { type: "node", nodeId: "start" },
      {
        type: "region",
        nodeId: "iter",
        memberCount: 3,
        phases: [
          {
            id: "iter:phase:0",
            kind: "parallel",
            nodeIds: ["body-a", "body-b"],
          },
          {
            id: "iter:phase:1",
            kind: "single",
            nodeIds: ["merge"],
          },
        ],
      },
      { type: "node", nodeId: "out" },
    ]);
  });

  it("labels mutually exclusive condition successors as a branch instead of parallel", () => {
    const conditional: WorkflowDefinition = {
      ...definition,
      nodes: [
        ...definition.nodes.filter(
          (node) => !["body-a", "body-b", "merge"].includes(node.id),
        ),
        {
          id: "condition",
          type: "workflow",
          parentId: "iter",
          position: { x: 120, y: 160 },
          data: { kind: "condition", title: "Route", description: "" },
        },
        {
          id: "left",
          type: "workflow",
          parentId: "iter",
          position: { x: 420, y: 80 },
          data: { kind: "agent", title: "Left", description: "" },
        },
        {
          id: "right",
          type: "workflow",
          parentId: "iter",
          position: { x: 420, y: 240 },
          data: { kind: "agent", title: "Right", description: "" },
        },
      ],
      edges: [
        { id: "start-iter", source: "start", target: "iter" },
        {
          id: "entry-condition",
          source: "iter",
          sourceHandle: "iteration-entry",
          target: "condition",
        },
        {
          id: "condition-left",
          source: "condition",
          sourceHandle: "case-left",
          target: "left",
        },
        {
          id: "condition-right",
          source: "condition",
          sourceHandle: "else",
          target: "right",
        },
        { id: "iter-out", source: "iter", target: "out" },
      ],
    };

    expect(projectRunPathStructure(conditional)[1]).toEqual({
      type: "region",
      nodeId: "iter",
      memberCount: 3,
      phases: [
        {
          id: "iter:phase:0",
          kind: "single",
          nodeIds: ["condition"],
        },
        {
          id: "iter:phase:1",
          kind: "conditional",
          nodeIds: ["left", "right"],
        },
      ],
    });
  });
});
