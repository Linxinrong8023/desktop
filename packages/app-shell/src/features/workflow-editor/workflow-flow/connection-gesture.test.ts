import { describe, expect, it } from "vitest";
import type { Edge } from "@xyflow/react";
import { connectionForCandidate, reconnectDraft } from "./connection-gesture";

describe("workflow connection gestures", () => {
  it("preserves the iteration entry handle on a whole-card drop", () => {
    expect(
      connectionForCandidate(
        {
          kind: "new",
          source: "iter",
          sourceHandle: "iteration-entry",
        },
        "body",
      ),
    ).toEqual({
      source: "iter",
      target: "body",
      sourceHandle: "iteration-entry",
      targetHandle: null,
    });
  });

  it("keeps the fixed entry source while reconnecting its target", () => {
    const edge: Edge = {
      id: "entry",
      source: "iter",
      sourceHandle: "iteration-entry",
      target: "old-body",
    };

    expect(
      connectionForCandidate(reconnectDraft(edge, "source"), "new-body"),
    ).toEqual({
      source: "iter",
      target: "new-body",
      sourceHandle: "iteration-entry",
      targetHandle: null,
    });
  });
});
