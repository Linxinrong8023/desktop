import type { Edge } from "@xyflow/react";
import type { DemoWorkflow } from "@ora/workflow-mock";
import { describe, expect, it } from "vitest";
import {
  captureWorkflowHistorySnapshot,
  commitWorkflowHistory,
  createWorkflowHistoryState,
  redoWorkflowHistory,
  undoWorkflowHistory,
} from "./workflow-history";

/** Builds the smallest valid workflow used by the history reducer tests. */
function createWorkflow(position = { x: 0, y: 0 }): DemoWorkflow {
  return {
    id: "workflow-1",
    name: "Demo",
    description: "",
    updatedAt: "2026-01-01T00:00:00.000Z",
    viewport: { x: 0, y: 0, zoom: 1 },
    nodes: [
      {
        id: "start",
        type: "workflow",
        position,
        data: { kind: "start", title: "Start", description: "" },
      },
    ],
    edges: [] as Edge[],
  };
}

describe("workflow history", () => {
  it("ignores semantic no-ops", () => {
    const workflow = createWorkflow();
    const snapshot = captureWorkflowHistorySnapshot(workflow);

    expect(
      commitWorkflowHistory(
        createWorkflowHistoryState(),
        snapshot,
        snapshot,
        "layout.organize",
      ),
    ).toEqual(createWorkflowHistoryState());
  });

  it("moves snapshots between past and future", () => {
    const initial = createWorkflow();
    const moved = createWorkflow({ x: 120, y: 80 });
    const initialSnapshot = captureWorkflowHistorySnapshot(initial);
    const movedSnapshot = captureWorkflowHistorySnapshot(moved);
    const committed = commitWorkflowHistory(
      createWorkflowHistoryState(),
      initialSnapshot,
      movedSnapshot,
      "node.move",
    );

    const undone = undoWorkflowHistory(committed, movedSnapshot);
    expect(undone.snapshot).toEqual(initialSnapshot);
    expect(undone.state.past).toHaveLength(0);
    expect(undone.state.future).toHaveLength(1);

    const redone = redoWorkflowHistory(undone.state, initialSnapshot);
    expect(redone.snapshot).toEqual(movedSnapshot);
    expect(redone.state.past).toHaveLength(1);
    expect(redone.state.future).toHaveLength(0);
  });

  it("clears redo history after a new edit", () => {
    const initial = createWorkflow();
    const moved = createWorkflow({ x: 120, y: 80 });
    const movedAgain = createWorkflow({ x: 240, y: 160 });
    const initialSnapshot = captureWorkflowHistorySnapshot(initial);
    const movedSnapshot = captureWorkflowHistorySnapshot(moved);
    const movedAgainSnapshot = captureWorkflowHistorySnapshot(movedAgain);
    const first = commitWorkflowHistory(
      createWorkflowHistoryState(),
      initialSnapshot,
      movedSnapshot,
      "node.move",
    );
    const undone = undoWorkflowHistory(first, movedSnapshot);
    const second = commitWorkflowHistory(
      undone.state,
      initialSnapshot,
      movedAgainSnapshot,
      "node.move",
    );

    expect(second.future).toEqual([]);
    expect(second.past).toHaveLength(1);
  });

  it("limits the session history to the most recent fifty edits", () => {
    let state = createWorkflowHistoryState();
    const previous = createWorkflow();
    let previousSnapshot = captureWorkflowHistorySnapshot(previous);
    for (let index = 1; index <= 51; index += 1) {
      const current = createWorkflow({ x: index, y: index });
      const currentSnapshot = captureWorkflowHistorySnapshot(current);
      state = commitWorkflowHistory(
        state,
        previousSnapshot,
        currentSnapshot,
        "node.move",
        undefined,
        String(index),
      );
      previousSnapshot = currentSnapshot;
    }

    expect(state.past).toHaveLength(50);
    expect(state.past[0]?.id).toBe("2");
  });
});
