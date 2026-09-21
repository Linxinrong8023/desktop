import type { Connection, Edge, HandleType } from "@xyflow/react";

/** The graph facts that must survive whole-card connection fallback. */
export type ConnectionDraft =
  | {
      kind: "new";
      source: string;
      sourceHandle: string | null;
    }
  | {
      kind: "reconnect";
      edgeId: string;
      endpoint: HandleType;
      source: string;
      target: string;
      sourceHandle: string | null;
      targetHandle: string | null;
    };

/** Resolves a whole-card drop without losing handles on the fixed side of the gesture. */
export function connectionForCandidate(
  draft: ConnectionDraft,
  candidateNodeId: string,
): Connection {
  if (draft.kind === "new") {
    return {
      source: draft.source,
      target: candidateNodeId,
      sourceHandle: draft.sourceHandle,
      targetHandle: null,
    };
  }
  return draft.endpoint === "source"
    ? {
        source: candidateNodeId,
        target: draft.target,
        sourceHandle: null,
        targetHandle: draft.targetHandle,
      }
    : {
        source: draft.source,
        target: candidateNodeId,
        sourceHandle: draft.sourceHandle,
        targetHandle: null,
      };
}

/** Captures the persisted handles needed when reconnecting an edge by its whole card. */
export function reconnectDraft(
  edge: Edge,
  reportedFixedHandle: HandleType,
): ConnectionDraft {
  return {
    kind: "reconnect",
    edgeId: edge.id,
    // React Flow reports the fixed opposite handle, so the moved endpoint is inverted.
    endpoint: reportedFixedHandle === "target" ? "source" : "target",
    source: edge.source,
    target: edge.target,
    sourceHandle: edge.sourceHandle ?? null,
    targetHandle: edge.targetHandle ?? null,
  };
}
