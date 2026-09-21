import { describe, expect, it } from "vitest";
import { workflowContainerNodes } from "./container-layout";

describe("workflowContainerNodes", () => {
  it("derives React Flow parentage from container ownership and orders the parent first", () => {
    const nodes = [
      {
        id: "child",
        position: { x: 40, y: 140 },
        data: { containerId: "loop", title: "Child" },
      },
      {
        id: "outside",
        position: { x: 800, y: 0 },
        data: { title: "Outside" },
      },
      {
        id: "loop",
        position: { x: 200, y: 0 },
        data: { title: "Loop" },
      },
    ];

    expect(workflowContainerNodes(nodes)).toEqual([
      nodes[2],
      { ...nodes[0], parentId: "loop" },
      nodes[1],
    ]);
  });

  it("does not invent parentage for a missing container", () => {
    const nodes = [
      {
        id: "orphan",
        position: { x: 0, y: 0 },
        data: { containerId: "missing" },
      },
    ];

    expect(workflowContainerNodes(nodes)).toEqual(nodes);
  });
});
