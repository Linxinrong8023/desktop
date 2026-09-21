import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { WorkflowNodeData } from "@ora/workflow-mock";
import {
  applyIterationFrameResize,
  compactIterationFrames,
  insertIterationMember,
  iterationExpandedSize,
  type IterationGraph,
  projectIterationEdges,
  repairIterationGraphAfterNodeDeletion,
  resolveIterationDeletionCascade,
} from "./workflow-iteration-graph";

function node(
  id: string,
  kind: WorkflowNodeData["kind"],
  position = { x: 0, y: 0 },
  parentId?: string,
): Node<WorkflowNodeData, "workflow"> {
  return {
    id,
    type: "workflow",
    position,
    data: {
      kind,
      title: id,
      description: "",
      ...(kind === "agent"
        ? {
            agentConfig: {
              schemaVersion: 3 as const,
              executor: { agentCli: "test", modelId: "test" },
              roleId: "",
              skills: [],
              mcps: [],
              prompt: "",
              interactive: true,
            },
          }
        : {}),
      ...(kind === "iteration"
        ? {
            iterationConfig: {
              iteratorSelector: ["start", "items"],
              collectSelector: ["result", "output"],
              errorStrategy: "fail" as const,
              maxIterations: 10,
            },
          }
        : {}),
    },
    ...(parentId === undefined ? {} : { parentId }),
  };
}

function graph(
  nodes: IterationGraph["nodes"],
  edges: Edge[] = [],
): IterationGraph {
  return { nodes, edges };
}

describe("iteration graph transforms", () => {
  it("hides folded-region edges only in the canvas projection", () => {
    const sourceGraph = graph(
      [
        node("start", "start"),
        node("iter", "iteration"),
        node("first", "agent", { x: 120, y: 100 }, "iter"),
        node("second", "agent", { x: 420, y: 100 }, "iter"),
      ],
      [
        { id: "outer", source: "start", target: "iter" },
        {
          id: "entry",
          source: "iter",
          sourceHandle: "iteration-entry",
          target: "first",
        },
        { id: "internal", source: "first", target: "second" },
      ],
    );

    expect(projectIterationEdges(sourceGraph, new Set(["iter"]))).toEqual([
      { id: "outer", source: "start", target: "iter" },
      {
        id: "entry",
        source: "iter",
        sourceHandle: "iteration-entry",
        target: "first",
        hidden: true,
      },
      {
        id: "internal",
        source: "first",
        target: "second",
        hidden: true,
      },
    ]);
    expect(sourceGraph.edges).toEqual([
      { id: "outer", source: "start", target: "iter" },
      {
        id: "entry",
        source: "iter",
        sourceHandle: "iteration-entry",
        target: "first",
      },
      { id: "internal", source: "first", target: "second" },
    ]);
  });

  it("adds an entry member with the decorative entry source handle", () => {
    const inserted = insertIterationMember(
      graph([node("iter", "iteration")]),
      { type: "entry", iterationId: "iter" },
      node("agent-1", "agent"),
    );

    expect(inserted.edges).toEqual([
      {
        id: "edge-1",
        source: "iter",
        sourceHandle: "iteration-entry",
        target: "agent-1",
        type: "workflow",
      },
    ]);
    expect(
      inserted.nodes.find((candidate) => candidate.id === "agent-1"),
    ).toMatchObject({
      parentId: "iter",
      position: { x: 120, y: 100 },
      data: { agentConfig: { interactive: false } },
    });
  });

  it("places another entry branch below the actual authored entry target", () => {
    const inserted = insertIterationMember(
      graph(
        [
          node("iter", "iteration"),
          node("existing", "agent", { x: 360, y: 280 }, "iter"),
        ],
        [
          {
            id: "entry",
            source: "iter",
            sourceHandle: "iteration-entry",
            target: "existing",
          },
        ],
      ),
      { type: "entry", iterationId: "iter" },
      node("agent-2", "agent"),
    );

    expect(inserted.edges).toEqual([
      {
        id: "entry",
        source: "iter",
        sourceHandle: "iteration-entry",
        target: "existing",
      },
      {
        id: "edge-1",
        source: "iter",
        sourceHandle: "iteration-entry",
        target: "agent-2",
        type: "workflow",
      },
    ]);
    expect(
      inserted.nodes.find((candidate) => candidate.id === "agent-2"),
    ).toMatchObject({
      parentId: "iter",
      position: { x: 120, y: 430 },
    });
  });

  it("skips an occupied first-column row when adding an entry branch", () => {
    const inserted = insertIterationMember(
      graph(
        [
          node("iter", "iteration"),
          node("entry", "agent", { x: 120, y: 280 }, "iter"),
          node("manual", "agent", { x: 180, y: 430 }, "iter"),
        ],
        [
          {
            id: "entry-edge",
            source: "iter",
            sourceHandle: "iteration-entry",
            target: "entry",
          },
        ],
      ),
      { type: "entry", iterationId: "iter" },
      node("agent-2", "agent"),
    );

    expect(
      inserted.nodes.find((candidate) => candidate.id === "agent-2"),
    ).toMatchObject({
      parentId: "iter",
      position: { x: 120, y: 580 },
    });
  });

  it("stacks a second entry member below a measured tall member", () => {
    const tallMember = {
      ...node("agent-1", "agent", { x: 120, y: 100 }, "iter"),
      measured: { width: 230, height: 177 },
    };
    const inserted = insertIterationMember(
      graph([node("iter", "iteration"), tallMember]),
      { type: "entry", iterationId: "iter" },
      node("condition-1", "condition"),
    );

    expect(
      inserted.nodes.find((candidate) => candidate.id === "condition-1"),
    ).toMatchObject({
      parentId: "iter",
      position: { x: 120, y: 100 + 177 + 52 },
    });
  });

  it("clears an entry-edge insertion off the first member's row", () => {
    const first = {
      ...node("agent-1", "agent", { x: 120, y: 100 }, "iter"),
      measured: { width: 230, height: 177 },
    };
    const inserted = insertIterationMember(
      graph(
        [node("iter", "iteration"), first],
        [
          {
            id: "entry",
            source: "iter",
            sourceHandle: "iteration-entry",
            target: "agent-1",
            type: "workflow",
          },
        ],
      ),
      { type: "edge", iterationId: "iter", edgeId: "entry" },
      node("agent-2", "agent"),
    );

    expect(
      inserted.nodes.find((candidate) => candidate.id === "agent-2"),
    ).toMatchObject({
      parentId: "iter",
      position: { x: 120, y: 100 + 177 + 52 },
    });
  });

  it("stacks a second branch append below a measured sibling", () => {
    const source = node("condition", "condition", { x: 80, y: 160 }, "iter");
    const firstAppend = {
      ...node("agent-1", "agent", { x: 426, y: 160 }, "iter"),
      measured: { width: 230, height: 161 },
    };
    const inserted = insertIterationMember(
      graph(
        [node("iter", "iteration"), source, firstAppend],
        [
          {
            id: "branch-a",
            source: "condition",
            sourceHandle: "else",
            target: "agent-1",
            type: "workflow",
          },
        ],
      ),
      {
        type: "output",
        iterationId: "iter",
        sourceId: "condition",
        sourceHandle: "else",
      },
      node("agent-2", "agent"),
    );

    expect(
      inserted.nodes.find((candidate) => candidate.id === "agent-2"),
    ).toMatchObject({
      parentId: "iter",
      position: { x: 80 + 320 + 100, y: 160 + 161 + 52 },
    });
  });

  it("inserts on an edge while preserving the original Condition source handle", () => {
    const original: Edge = {
      id: "branch",
      source: "condition",
      sourceHandle: "case-1",
      target: "target",
      targetHandle: "input",
      type: "workflow",
      label: "IF",
    };
    const inserted = insertIterationMember(
      graph(
        [
          node("iter", "iteration"),
          node("condition", "condition", { x: 80, y: 160 }, "iter"),
          node("target", "agent", { x: 520, y: 160 }, "iter"),
        ],
        [original],
      ),
      { type: "edge", iterationId: "iter", edgeId: "branch" },
      node("agent-2", "agent"),
    );

    expect(inserted.edges).toEqual([
      {
        id: "branch",
        source: "condition",
        sourceHandle: "case-1",
        target: "agent-2",
        type: "workflow",
        label: "IF",
      },
      {
        id: "edge-1",
        source: "agent-2",
        target: "target",
        targetHandle: "input",
        type: "workflow",
      },
    ]);
  });

  it("appends from a specific unconnected Condition branch", () => {
    const inserted = insertIterationMember(
      graph([
        node("iter", "iteration"),
        node("condition", "condition", { x: 80, y: 160 }, "iter"),
      ]),
      {
        type: "output",
        iterationId: "iter",
        sourceId: "condition",
        sourceHandle: "else",
      },
      node("agent-1", "agent"),
    );

    expect(inserted.edges).toEqual([
      {
        id: "edge-1",
        source: "condition",
        sourceHandle: "else",
        target: "agent-1",
        type: "workflow",
      },
    ]);
  });

  it("expands for far members and compacts back to the minimum", () => {
    const expanded = insertIterationMember(
      graph([
        {
          ...node("iter", "iteration"),
          initialWidth: 560,
          initialHeight: 340,
        },
        node("source", "agent", { x: 480, y: 300 }, "iter"),
      ]),
      {
        type: "output",
        iterationId: "iter",
        sourceId: "source",
      },
      node("target", "agent"),
    );
    const frame = expanded.nodes.find((candidate) => candidate.id === "iter");
    expect(frame?.initialWidth).toBeGreaterThan(560);
    expect(frame?.initialHeight).toBeGreaterThan(340);

    const compacted = compactIterationFrames(
      graph([node("iter", "iteration")]),
    );
    expect(compacted.nodes[0]).toMatchObject({
      initialWidth: 560,
      initialHeight: 340,
    });
  });

  it("clears a deleted collect target and leaves unrelated selectors untouched", () => {
    const otherIteration = {
      ...node("other", "iteration"),
      data: {
        ...node("other", "iteration").data,
        iterationConfig: {
          iteratorSelector: ["start", "items"],
          collectSelector: ["keep", "output"],
          errorStrategy: "fail" as const,
          maxIterations: 10,
        },
      },
    };
    const repaired = repairIterationGraphAfterNodeDeletion(
      graph([node("iter", "iteration"), otherIteration]),
      new Set(["result"]),
    );

    expect(repaired.clearedCollectSelectorIterationIds).toEqual(["iter"]);
    expect(
      repaired.graph.nodes.find((candidate) => candidate.id === "iter")?.data
        .iterationConfig?.collectSelector,
    ).toEqual([]);
    expect(
      repaired.graph.nodes.find((candidate) => candidate.id === "other")?.data
        .iterationConfig?.collectSelector,
    ).toEqual(["keep", "output"]);
  });

  it("keeps the persisted expanded size while a frame is collapsed", () => {
    const frame = {
      ...node("iter", "iteration"),
      initialWidth: 840,
      initialHeight: 520,
      data: {
        ...node("iter", "iteration").data,
        collapsed: true,
      },
    };

    expect(iterationExpandedSize(frame)).toEqual({ width: 840, height: 520 });
  });

  it("resolves members and every incident edge for a container cascade", () => {
    const cascade = resolveIterationDeletionCascade(
      graph(
        [
          node("iter", "iteration"),
          node("member-a", "agent", { x: 96, y: 160 }, "iter"),
          node("member-b", "condition", { x: 420, y: 160 }, "iter"),
          node("outer", "output"),
        ],
        [
          { id: "into-iter", source: "outer", target: "iter" },
          { id: "entry", source: "iter", target: "member-a" },
          { id: "internal", source: "member-a", target: "member-b" },
        ],
      ),
      new Set(["iter"]),
    );

    expect(cascade).toEqual({
      nodeIds: new Set(["iter", "member-a", "member-b"]),
      edgeIds: new Set(["into-iter", "entry", "internal"]),
      memberCount: 2,
    });
  });

  it("mirrors a manual resize gesture onto the persisted frame size", () => {
    const frame = {
      ...node("iter", "iteration"),
      initialWidth: 560,
      initialHeight: 340,
    };
    const resizing = applyIterationFrameResize(
      [frame],
      [
        {
          id: "iter",
          type: "dimensions",
          resizing: true,
          setAttributes: true,
          dimensions: { width: 723, height: 419 },
        },
      ],
    );

    expect(resizing).toEqual([
      {
        ...frame,
        initialWidth: 740,
        initialHeight: 420,
      },
    ]);
  });

  it("ignores plain measurement changes and non-frame resize targets", () => {
    const frame = {
      ...node("iter", "iteration"),
      initialWidth: 560,
      initialHeight: 340,
    };
    const member = node("member", "agent", { x: 120, y: 100 }, "iter");
    const measurement = applyIterationFrameResize(
      [frame, member],
      [
        {
          id: "iter",
          type: "dimensions",
          dimensions: { width: 999, height: 999 },
        },
        {
          id: "member",
          type: "dimensions",
          resizing: true,
          setAttributes: true,
          dimensions: { width: 230, height: 140 },
        },
        { id: "member", type: "position", position: { x: 140, y: 100 } },
      ],
    );

    expect(measurement).toEqual([frame, member]);
  });

  it("keeps a manually resized frame's pinned box in sync when compacting", () => {
    const frame = {
      ...node("iter", "iteration"),
      initialWidth: 840,
      initialHeight: 520,
      width: 840,
      height: 520,
    };

    const compacted = compactIterationFrames(graph([frame]));
    expect(compacted.nodes[0]).toMatchObject({
      initialWidth: 560,
      initialHeight: 340,
      width: 560,
      height: 340,
    });
  });
});
